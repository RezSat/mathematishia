//! Exact multivariate division over Q with ordered divisors and lexicographic monomials.
//!
//! Quotients depend on divisor priority. A remainder has no term divisible
//! by any divisor's leading term, but it is not a canonical normal form
//! unless the divisors constitute an appropriate Groebner basis.

use std::error::Error;
use std::fmt;

use mathema::Rational;

use crate::{MultivariateError, MultivariateLimits, MultivariatePolynomial};

const MAX_DIVISORS: usize = 32;
const MAX_DIVISION_STEPS: usize = 8_192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultivariateDivisionError {
    NoDivisors,
    TooManyDivisors,
    ZeroDivisor,
    ResourceLimit,
    NondecreasingLeadingTerm,
    InvalidCertificate,
    Polynomial(MultivariateError),
}

impl fmt::Display for MultivariateDivisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDivisors => f.write_str("at least one polynomial divisor is required"),
            Self::TooManyDivisors => f.write_str("multivariate divisor count limit exceeded"),
            Self::ZeroDivisor => f.write_str("multivariate division by zero polynomial"),
            Self::ResourceLimit => f.write_str("multivariate division step limit exceeded"),
            Self::NondecreasingLeadingTerm => {
                f.write_str("multivariate reduction did not decrease its leading monomial")
            }
            Self::InvalidCertificate => {
                f.write_str("multivariate quotient and remainder failed exact verification")
            }
            Self::Polynomial(error) => write!(f, "{error}"),
        }
    }
}
impl Error for MultivariateDivisionError {}
impl From<MultivariateError> for MultivariateDivisionError {
    fn from(value: MultivariateError) -> Self {
        Self::Polynomial(value)
    }
}

/// The quotient list is in the exact order of the supplied divisors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultivariateDivision {
    quotients: Vec<MultivariatePolynomial>,
    remainder: MultivariatePolynomial,
    steps: usize,
}

impl MultivariateDivision {
    pub fn quotients(&self) -> &[MultivariatePolynomial] {
        &self.quotients
    }

    pub fn remainder(&self) -> &MultivariatePolynomial {
        &self.remainder
    }

    pub const fn steps(&self) -> usize {
        self.steps
    }
}

fn check_divisors(
    dividend: &MultivariatePolynomial,
    divisors: &[MultivariatePolynomial],
) -> Result<(), MultivariateDivisionError> {
    if divisors.is_empty() {
        return Err(MultivariateDivisionError::NoDivisors);
    }
    if divisors.len() > MAX_DIVISORS {
        return Err(MultivariateDivisionError::TooManyDivisors);
    }
    for divisor in divisors {
        if divisor.variables() != dividend.variables() {
            return Err(MultivariateError::VariableMismatch.into());
        }
        if divisor.is_zero() {
            return Err(MultivariateDivisionError::ZeroDivisor);
        }
    }
    Ok(())
}

fn divides(left: &[u32], right: &[u32]) -> bool {
    left.iter().zip(right).all(|(a, b)| a <= b)
}

fn coefficient_quotient(numerator: &Rational, denominator: &Rational) -> Rational {
    // Leading coefficients of stored polynomials are nonzero.
    Rational::new(
        numerator.numerator() * denominator.denominator(),
        numerator.denominator() * denominator.numerator(),
    )
    .expect("a divisor's leading coefficient is nonzero")
}

fn monomial(
    prototype: &MultivariatePolynomial,
    powers: Vec<u32>,
    coefficient: Rational,
    limits: MultivariateLimits,
) -> Result<MultivariatePolynomial, MultivariateDivisionError> {
    let converted = powers
        .into_iter()
        .map(|n| i32::try_from(n).map_err(|_| MultivariateDivisionError::ResourceLimit))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MultivariatePolynomial::try_new(
        prototype.variables().to_vec(),
        [(converted, coefficient)],
        limits,
    )?)
}

