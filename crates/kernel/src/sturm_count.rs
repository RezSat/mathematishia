//! Bounded, exact Sturm root counts for square-free polynomials over Q.
//!
//! Endpoints are finite rationals and must not be roots of the input.
//! The count is for distinct real roots in the open interval only.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::budget::WorkBudget;
use crate::{EvaluationContext, EvaluationError, RationalPolynomial};

/// The exact sign of a rational evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SturmSign {
    Negative,
    Zero,
    Positive,
}

impl SturmSign {
    fn of(value: &Rational) -> Self {
        match value.numerator().cmp(&Integer::zero()) {
            Ordering::Less => Self::Negative,
            Ordering::Equal => Self::Zero,
            Ordering::Greater => Self::Positive,
        }
    }
}

/// Exact division of consecutive Sturm polynomials: dividend = quotient * divisor + remainder.
/// The next Sturm polynomial is the NEGATIVE of a nonzero remainder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SturmDivisionEvidence {
    quotient: RationalPolynomial,
    remainder: RationalPolynomial,
}

impl SturmDivisionEvidence {
    pub fn quotient(&self) -> &RationalPolynomial {
        &self.quotient
    }

    pub fn remainder(&self) -> &RationalPolynomial {
        &self.remainder
    }
}

/// Complete, reproducible certificate for an exact root count on an open interval.
///
/// For a nonconstant input, the sequence begins with (p, p') and continues
/// with negative Euclidean remainders, ending at a nonzero constant.
/// Zero signs of intermediate members are omitted when counting variations.
/// A nonzero constant input has a one-member sequence and zero divisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SturmCountCertificate {
    polynomial: RationalPolynomial,
    lower: Rational,
    upper: Rational,
    sequence: Vec<RationalPolynomial>,
    divisions: Vec<SturmDivisionEvidence>,
    lower_signs: Vec<SturmSign>,
    upper_signs: Vec<SturmSign>,
    lower_variations: usize,
    upper_variations: usize,
    root_count: usize,
}

impl SturmCountCertificate {
    pub fn polynomial(&self) -> &RationalPolynomial {
        &self.polynomial
    }

    pub fn lower(&self) -> &Rational {
        &self.lower
    }

    pub fn upper(&self) -> &Rational {
        &self.upper
    }

    pub fn sequence(&self) -> &[RationalPolynomial] {
        &self.sequence
    }

    pub fn divisions(&self) -> &[SturmDivisionEvidence] {
        &self.divisions
    }

    pub fn lower_signs(&self) -> &[SturmSign] {
        &self.lower_signs
    }

    pub fn upper_signs(&self) -> &[SturmSign] {
        &self.upper_signs
    }

    pub const fn lower_variations(&self) -> usize {
        self.lower_variations
    }

    pub const fn upper_variations(&self) -> usize {
        self.upper_variations
    }

    pub const fn root_count(&self) -> usize {
        self.root_count
    }
}

/// Invalid inputs and bounded exact-arithmetic failures are distinct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SturmCountError {
    ZeroPolynomial,
    NonSquareFree,
    InvalidInterval,
    RootAtEndpoint,
    Evaluation(EvaluationError),
    Verification(SturmVerificationError),
}

impl fmt::Display for SturmCountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for SturmCountError {}

impl From<EvaluationError> for SturmCountError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

/// A verifier never trusts a supplied sequence, sign vector, or count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SturmVerificationError {
    InvalidInput,
    InvalidEvidence,
    Evaluation(EvaluationError),
}

impl fmt::Display for SturmVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for SturmVerificationError {}

impl From<EvaluationError> for SturmVerificationError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

fn check_input(
    ctx: &EvaluationContext,
    polynomial: &RationalPolynomial,
    lower: &Rational,
    upper: &Rational,
) -> Result<(), SturmCountError> {
    ctx.check_polynomial(polynomial)?;
    ctx.check_rational(lower)?;
    ctx.check_rational(upper)?;
    if polynomial.is_zero() {
        return Err(SturmCountError::ZeroPolynomial);
    }
    if lower >= upper {
        return Err(SturmCountError::InvalidInterval);
    }
    Ok(())
}

fn variations(signs: &[SturmSign]) -> usize {
    let mut previous = None;
    let mut changes = 0;
    for &current in signs {
        if current == SturmSign::Zero {
            continue;
        }
        if previous.is_some_and(|sign| sign != current) {
            changes += 1;
        }
        previous = Some(current);
    }
    changes
}

fn endpoint_signs(
    work: &mut WorkBudget,
    sequence: &[RationalPolynomial],
    at: &Rational,
) -> Result<Vec<SturmSign>, EvaluationError> {
    let mut signs = Vec::with_capacity(sequence.len());
    for polynomial in sequence {
        work.charge(1)?;
        signs.push(SturmSign::of(&work.evaluate(polynomial, at)?));
    }
    Ok(signs)
}

