//! Certified rational factors and explicitly unresolved polynomial cofactors.
//!
//! Complete irreducibility decisions are available through degree three.
//! Rational-root enumeration is bounded; exhaustion is never a proof.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::quartic_factor::{QuarticSearch, search_quartic_quadratic};
use crate::{
    EvaluationContext, EvaluationError, RationalPolynomial,
    modular_irreducibility_witness,
};

const MAX_PRIMITIVE_COEFFICIENT: i64 = 100_000;
const MAX_CANDIDATES: usize = 4_096;

fn one() -> Rational {
    Rational::from(Integer::one())
}

fn positive_divisors(value: u64) -> Vec<u64> {
    let mut divisors = Vec::new();
    let mut candidate = 1;
    while candidate <= value / candidate {
        if value.is_multiple_of(candidate) {
            divisors.push(candidate);
            if candidate != value / candidate {
                divisors.push(value / candidate);
            }
        }
        candidate += 1;
    }
    divisors.sort_unstable();
    divisors
}

/// Exact, exhaustive rational-root candidates for admitted small primitive
/// coefficients. Returns None if this bounded implementation cannot
/// exhaust the rational root theorem.
fn root_candidates(poly: &RationalPolynomial) -> Option<Vec<Rational>> {
    if poly.degree().is_none_or(|degree| degree == 0) {
        return Some(Vec::new());
    }
    let lcm = poly
        .coefficients()
        .iter()
        .fold(Integer::one(), |l, coefficient| {
            l.lcm(coefficient.denominator())
        });
    let integral = poly
        .coefficients()
        .iter()
        .map(|coefficient| {
            let scale = lcm.divide_exact(coefficient.denominator())?;
            Some(&scale * coefficient.numerator())
        })
        .collect::<Option<Vec<_>>>()?;
    let content = integral
        .iter()
        .fold(Integer::zero(), |g, coefficient| g.gcd(coefficient));
    let primitive = integral
        .iter()
        .map(|value| value.divide_exact(&content)?.try_i64())
        .collect::<Option<Vec<_>>>()?;
    if primitive
        .iter()
        .any(|&value| value.unsigned_abs() > MAX_PRIMITIVE_COEFFICIENT as u64)
    {
        return None;
    }
    let constant = primitive.first()?.unsigned_abs();
    let leading = primitive.last()?.unsigned_abs();
    if constant == 0 {
        return Some(vec![Rational::from(Integer::zero())]);
    }
    let mut candidates = BTreeSet::new();
    for numerator in positive_divisors(constant) {
        for denominator in positive_divisors(leading) {
            for sign in [-1_i64, 1] {
                let value = Rational::new(
                    Integer::from(sign * numerator as i64),
                    Integer::from(denominator as i64),
                )
                .expect("positive candidate denominator");
                candidates.insert(value);
                if candidates.len() > MAX_CANDIDATES {
                    return None;
                }
            }
        }
    }
    Some(candidates.into_iter().collect())
}

fn rational_root(poly: &RationalPolynomial) -> Option<Option<Rational>> {
    let candidates = root_candidates(poly)?;
    Some(
        candidates
            .into_iter()
            .find(|candidate| poly.evaluate(candidate).numerator().is_zero()),
    )
}

fn irreducible_small(poly: &RationalPolynomial) -> bool {
    match poly.degree() {
        Some(1) => true,
        Some(2) => {
            let b = &poly.coefficients()[1];
            let c = &poly.coefficients()[0];
            let delta = &(b * b) - &(&Rational::from(Integer::from(4)) * c);
            delta.exact_sqrt().is_none()
        }
        Some(3) => rational_root(poly) == Some(None) || modular_irreducibility_witness(poly).is_some(),
        Some(4) => matches!(
            search_quartic_quadratic(poly, &EvaluationContext::default()),
            Ok(QuarticSearch::Exhausted)
        ) || modular_irreducibility_witness(poly).is_some(),
        Some(degree) if degree > 4 => modular_irreducibility_witness(poly).is_some(),
        _ => false,
    }
}

