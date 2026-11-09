//! Exhaustive bounded quadratic splitting for monic integral quartics.
//!
//! A monic factor over Q of a monic integer polynomial has integer
//! coefficients. Cauchy's root bound limits the linear coefficient of
//! a quadratic factor; its constant term divides the quartic constant.

use mathema::{Integer, Rational};

use crate::{EvaluationContext, EvaluationError, RationalPolynomial};

const MAX_ROOT_BOUND: u64 = 40;
const MAX_SEARCH_PAIRS: usize = 8_192;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QuarticSearch {
    Split(RationalPolynomial, RationalPolynomial),
    Exhausted,
    Inconclusive,
}

fn divisors(value: u64) -> Vec<u64> {
    let mut divisors = Vec::new();
    let mut candidate = 1_u64;
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

/// `Exhausted` proves there is no monic quadratic divisor in Z[x].
/// A quartic without rational roots is then irreducible over Q.
pub(crate) fn search_quartic_quadratic(
    polynomial: &RationalPolynomial,
    ctx: &EvaluationContext,
) -> Result<QuarticSearch, EvaluationError> {
    if polynomial.degree() != Some(4)
        || polynomial.coefficients()[4] != Rational::from(Integer::one())
    {
        return Ok(QuarticSearch::Inconclusive);
    }
    let integers = polynomial
        .coefficients()
        .iter()
        .map(|coefficient| {
            if !coefficient.denominator().is_one() {
                return None;
            }
            coefficient.numerator().try_i64()
        })
        .collect::<Option<Vec<_>>>();
    let Some(values) = integers else {
        return Ok(QuarticSearch::Inconclusive);
    };
    if values[0] == 0 {
        return Ok(QuarticSearch::Inconclusive);
    }

    let max_coefficient = values[..4]
        .iter()
        .map(|value| value.unsigned_abs())
        .max()
        .unwrap_or(0);
    let bound = max_coefficient.saturating_add(1);
    if bound > MAX_ROOT_BOUND {
        return Ok(QuarticSearch::Inconclusive);
    }
    let constant_divisors = divisors(values[0].unsigned_abs());
    let linear_bound = (2 * bound) as i64;
    let trial_count = (2 * constant_divisors.len()).saturating_mul((2 * linear_bound + 1) as usize);
    if trial_count > MAX_SEARCH_PAIRS {
        return Ok(QuarticSearch::Inconclusive);
    }
    for constant in constant_divisors {
        for sign in [-1_i64, 1] {
            let value = sign * constant as i64;
            // Every root has complex modulus < 1+max |a_i|.
            // Thus the sum of two roots has absolute value < 2*bound.
            for linear in -linear_bound..=linear_bound {
                let divisor = RationalPolynomial::new(vec![
                    Rational::from(Integer::from(value)),
                    Rational::from(Integer::from(linear)),
                    Rational::from(Integer::one()),
                ]);
                let (quotient, remainder) = ctx.divide(polynomial, &divisor)?;
                if remainder.is_zero() {
                    return Ok(QuarticSearch::Split(divisor, quotient));
                }
            }
        }
    }
    Ok(QuarticSearch::Exhausted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            values
                .iter()
                .map(|&n| Rational::from(Integer::from(n)))
                .collect(),
        )
    }

    #[test]
    fn splits_sophie_germain_quartic() {
        let original = p(&[4, 0, 0, 0, 1]);
        let found = search_quartic_quadratic(&original, &EvaluationContext::default()).unwrap();
        let QuarticSearch::Split(first, second) = found else {
            panic!("must split")
        };
        assert_eq!(first.multiply(&second), original);
        assert_eq!(first.degree(), Some(2));
        assert_eq!(second.degree(), Some(2));
    }

    #[test]
    fn certifies_x_four_plus_one_by_exhaustion() {
        assert_eq!(
            search_quartic_quadratic(&p(&[1, 0, 0, 0, 1]), &EvaluationContext::default()).unwrap(),
            QuarticSearch::Exhausted,
        );
    }

    #[test]
    fn unsupported_nonmonic_and_large_inputs_are_not_certified() {
        let ctx = EvaluationContext::default();
        assert_eq!(
            search_quartic_quadratic(&p(&[1, 0, 0, 0, 2]), &ctx).unwrap(),
            QuarticSearch::Inconclusive
        );
        assert_eq!(
            search_quartic_quadratic(&p(&[41, 0, 0, 0, 1]), &ctx).unwrap(),
            QuarticSearch::Inconclusive
        );
    }
}