fn is_square_free(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
    derivative: &RationalPolynomial,
) -> Result<bool, EvaluationError> {
    let gcd = work.gcd(polynomial, derivative)?;
    Ok(gcd == RationalPolynomial::new(vec![Rational::from(Integer::one())]))
}

impl EvaluationContext {
    /// Counts distinct real zeros strictly between two rational non-root endpoints.
    ///
    /// Only nonzero square-free polynomials are admitted. A repeated factor
    /// receives NonSquareFree even if its repeated roots lie outside the interval.
    /// The canonical unscaled Sturm chain uses exact negative remainders;
    /// multiplying a chain member by an arbitrary negative unit would change
    /// the signs and is not permitted. A zero internal sign contributes no
    /// variation. No sampling, rounding or claim of all-roots isolation is used.
    ///
    /// Work, polynomial-degree, scalar-bit and intermediate-bit limits from the
    /// evaluation context apply. Resource refusal is an error, never a count.
    pub fn count_real_roots_sturm(
        &self,
        polynomial: &RationalPolynomial,
        lower: &Rational,
        upper: &Rational,
    ) -> Result<SturmCountCertificate, SturmCountError> {
        check_input(self, polynomial, lower, upper)?;
        let mut work = WorkBudget::new(self.limits());

        if SturmSign::of(&work.evaluate(polynomial, lower)?) == SturmSign::Zero
            || SturmSign::of(&work.evaluate(polynomial, upper)?) == SturmSign::Zero
        {
            return Err(SturmCountError::RootAtEndpoint);
        }

        let mut sequence = vec![polynomial.clone()];
        let mut divisions = Vec::new();
        if polynomial.degree().is_some_and(|degree| degree > 0) {
            let derivative = work.differentiate(polynomial)?;
            if !is_square_free(&mut work, polynomial, &derivative)? {
                return Err(SturmCountError::NonSquareFree);
            }
            sequence.push(derivative);
            loop {
                work.charge(1)?;
                let n = sequence.len();
                let (quotient, remainder) = work.divide(&sequence[n - 2], &sequence[n - 1])?;
                let done = remainder.is_zero();
                let next = if done {
                    None
                } else {
                    Some(work.subtract(&RationalPolynomial::zero(), &remainder)?)
                };
                divisions.push(SturmDivisionEvidence {
                    quotient,
                    remainder,
                });
                if let Some(next) = next {
                    sequence.push(next);
                } else {
                    break;
                }
            }
        }

        let lower_signs = endpoint_signs(&mut work, &sequence, lower)?;
        let upper_signs = endpoint_signs(&mut work, &sequence, upper)?;
        let lower_variations = variations(&lower_signs);
        let upper_variations = variations(&upper_signs);
        let root_count = lower_variations
            .checked_sub(upper_variations)
            .ok_or(SturmVerificationError::InvalidEvidence)
            .map_err(SturmCountError::Verification)?;
        let certificate = SturmCountCertificate {
            polynomial: polynomial.clone(),
            lower: lower.clone(),
            upper: upper.clone(),
            sequence,
            divisions,
            lower_signs,
            upper_signs,
            lower_variations,
            upper_variations,
            root_count,
        };
        verify_sturm_count(self, polynomial, lower, upper, &certificate).map_err(|error| {
            match error {
                SturmVerificationError::Evaluation(error) => SturmCountError::Evaluation(error),
                other => SturmCountError::Verification(other),
            }
        })?;
        Ok(certificate)
    }
}