/// Multivariate long division with lexicographic monomials in the canonical
/// variable-name order. Divisors are tried strictly from left to right.
/// Resource failure is an error; no partial result is certified.
pub fn divide_multivariate(
    dividend: &MultivariatePolynomial,
    divisors: &[MultivariatePolynomial],
    limits: MultivariateLimits,
) -> Result<MultivariateDivision, MultivariateDivisionError> {
    check_divisors(dividend, divisors)?;
    let mut quotients = Vec::with_capacity(divisors.len());
    for _ in divisors {
        quotients.push(MultivariatePolynomial::zero(
            dividend.variables().to_vec(),
            limits,
        )?);
    }
    let mut remainder = MultivariatePolynomial::zero(dividend.variables().to_vec(), limits)?;
    let mut pending = dividend.clone();
    let mut steps = 0_usize;
    let budget =
        MAX_DIVISION_STEPS.min(usize::try_from(limits.max_work_units).unwrap_or(usize::MAX));

    while let Some((powers, coefficient)) = pending.terms().last_key_value() {
        if steps >= budget {
            return Err(MultivariateDivisionError::ResourceLimit);
        }
        steps += 1;
        let powers = powers.clone();
        let coefficient = coefficient.clone();
        let old_lead = powers.clone();
        let mut reduced = false;
        for (i, divisor) in divisors.iter().enumerate() {
            let (divisor_powers, divisor_coefficient) = divisor
                .terms()
                .last_key_value()
                .expect("all divisors were checked nonzero");
            if !divides(divisor_powers, &powers) {
                continue;
            }
            let difference = powers
                .iter()
                .zip(divisor_powers)
                .map(|(a, b)| a - b)
                .collect();
            let factor = monomial(
                dividend,
                difference,
                coefficient_quotient(&coefficient, divisor_coefficient),
                limits,
            )?;
            let product = factor.multiply(divisor, limits)?;
            let next = pending.subtract(&product, limits)?;
            if next
                .terms()
                .last_key_value()
                .is_some_and(|(key, _)| key >= &old_lead)
            {
                return Err(MultivariateDivisionError::NondecreasingLeadingTerm);
            }
            quotients[i] = quotients[i].add(&factor, limits)?;
            pending = next;
            reduced = true;
            break;
        }
        if !reduced {
            let leading = monomial(dividend, powers, coefficient, limits)?;
            pending = pending.subtract(&leading, limits)?;
            remainder = remainder.add(&leading, limits)?;
        }
    }
    let result = MultivariateDivision {
        quotients,
        remainder,
        steps,
    };
    if !verify_multivariate_division(dividend, divisors, &result, limits)? {
        return Err(MultivariateDivisionError::InvalidCertificate);
    }
    Ok(result)
}

/// Independent quotient/remainder verification through exact recomposition.
/// Checks the ambient ring and that no remainder monomial is divisible by a
/// divisor leading monomial. This is a certificate for polynomial division,
/// not a claim of unique reduction modulo an arbitrary generating set.
pub fn verify_multivariate_division(
    dividend: &MultivariatePolynomial,
    divisors: &[MultivariatePolynomial],
    claimed: &MultivariateDivision,
    limits: MultivariateLimits,
) -> Result<bool, MultivariateDivisionError> {
    check_divisors(dividend, divisors)?;
    if claimed.quotients.len() != divisors.len()
        || claimed.steps > MAX_DIVISION_STEPS
        || claimed.steps > usize::try_from(limits.max_work_units).unwrap_or(usize::MAX)
        || claimed.remainder.variables() != dividend.variables()
        || claimed
            .quotients
            .iter()
            .any(|q| q.variables() != dividend.variables())
    {
        return Ok(false);
    }
    for powers in claimed.remainder.terms().keys() {
        if divisors.iter().any(|g| {
            divides(
                g.terms().last_key_value().expect("nonzero divisor").0,
                powers,
            )
        }) {
            return Ok(false);
        }
    }
    let mut sum = claimed.remainder.clone();
    for (quotient, divisor) in claimed.quotients.iter().zip(divisors) {
        let product = quotient.multiply(divisor, limits)?;
        sum = sum.add(&product, limits)?;
    }
    Ok(&sum == dividend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathema::{Integer, Symbol};

    fn sym(s: &str) -> Symbol {
        Symbol::new(s).unwrap()
    }
    fn p(terms: &[(i32, i32, i64)]) -> MultivariatePolynomial {
        MultivariatePolynomial::try_new(
            vec![sym("x"), sym("y")],
            terms
                .iter()
                .map(|&(x, y, c)| (vec![x, y], Rational::from(Integer::from(c)))),
            MultivariateLimits::default(),
        )
        .unwrap()
    }

    #[test]
    fn forged_quotient_and_remainder_are_rejected() {
        let limits = MultivariateLimits::default();
        let dividend = p(&[(2, 0, 1), (0, 1, 1)]);
        let divisor = p(&[(1, 0, 1), (0, 0, 1)]);
        let result =
            divide_multivariate(&dividend, std::slice::from_ref(&divisor), limits).unwrap();
        assert!(
            verify_multivariate_division(
                &dividend,
                std::slice::from_ref(&divisor),
                &result,
                limits
            )
            .unwrap()
        );
        let mut forged = result.clone();
        forged.quotients[0] = p(&[(1, 0, 5)]);
        assert!(
            !verify_multivariate_division(
                &dividend,
                std::slice::from_ref(&divisor),
                &forged,
                limits
            )
            .unwrap()
        );
        let mut forged = result.clone();
        forged.remainder = p(&[(1, 0, 1)]);
        assert!(!verify_multivariate_division(&dividend, &[divisor], &forged, limits).unwrap());
    }

    #[test]
    fn limit_exhaustion_rejects_without_partial_result() {
        let dividend = p(&[(3, 0, 1), (2, 0, 1), (1, 0, 1), (0, 0, 1)]);
        let divisor = p(&[(1, 0, 1), (0, 0, 1)]);
        let limits = MultivariateLimits {
            max_work_units: 1,
            ..MultivariateLimits::default()
        };
        assert!(matches!(
            divide_multivariate(&dividend, &[divisor], limits),
            Err(MultivariateDivisionError::ResourceLimit
                | MultivariateDivisionError::Polynomial(MultivariateError::WorkLimit))
        ));
    }
}