fn linear(root: &Rational) -> RationalPolynomial {
    RationalPolynomial::new(vec![-root, one()])
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalFactor {
    polynomial: RationalPolynomial,
    multiplicity: usize,
}

impl RationalFactor {
    pub fn polynomial(&self) -> &RationalPolynomial {
        &self.polynomial
    }
    pub fn multiplicity(&self) -> usize {
        self.multiplicity
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalFactorization {
    unit: Rational,
    factors: Vec<RationalFactor>,
    unresolved: Vec<RationalFactor>,
}

impl RationalFactorization {
    pub fn unit(&self) -> &Rational {
        &self.unit
    }
    pub fn factors(&self) -> &[RationalFactor] {
        &self.factors
    }
    pub fn unresolved(&self) -> &[RationalFactor] {
        &self.unresolved
    }
    pub fn is_complete(&self) -> bool {
        self.unresolved.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactorizationVerificationError {
    ZeroPolynomial,
    IncorrectUnit,
    InvalidFactor,
    DuplicateFactor,
    UncertifiedIrreducible,
    ProductMismatch,
}

impl fmt::Display for FactorizationVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for FactorizationVerificationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalFactorizationError {
    Evaluation(EvaluationError),
    ZeroPolynomial,
    InvalidCertificate(FactorizationVerificationError),
}
impl fmt::Display for RationalFactorizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for RationalFactorizationError {}
impl From<EvaluationError> for RationalFactorizationError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

fn reconstruct_product(result: &RationalFactorization) -> RationalPolynomial {
    let mut product = RationalPolynomial::new(vec![result.unit.clone()]);
    for factor in result.factors.iter().chain(result.unresolved.iter()) {
        for _ in 0..factor.multiplicity {
            product = product.multiply(&factor.polynomial);
        }
    }
    product
}

/// Checks the full exact product, multiplicities and claimed irreducibility.
/// A higher-degree or unenumerably large factor cannot be marked certified.
pub fn verify_rational_factorization(
    original: &RationalPolynomial,
    result: &RationalFactorization,
) -> Result<(), FactorizationVerificationError> {
    use FactorizationVerificationError as E;
    let Some(lead) = original.coefficients().last() else {
        return Err(E::ZeroPolynomial);
    };
    if &result.unit != lead {
        return Err(E::IncorrectUnit);
    }
    let mut all = Vec::new();
    for item in result.factors.iter().chain(result.unresolved.iter()) {
        if item.multiplicity == 0
            || item.polynomial.degree().is_none_or(|degree| degree == 0)
            || item.polynomial.coefficients().last() != Some(&one())
        {
            return Err(E::InvalidFactor);
        }
        if all.contains(&&item.polynomial) {
            return Err(E::DuplicateFactor);
        }
        all.push(&item.polynomial);
    }
    for factor in &result.factors {
        if !irreducible_small(&factor.polynomial) {
            return Err(E::UncertifiedIrreducible);
        }
    }
    if reconstruct_product(result) != *original {
        return Err(E::ProductMismatch);
    }
    Ok(())
}

fn record(target: &mut Vec<RationalFactor>, polynomial: RationalPolynomial, multiplicity: usize) {
    if let Some(existing) = target.iter_mut().find(|f| f.polynomial == polynomial) {
        existing.multiplicity += multiplicity;
    } else {
        target.push(RationalFactor {
            polynomial,
            multiplicity,
        });
    }
}

fn factor_squarefree(
    mut polynomial: RationalPolynomial,
    multiplicity: usize,
    factors: &mut Vec<RationalFactor>,
    unresolved: &mut Vec<RationalFactor>,
    ctx: &EvaluationContext,
) -> Result<(), RationalFactorizationError> {
    while let Some(degree) = polynomial.degree() {
        if degree == 0 {
            break;
        }
        if degree == 1 {
            record(factors, polynomial, multiplicity);
            break;
        }
        if degree == 2 {
            let b = &polynomial.coefficients()[1];
            let c = &polynomial.coefficients()[0];
            let delta = &(b * b) - &(&Rational::from(Integer::from(4)) * c);
            if let Some(square_root) = delta.exact_sqrt() {
                let root = &(&(-b) + &square_root)
                    * &Rational::new(Integer::one(), Integer::from(2)).expect("2 is nonzero");
                let divisor = linear(&root);
                let (quotient, remainder) = ctx.divide(&polynomial, &divisor)?;
                debug_assert!(remainder.is_zero());
                record(factors, divisor, multiplicity);
                polynomial = quotient;
                continue;
            }
            record(factors, polynomial, multiplicity);
            break;
        }
        match rational_root(&polynomial) {
            Some(Some(root)) => {
                let divisor = linear(&root);
                let (quotient, remainder) = ctx.divide(&polynomial, &divisor)?;
                debug_assert!(remainder.is_zero());
                record(factors, divisor, multiplicity);
                polynomial = quotient;
            }
            Some(None) if degree == 3 => {
                record(factors, polynomial, multiplicity);
                break;
            }
            _ => {
                if degree == 4 {
                    match search_quartic_quadratic(&polynomial, ctx)? {
                        QuarticSearch::Split(first, second) => {
                            factor_squarefree(first, multiplicity, factors, unresolved, ctx)?;
                            factor_squarefree(second, multiplicity, factors, unresolved, ctx)?;
                            break;
                        }
                        QuarticSearch::Exhausted => {
                            record(factors, polynomial, multiplicity);
                            break;
                        }
                        QuarticSearch::Inconclusive => {}
                    }
                }
                if modular_irreducibility_witness(&polynomial).is_some() {
                    record(factors, polynomial, multiplicity);
                } else {
                    record(unresolved, polynomial, multiplicity);
                }
                break;
            }
        }
    }
    Ok(())
}

impl EvaluationContext {
    /// Extracts exact rational linear factors and certifies irreducible
    /// quadratics/cubics. Higher-degree cofactors remain explicitly unresolved.
    pub fn factor_rational_polynomial(
        &self,
        polynomial: &RationalPolynomial,
    ) -> Result<RationalFactorization, RationalFactorizationError> {
        self.check_polynomial(polynomial)?;
        if polynomial.is_zero() {
            return Err(RationalFactorizationError::ZeroPolynomial);
        }
        let decomposition = self.square_free_decomposition(polynomial)?;
        let mut result = RationalFactorization {
            unit: decomposition.unit().clone(),
            factors: Vec::new(),
            unresolved: Vec::new(),
        };
        for class in decomposition.factors() {
            factor_squarefree(
                class.polynomial().clone(),
                class.multiplicity(),
                &mut result.factors,
                &mut result.unresolved,
                self,
            )?;
        }
        let order = |factor: &RationalFactor| {
            (
                factor.polynomial.degree().unwrap_or(0),
                factor
                    .polynomial
                    .coefficients()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            )
        };
        result.factors.sort_by_key(&order);
        result.unresolved.sort_by_key(order);
        verify_rational_factorization(polynomial, &result)
            .map_err(RationalFactorizationError::InvalidCertificate)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(coeffs: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            coeffs
                .iter()
                .map(|&n| Rational::from(Integer::from(n)))
                .collect(),
        )
    }

    #[test]
    fn reject_forged_irreducible_factor() {
        let original = p(&[-1, 0, 1]);
        let forged = RationalFactorization {
            unit: one(),
            factors: vec![RationalFactor {
                polynomial: original.clone(),
                multiplicity: 1,
            }],
            unresolved: Vec::new(),
        };
        assert_eq!(
            verify_rational_factorization(&original, &forged),
            Err(FactorizationVerificationError::UncertifiedIrreducible)
        );
    }

    #[test]
    fn reject_wrong_multiplicity_and_missing_product() {
        let original = p(&[1, 2, 1]);
        let mut evidence = EvaluationContext::default()
            .factor_rational_polynomial(&original)
            .unwrap();
        evidence.factors[0].multiplicity += 1;
        assert_eq!(
            verify_rational_factorization(&original, &evidence),
            Err(FactorizationVerificationError::ProductMismatch)
        );
    }
}
