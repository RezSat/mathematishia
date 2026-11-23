//! Exact limits of partial univariate rational functions at finite rational points.
//!
//! A removed point stays removed from the source function even when its
//! punctured-neighborhood limit exists. The certificate proves local
//! factorization and signs using exact rational arithmetic.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{DomainExpression, Integer, Rational, Symbol};

use crate::budget::WorkBudget;
use crate::{
    EvaluationContext, EvaluationError, RationalExpressionError, RationalFunction,
    RationalPolynomial,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LimitDirection {
    Left,
    Right,
    TwoSided,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LimitValue {
    Finite(Rational),
    PositiveInfinity,
    NegativeInfinity,
    DoesNotExist,
}

/// Exact identity p(x) = (x - a)^order * cofactor(x), with cofactor(a) != 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitFactor {
    order: usize,
    cofactor: RationalPolynomial,
}

impl LimitFactor {
    pub const fn order(&self) -> usize {
        self.order
    }

    pub fn cofactor(&self) -> &RationalPolynomial {
        &self.cofactor
    }
}

/// A checked local factorization, direction, result and source-domain record.
///
/// A missing numerator factor denotes the identically zero numerator.
/// Source exclusions are retained in the embedded partial rational function;
/// their exact evaluations at the target are recorded, never discarded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalLimit {
    source: RationalFunction,
    at: Rational,
    direction: LimitDirection,
    value: LimitValue,
    numerator_factor: Option<LimitFactor>,
    denominator_factor: LimitFactor,
    exclusion_values: Vec<Rational>,
}

impl RationalLimit {
    pub fn source(&self) -> &RationalFunction {
        &self.source
    }

    pub fn at(&self) -> &Rational {
        &self.at
    }

    pub const fn direction(&self) -> LimitDirection {
        self.direction
    }

    pub fn value(&self) -> &LimitValue {
        &self.value
    }

    pub fn numerator_factor(&self) -> Option<&LimitFactor> {
        self.numerator_factor.as_ref()
    }

    pub fn denominator_factor(&self) -> &LimitFactor {
        &self.denominator_factor
    }

    pub fn exclusion_values(&self) -> &[Rational] {
        &self.exclusion_values
    }