/// Independently verifies a Sturm certificate against the supplied original
/// polynomial and interval. It rechecks square-freeness, every exact quotient
/// and remainder, Euclidean recomposition, the canonical negative-remainder
/// chain, endpoint values and zero-skipping variation counts. It does not call
/// the certificate generator. All verification arithmetic is context-bounded.
pub fn verify_sturm_count(
    ctx: &EvaluationContext,
    polynomial: &RationalPolynomial,
    lower: &Rational,
    upper: &Rational,
    claimed: &SturmCountCertificate,
) -> Result<(), SturmVerificationError> {
    check_input(ctx, polynomial, lower, upper).map_err(|error| match error {
        SturmCountError::Evaluation(error) => SturmVerificationError::Evaluation(error),
        _ => SturmVerificationError::InvalidInput,
    })?;
    if claimed.polynomial != *polynomial || claimed.lower != *lower || claimed.upper != *upper {
        return Err(SturmVerificationError::InvalidEvidence);
    }
    let degree = polynomial.degree().expect("nonzero polynomial");
    let expected_len = if degree == 0 { 1 } else { 2 };
    if claimed.sequence.len() < expected_len
        || claimed.sequence.len() > degree.saturating_add(1)
        || claimed.divisions.len() != claimed.sequence.len().saturating_sub(1)
        || claimed.lower_signs.len() != claimed.sequence.len()
        || claimed.upper_signs.len() != claimed.sequence.len()
        || claimed.sequence[0] != *polynomial
    {
        return Err(SturmVerificationError::InvalidEvidence);
    }

    let mut work = WorkBudget::new(ctx.limits());
    if SturmSign::of(&work.evaluate(polynomial, lower)?) == SturmSign::Zero
        || SturmSign::of(&work.evaluate(polynomial, upper)?) == SturmSign::Zero
    {
        return Err(SturmVerificationError::InvalidInput);
    }

    if degree > 0 {
        let derivative = work.differentiate(polynomial)?;
        if !is_square_free(&mut work, polynomial, &derivative)? {
            return Err(SturmVerificationError::InvalidInput);
        }
        if claimed.sequence[1] != derivative {
            return Err(SturmVerificationError::InvalidEvidence);
        }
        for (i, evidence) in claimed.divisions.iter().enumerate() {
            work.charge(1)?;
            let dividend = &claimed.sequence[i];
            let divisor = &claimed.sequence[i + 1];
            if divisor.is_zero() {
                return Err(SturmVerificationError::InvalidEvidence);
            }
            let (expected_quotient, expected_remainder) = work.divide(dividend, divisor)?;
            if evidence.quotient != expected_quotient
                || evidence.remainder != expected_remainder
                || evidence
                    .remainder
                    .degree()
                    .is_some_and(|rem_degree| rem_degree >= divisor.degree().unwrap_or(0))
            {
                return Err(SturmVerificationError::InvalidEvidence);
            }
            let product = work.multiply(divisor, &evidence.quotient)?;
            let recomposed = work.add(&product, &evidence.remainder)?;
            if recomposed != *dividend {
                return Err(SturmVerificationError::InvalidEvidence);
            }
            if evidence.remainder.is_zero() {
                if i + 1 != claimed.divisions.len() || i + 2 != claimed.sequence.len() {
                    return Err(SturmVerificationError::InvalidEvidence);
                }
            } else {
                if i + 2 >= claimed.sequence.len() {
                    return Err(SturmVerificationError::InvalidEvidence);
                }
                let negative = work.subtract(&RationalPolynomial::zero(), &evidence.remainder)?;
                if claimed.sequence[i + 2] != negative {
                    return Err(SturmVerificationError::InvalidEvidence);
                }
            }
        }
        if claimed.sequence.last().and_then(RationalPolynomial::degree) != Some(0) {
            return Err(SturmVerificationError::InvalidEvidence);
        }
    }

    let lower_signs = endpoint_signs(&mut work, &claimed.sequence, lower)?;
    let upper_signs = endpoint_signs(&mut work, &claimed.sequence, upper)?;
    if claimed.lower_signs != lower_signs || claimed.upper_signs != upper_signs {
        return Err(SturmVerificationError::InvalidEvidence);
    }
    let lower_variations = variations(&lower_signs);
    let upper_variations = variations(&upper_signs);
    let root_count = lower_variations
        .checked_sub(upper_variations)
        .ok_or(SturmVerificationError::InvalidEvidence)?;
    if claimed.lower_variations != lower_variations
        || claimed.upper_variations != upper_variations
        || claimed.root_count != root_count
    {
        return Err(SturmVerificationError::InvalidEvidence);
    }
    Ok(())
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    fn r(v: i64) -> Rational {
        Rational::from(Integer::from(v))
    }

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(values.iter().copied().map(r).collect())
    }

    #[test]
    fn verifier_rejects_corruption_of_every_claimed_evidence_kind() {
        let ctx = EvaluationContext::default();
        let source = p(&[-2, 0, 1]);
        let a = r(-2);
        let b = r(2);
        let original = ctx.count_real_roots_sturm(&source, &a, &b).unwrap();
        let verify = |candidate: &SturmCountCertificate| {
            verify_sturm_count(&ctx, &source, &a, &b, candidate)
        };
        assert_eq!(verify(&original), Ok(()));

        let mut wrong = original.clone();
        wrong.root_count += 1;
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.lower_variations += 1;
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.upper_variations += 1;
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.lower_signs[1] = SturmSign::Positive;
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.upper_signs[0] = SturmSign::Negative;
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.sequence[1] = p(&[1]);
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.divisions[0].remainder = p(&[1]);
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.divisions[0].quotient = p(&[99]);
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.divisions.pop();
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.polynomial = p(&[-3, 0, 1]);
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.lower = r(-3);
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
        let mut wrong = original.clone();
        wrong.upper = r(3);
        assert_eq!(verify(&wrong), Err(SturmVerificationError::InvalidEvidence));
    }

    #[test]
    fn omitting_internal_zero_signs_changes_variations_and_is_rejected() {
        let ctx = EvaluationContext::default();
        let source = p(&[-2, 0, 1]);
        let mut wrong = ctx.count_real_roots_sturm(&source, &r(0), &r(2)).unwrap();
        assert_eq!(wrong.lower_signs[1], SturmSign::Zero);
        wrong.lower_signs.remove(1);
        assert_eq!(
            verify_sturm_count(&ctx, &source, &r(0), &r(2), &wrong),
            Err(SturmVerificationError::InvalidEvidence)
        );
    }
}
