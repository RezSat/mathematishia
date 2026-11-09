//! Exact factor reconstruction from square-free modular factorizations.
//!
//! A candidate is accepted only after exact division over Q[x] succeeds.
//! If the bounded search cannot find a factor, it returns None: that is not
//! an irreducibility certificate.

use mathema::{Integer, Rational};

use crate::{
    EvaluationContext, HenselError, IntegerPolynomial, RationalPolynomial,
    factor_finite_field, hensel_lift_pair, FiniteFieldPolynomial,
};

const PRIMES: [u32; 5] = [2, 3, 5, 7, 11];
const MAX_DEGREE: usize = 8;
const MAX_COEFFICIENT: i64 = 10_000;
const MAX_LIFTS: usize = 8;

fn integral_monic(value: &RationalPolynomial) -> Option<IntegerPolynomial> {
    let degree = value.degree()?;
    if !(2..=MAX_DEGREE).contains(&degree)
        || value.coefficients().last() != Some(&Rational::from(Integer::one()))
    {
        return None;
    }
    let coefficients = value.coefficients().iter().map(|coefficient| {
        if coefficient.denominator() != &Integer::one() {
            return None;
        }
        let n = coefficient.numerator().try_i64()?;
        (n.unsigned_abs() <= MAX_COEFFICIENT as u64).then_some(Integer::from(n))
    }).collect::<Option<Vec<_>>>()?;
    Some(IntegerPolynomial::new(coefficients))
}

fn reduce(
    polynomial: &IntegerPolynomial,
    prime: u32,
) -> Option<FiniteFieldPolynomial> {
    let coefficients = polynomial.coefficients().iter()
        .map(|number| number.modulo_u32(prime).map(i64::from))
        .collect::<Option<Vec<_>>>()?;
    FiniteFieldPolynomial::new(prime, coefficients).ok()
}

fn lift_start(value: &FiniteFieldPolynomial) -> IntegerPolynomial {
    IntegerPolynomial::new(value.coefficients().iter()
        .map(|&number| Integer::from(number))
        .collect())
}

/// Symmetric representatives of Z/(m); these are only trial factors.
/// Their exact integer divisibility is checked independently.
fn centered(poly: &IntegerPolynomial, modulus: &Integer) -> Option<RationalPolynomial> {
    let two = Integer::from(2);
    let coefficients = poly.coefficients().iter().map(|number| {
        let residue = number.modulo_positive(modulus)?;
        let centered = if &residue * &two > *modulus {
            &residue - modulus
        } else {
            residue
        };
        Some(Rational::from(centered))
    }).collect::<Option<Vec<_>>>()?;
    Some(RationalPolynomial::new(coefficients))
}

/// Finds a genuine rational factor using bounded modular factorization,
/// coprime Hensel lifting and an exact division check. Does not infer
/// irreducibility when reconstruction is inconclusive.
pub(crate) fn reconstruct_factor(
    ctx: &EvaluationContext,
    polynomial: &RationalPolynomial,
) -> Result<Option<(RationalPolynomial, RationalPolynomial)>, HenselError> {
    let Some(target) = integral_monic(polynomial) else {
        return Ok(None);
    };
    for prime in PRIMES {
        let Some(reduced) = reduce(&target, prime) else {
            continue;
        };
        let derivative = reduced.derivative();
        if reduced.gcd(&derivative)?.degree() != Some(0) {
            continue;
        }
        let factorization = factor_finite_field(&reduced)?;
        if !factorization.is_complete()
            || factorization.factors().len() < 2
            || factorization.factors().iter().any(|f| f.multiplicity() != 1)
        {
            continue;
        }

        let irreducibles = factorization.factors();
        // Every unordered bipartition is represented exactly once.
        let masks = 1usize << (irreducibles.len() - 1);
        for mask in 1..masks {
            let mut left = FiniteFieldPolynomial::new(prime, vec![1])?;
            let mut right = FiniteFieldPolynomial::new(prime, vec![1])?;
            for (index, factor) in irreducibles.iter().enumerate() {
                if (mask >> index) & 1 == 1 {
                    left = left.multiply(factor.polynomial())?;
                } else {
                    right = right.multiply(factor.polynomial())?;
                }
            }
            if left.degree().is_none_or(|degree| degree == 0)
                || right.degree().is_none_or(|degree| degree == 0)
            {
                continue;
            }
            let mut g = lift_start(&left);
            let mut h = lift_start(&right);
            let mut modulus = Integer::from(prime);
            for _ in 0..=MAX_LIFTS {
                if let Some(candidate) = centered(&g, &modulus) {
                    let (quotient, remainder) = ctx.divide(polynomial, &candidate)?;
                    if remainder.is_zero() && quotient.degree().is_some_and(|d| d > 0) {
                        // A quotient from exact division is the actual
                        // cofactor, regardless of lifted representative.
                        return Ok(Some((candidate, quotient)));
                    }
                }
                let step = hensel_lift_pair(ctx, &target, &g, &h, prime, &modulus)?;
                g = step.left().clone();
                h = step.right().clone();
                modulus = step.modulus_after().clone();
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(coefficients: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(coefficients.iter()
            .map(|&number| Rational::from(Integer::from(number))).collect())
    }

    #[test]
    fn reconstructs_two_irreducible_cubics_with_no_rational_roots() {
        let ctx = EvaluationContext::default();
        let first = p(&[1,1,0,1]);
        let second = p(&[1,0,1,1]);
        let original = first.multiply(&second);
        let (a,b) = reconstruct_factor(&ctx, &original).unwrap().unwrap();
        assert_eq!(a.multiply(&b), original);
        assert!(a.degree().is_some_and(|degree| degree >= 2));
    }

    #[test]
    fn reconstruction_never_guesses_a_factor_of_irreducible_quartic() {
        let ctx = EvaluationContext::default();
        assert_eq!(reconstruct_factor(&ctx, &p(&[1,0,0,0,1])).unwrap(), None);
    }

    #[test]
    fn integral_monic_check_rejects_nonintegral_inputs() {
        let half = Rational::new(Integer::one(), Integer::from(2)).unwrap();
        let value = RationalPolynomial::new(vec![half,Rational::from(Integer::zero()),
            Rational::from(Integer::one())]);
        assert!(integral_monic(&value).is_none());
    }
}
