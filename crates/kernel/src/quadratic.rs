//! Exact rational roots of degree-at-most-two polynomials over Q.
//!
//! Absence of a rational root does NOT imply absence of real or complex roots.
//! Higher-degree inputs are explicitly unsupported. This is not a general solver.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::budget::WorkBudget;
use crate::{EvaluationError, RationalPolynomial};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuadraticError {
    DegreeExceeded,
}

impl fmt::Display for QuadraticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DegreeExceeded => {
                f.write_str("equation has degree greater than two in the selected variable")
            }
        }
    }
}
impl Error for QuadraticError {}

/// Exactly the rational roots, or all Q for the zero polynomial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuadraticRoots {
    /// Strictly sorted and distinct rational roots (at most two).
    Finite(Vec<Rational>),
    /// The zero polynomial vanishes at every rational number.
    AllRationals,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuadraticResult {
    original: RationalPolynomial,
    roots: QuadraticRoots,
}

impl QuadraticResult {
    pub fn original(&self) -> &RationalPolynomial {
        &self.original
    }

    pub fn roots(&self) -> &QuadraticRoots {
        &self.roots
    }
}

fn coefficient(poly: &RationalPolynomial, power: usize) -> Rational {
    poly.coefficients()
        .get(power)
        .cloned()
        .unwrap_or_else(|| Rational::from(Integer::zero()))
}

/// Generates roots under a shared deterministic arithmetic work budget.
pub(crate) fn checked_quadratic(
    original: &RationalPolynomial,
    work: &mut WorkBudget,
) -> Result<QuadraticResult, EvaluationError> {
    if original.degree().is_some_and(|degree| degree > 2) {
        return Err(QuadraticError::DegreeExceeded.into());
    }
    let roots = match original.degree() {
        None => QuadraticRoots::AllRationals,
        Some(0) => QuadraticRoots::Finite(Vec::new()),
        Some(1) => {
            let root = work.quotient(&(-coefficient(original, 0)), &coefficient(original, 1))?;
            QuadraticRoots::Finite(vec![root])
        }
        Some(2) => {
            let a = coefficient(original, 2);
            let b = coefficient(original, 1);
            let c = coefficient(original, 0);
            let bb = work.multiply_scalar(&b, &b)?;
            let ac = work.multiply_scalar(&a, &c)?;
            let four_ac = work.multiply_scalar(&Rational::from(Integer::from(4)), &ac)?;
            let delta = work.subtract_scalar(&bb, &four_ac)?;
            // Account conservatively for exact integer square-root work.
            let bits = delta
                .numerator()
                .bit_len()
                .saturating_add(delta.denominator().bit_len());
            work.charge(
                bits.saturating_mul(bits)
                    .saturating_div(64)
                    .saturating_add(1),
            )?;
            if let Some(square_root) = delta.exact_sqrt() {
                let denominator = work.multiply_scalar(&Rational::from(Integer::from(2)), &a)?;
                let negative_b = -&b;
                let first = work.add_scalar(&negative_b, &square_root)?;
                let second = work.subtract_scalar(&negative_b, &square_root)?;
                let mut roots = vec![
                    work.quotient(&first, &denominator)?,
                    work.quotient(&second, &denominator)?,
                ];
                roots.sort();
                roots.dedup();
                QuadraticRoots::Finite(roots)
            } else {
                QuadraticRoots::Finite(Vec::new())
            }
        }
        Some(_) => return Err(QuadraticError::DegreeExceeded.into()),
    };
    Ok(QuadraticResult {
        original: original.clone(),
        roots,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuadraticVerificationError {
    UnsupportedDegree,
    IncorrectRoots,
}

impl fmt::Display for QuadraticVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDegree => f.write_str("quadratic verification degree exceeded"),
            Self::IncorrectRoots => f.write_str("incorrect rational root classification"),
        }
    }
}
impl Error for QuadraticVerificationError {}

fn quotient(a: &Rational, b: &Rational) -> Rational {
    Rational::new(
        a.numerator() * b.denominator(),
        a.denominator() * b.numerator(),
    )
    .expect("nonzero linear or quadratic leading coefficient")
}

/// Independently checks completeness via the exact discriminant criterion.
///
/// It does not call the metered generator and also checks every claimed root
/// by exact polynomial substitution (no numerical tolerance).
pub fn verify_quadratic_roots(result: &QuadraticResult) -> Result<(), QuadraticVerificationError> {
    use QuadraticVerificationError as V;
    let original = result.original();
    let expected = match original.degree() {
        None => QuadraticRoots::AllRationals,
        Some(0) => QuadraticRoots::Finite(Vec::new()),
        Some(1) => QuadraticRoots::Finite(vec![quotient(
            &(-coefficient(original, 0)),
            &coefficient(original, 1),
        )]),
        Some(2) => {
            let a = coefficient(original, 2);
            let b = coefficient(original, 1);
            let c = coefficient(original, 0);
            let four = Rational::from(Integer::from(4));
            let delta = &(&b * &b) - &(&four * &(&a * &c));
            let roots = if let Some(sqrt) = delta.exact_sqrt() {
                let two_a = &Rational::from(Integer::from(2)) * &a;
                let minus_b = -&b;
                let mut values = vec![
                    quotient(&(&minus_b + &sqrt), &two_a),
                    quotient(&(&minus_b - &sqrt), &two_a),
                ];
                values.sort();
                values.dedup();
                values
            } else {
                Vec::new()
            };
            QuadraticRoots::Finite(roots)
        }
        Some(_) => return Err(V::UnsupportedDegree),
    };
    if expected != *result.roots() {
        return Err(V::IncorrectRoots);
    }
    if let QuadraticRoots::Finite(values) = result.roots()
        && values
            .iter()
            .any(|value| !original.evaluate(value).numerator().is_zero())
    {
        return Err(V::IncorrectRoots);
    }
    Ok(())
}

#[cfg(test)]
mod adversarial {
    use super::*;
    use crate::EvaluationContext;

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            values
                .iter()
                .copied()
                .map(|n| Rational::from(Integer::from(n)))
                .collect(),
        )
    }

    #[test]
    fn checker_rejects_missing_or_forged_rational_roots() {
        let mut result = EvaluationContext::default()
            .solve_quadratic(&p(&[6, -5, 1]))
            .unwrap();
        result.roots = QuadraticRoots::Finite(vec![Rational::from(Integer::from(2))]);
        assert_eq!(
            verify_quadratic_roots(&result),
            Err(QuadraticVerificationError::IncorrectRoots)
        );
        result.roots = QuadraticRoots::AllRationals;
        assert_eq!(
            verify_quadratic_roots(&result),
            Err(QuadraticVerificationError::IncorrectRoots)
        );
    }
}