    /// Whether the original partial function excludes the target itself.
    /// This does not prevent a limit through admissible nearby points.
    pub fn excluded_at_point(&self) -> bool {
        self.exclusion_values
            .iter()
            .any(|value| value.numerator().is_zero())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitError {
    Evaluation(EvaluationError),
    Conversion(RationalExpressionError),
    Verification(LimitVerificationError),
    InconsistentFactorization,
}

impl fmt::Display for LimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for LimitError {}

impl From<EvaluationError> for LimitError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl From<RationalExpressionError> for LimitError {
    fn from(error: RationalExpressionError) -> Self {
        Self::Conversion(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitVerificationError {
    InvalidCertificate,
    Evaluation(EvaluationError),
}

impl fmt::Display for LimitVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for LimitVerificationError {}

impl From<EvaluationError> for LimitVerificationError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

fn zero() -> Rational {
    Rational::from(Integer::zero())
}

fn one() -> Rational {
    Rational::from(Integer::one())
}

fn linear_factor(at: &Rational) -> RationalPolynomial {
    RationalPolynomial::new(vec![-at, one()])
}

fn factor_at(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
    at: &Rational,
) -> Result<LimitFactor, LimitError> {
    let linear = linear_factor(at);
    let mut cofactor = polynomial.clone();
    let mut order = 0;
    loop {
        let evaluation = work.evaluate(&cofactor, at)?;
        if !evaluation.numerator().is_zero() {
            break;
        }
        let (quotient, remainder) = work.divide(&cofactor, &linear)?;
        if !remainder.is_zero() {
            return Err(LimitError::InconsistentFactorization);
        }
        cofactor = quotient;
        order += 1;
    }
    Ok(LimitFactor { order, cofactor })
}

fn verify_factor(
    work: &mut WorkBudget,
    original: &RationalPolynomial,
    at: &Rational,
    claimed: &LimitFactor,
) -> Result<Rational, LimitVerificationError> {
    if claimed.order > original.degree().unwrap_or(0) {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    let evaluation = work.evaluate(&claimed.cofactor, at)?;
    if evaluation.numerator().is_zero() {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    let linear = linear_factor(at);
    let mut reconstructed = claimed.cofactor.clone();
    for _ in 0..claimed.order {
        reconstructed = work.multiply(&reconstructed, &linear)?;
    }
    if reconstructed != *original {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    Ok(evaluation)
}

fn classify(
    work: &mut WorkBudget,
    numerator: Option<(&LimitFactor, &Rational)>,
    denominator: (&LimitFactor, &Rational),
    direction: LimitDirection,
) -> Result<LimitValue, EvaluationError> {
    let Some((numerator, leading_n)) = numerator else {
        return Ok(LimitValue::Finite(zero()));
    };
    let (denominator, leading_d) = denominator;
    match numerator.order.cmp(&denominator.order) {
        Ordering::Greater => Ok(LimitValue::Finite(zero())),
        Ordering::Equal => Ok(LimitValue::Finite(work.quotient(leading_n, leading_d)?)),
        Ordering::Less => {
            let positive_n = leading_n.numerator() > &Integer::zero();
            let positive_d = leading_d.numerator() > &Integer::zero();
            let positive_right = positive_n == positive_d;
            let odd_pole = (denominator.order - numerator.order) % 2 == 1;
            if direction == LimitDirection::TwoSided && odd_pole {
                return Ok(LimitValue::DoesNotExist);
            }
            let positive = if direction == LimitDirection::Left && odd_pole {
                !positive_right
            } else {
                positive_right
            };
            Ok(if positive {
                LimitValue::PositiveInfinity
            } else {
                LimitValue::NegativeInfinity
            })
        }
    }
}

impl EvaluationContext {
    /// Computes a finite-point one-sided or two-sided limit of a partial Q(x)
    /// function. No target evaluation or numerical sampling is performed.
    ///
    /// A nonzero rational denominator is required; transcendental and symbolic
    /// targets are not part of this API. Work and bit limits are enforced
    /// during factor division, evaluation and independent verification.
    pub fn rational_limit(
        &self,
        source: &RationalFunction,
        at: &Rational,
        direction: LimitDirection,
    ) -> Result<RationalLimit, LimitError> {
        self.check_rational_function(source)?;
        self.check_rational(at)?;
        let mut work = WorkBudget::new(self.limits());
        work.charge(1)?;

        let numerator_factor = if source.numerator().is_zero() {
            None
        } else {
            Some(factor_at(&mut work, source.numerator(), at)?)
        };
        let denominator_factor = factor_at(&mut work, source.denominator(), at)?;
        let numerator_value = numerator_factor
            .as_ref()
            .map(|factor| work.evaluate(&factor.cofactor, at))
            .transpose()?;
        let denominator_value = work.evaluate(&denominator_factor.cofactor, at)?;
        let exclusion_values = source
            .excluded_zero_sets()
            .iter()
            .map(|condition| work.evaluate(condition, at))
            .collect::<Result<Vec<_>, _>>()?;

        let value = classify(
            &mut work,
            numerator_factor
                .as_ref()
                .zip(numerator_value.as_ref()),
            (&denominator_factor, &denominator_value),
            direction,
        )?;
        let result = RationalLimit {
            source: source.clone(),
            at: at.clone(),
            direction,
            value,
            numerator_factor,
            denominator_factor,
            exclusion_values,
        };
        verify_rational_limit(self, source, at, direction, &result)
            .map_err(LimitError::Verification)?;
        Ok(result)
    }

    /// Converts a domain-aware source without erasing historical exclusions.
    /// Unsupported functions, foreign symbols and powers return typed errors.
    pub fn rational_expression_limit(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        at: &Rational,
        direction: LimitDirection,
    ) -> Result<RationalLimit, LimitError> {
        let rational = self.rational_expression(source, variable)?;
        self.rational_limit(&rational, at, direction)
    }
}

/// Independently verifies local polynomial identities, nonvanishing cofactors,
/// all recorded source restrictions and the exact directional classification.
/// It never calls the limit generator and fails closed on exhausted budgets.
pub fn verify_rational_limit(
    ctx: &EvaluationContext,
    source: &RationalFunction,
    at: &Rational,
    direction: LimitDirection,
    claimed: &RationalLimit,
) -> Result<(), LimitVerificationError> {
    ctx.check_rational_function(source)?;
    ctx.check_rational(at)?;
    if claimed.source != *source || claimed.at != *at || claimed.direction != direction {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    let mut work = WorkBudget::new(ctx.limits());
    work.charge(1)?;
    let numerator_value = match (&claimed.numerator_factor, source.numerator().is_zero()) {
        (None, true) => None,
        (Some(factor), false) => Some(verify_factor(
            &mut work,
            source.numerator(),
            at,
            factor,
        )?),
        _ => return Err(LimitVerificationError::InvalidCertificate),
    };
    let denominator_value = verify_factor(
        &mut work,
        source.denominator(),
        at,
        &claimed.denominator_factor,
    )?;
    if claimed.exclusion_values.len() != source.excluded_zero_sets().len() {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    for (condition, recorded) in source
        .excluded_zero_sets()
        .iter()
        .zip(&claimed.exclusion_values)
    {
        if work.evaluate(condition, at)? != *recorded {
            return Err(LimitVerificationError::InvalidCertificate);
        }
    }
    let expected = classify(
        &mut work,
        claimed
            .numerator_factor
            .as_ref()
            .zip(numerator_value.as_ref()),
        (&claimed.denominator_factor, &denominator_value),
        direction,
    )?;
    if expected != claimed.value {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    Ok(())
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;
    use crate::ResourceLimits;

    fn r(n: i64) -> Rational {
        Rational::from(Integer::from(n))
    }

    fn p(coefs: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(coefs.iter().copied().map(r).collect())
    }

    #[test]
    fn forged_factors_domain_claims_and_answers_are_rejected() {
        let ctx = EvaluationContext::default();
        let source = ctx.rational_function(&p(&[-1, 0, 1]), &p(&[-1, 1])).unwrap();
        let at = r(1);
        let good = ctx
            .rational_limit(&source, &at, LimitDirection::TwoSided)
            .unwrap();
        assert_eq!(
            verify_rational_limit(&ctx, &source, &at, LimitDirection::TwoSided, &good),
            Ok(())
        );

        let mut false_value = good.clone();
        false_value.value = LimitValue::Finite(r(3));
        let mut false_order = good.clone();
        false_order.numerator_factor.as_mut().unwrap().order = 1;
        let mut false_cofactor = good.clone();
        false_cofactor.denominator_factor.cofactor = p(&[2]);
        let mut false_domain = good.clone();
        false_domain.exclusion_values[0] = r(1);
        let mut false_point = good.clone();
        false_point.at = r(2);
        for bad in [
            false_value,
            false_order,
            false_cofactor,
            false_domain,
            false_point,
        ] {
            assert_eq!(
                verify_rational_limit(&ctx, &source, &at, LimitDirection::TwoSided, &bad),
                Err(LimitVerificationError::InvalidCertificate)
            );
        }
    }

    #[test]
    fn zero_numerator_cannot_be_forged_into_nonzero_evidence() {
        let ctx = EvaluationContext::default();
        let source = ctx.rational_function(&p(&[]), &p(&[0, 1])).unwrap();
        let mut result = ctx
            .rational_limit(&source, &r(0), LimitDirection::Left)
            .unwrap();
        result.numerator_factor = Some(LimitFactor {
            order: 0,
            cofactor: p(&[1]),
        });
        assert_eq!(
            verify_rational_limit(&ctx, &source, &r(0), LimitDirection::Left, &result),
            Err(LimitVerificationError::InvalidCertificate)
        );
    }

    #[test]
    fn verifier_does_not_ignore_exhausted_budget() {
        let ctx = EvaluationContext::default();
        let source = ctx.rational_function(&p(&[1]), &p(&[0, 1])).unwrap();
        let result = ctx
            .rational_limit(&source, &r(0), LimitDirection::Right)
            .unwrap();
        let restricted = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        assert_eq!(
            verify_rational_limit(&restricted, &source, &r(0), LimitDirection::Right, &result),
            Err(LimitVerificationError::Evaluation(EvaluationError::Limit(
                crate::ResourceLimitKind::WorkUnits
            )))
        );
    }
}
