//! Exact extended Euclidean algorithm over Q[x] and verifiable Bézout identities.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::{EvaluationContext, EvaluationError, RationalPolynomial};

fn one() -> RationalPolynomial {
    RationalPolynomial::new(vec![Rational::from(Integer::one())])
}

fn scale(polynomial: &RationalPolynomial, factor: &Rational) -> RationalPolynomial {
    RationalPolynomial::new(
        polynomial
            .coefficients()
            .iter()
            .map(|coefficient| coefficient * factor)
            .collect(),
    )
}

/// Exact Euclidean division record. Every step satisfies a = q*b + r.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EuclideanStep {
    dividend: RationalPolynomial,
    divisor: RationalPolynomial,
    quotient: RationalPolynomial,
    remainder: RationalPolynomial,
}

impl EuclideanStep {
    pub fn dividend(&self) -> &RationalPolynomial {
        &self.dividend
    }
    pub fn divisor(&self) -> &RationalPolynomial {
        &self.divisor
    }
    pub fn quotient(&self) -> &RationalPolynomial {
        &self.quotient
    }
    pub fn remainder(&self) -> &RationalPolynomial {
        &self.remainder
    }
}

/// The monic gcd and s,t in Q[x] such that s*a + t*b = gcd(a,b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtendedGcdResult {
    left: RationalPolynomial,
    right: RationalPolynomial,
    gcd: RationalPolynomial,
    left_coefficient: RationalPolynomial,
    right_coefficient: RationalPolynomial,
    divisions: Vec<EuclideanStep>,
}

impl ExtendedGcdResult {
    pub fn left(&self) -> &RationalPolynomial {
        &self.left
    }
    pub fn right(&self) -> &RationalPolynomial {
        &self.right
    }
    pub fn gcd(&self) -> &RationalPolynomial {
        &self.gcd
    }
    pub fn left_coefficient(&self) -> &RationalPolynomial {
        &self.left_coefficient
    }
    pub fn right_coefficient(&self) -> &RationalPolynomial {
        &self.right_coefficient
    }
    pub fn divisions(&self) -> &[EuclideanStep] {
        &self.divisions
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtendedGcdVerificationError {
    InvalidDivision,
    IncompleteChain,
    InvalidGcd,
    InvalidBezout,
}

impl fmt::Display for ExtendedGcdVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDivision => f.write_str("invalid Euclidean division evidence"),
            Self::IncompleteChain => f.write_str("incomplete Euclidean division chain"),
            Self::InvalidGcd => f.write_str("incorrect or nonmonic polynomial gcd"),
            Self::InvalidBezout => f.write_str("incorrect polynomial Bézout identity"),
        }
    }
}

impl Error for ExtendedGcdVerificationError {}

/// Checks the exact division identities and their complete remainder chain,
/// then checks the final gcd and Bézout identity. It does not regenerate the
/// quotient/remainder sequence.
pub fn verify_extended_gcd(result: &ExtendedGcdResult) -> Result<(), ExtendedGcdVerificationError> {
    use ExtendedGcdVerificationError as E;

    let mut a = result.left.clone();
    let mut b = result.right.clone();
    for step in &result.divisions {
        if b.is_zero() || step.dividend != a || step.divisor != b {
            return Err(E::InvalidDivision);
        }
        if step.quotient.multiply(&b).add(&step.remainder) != a {
            return Err(E::InvalidDivision);
        }
        if step
            .remainder
            .degree()
            .is_some_and(|degree| b.degree().is_none_or(|bound| degree >= bound))
        {
            return Err(E::InvalidDivision);
        }
        a = b;
        b = step.remainder.clone();
    }
    if !b.is_zero() {
        return Err(E::IncompleteChain);
    }

    let expected = if a.is_zero() {
        RationalPolynomial::zero()
    } else {
        let leading = a
            .coefficients()
            .last()
            .expect("nonzero polynomial has a leading coefficient");
        let inverse = Rational::new(leading.denominator().clone(), leading.numerator().clone())
            .expect("leading coefficient is nonzero");
        scale(&a, &inverse)
    };

    if result.gcd != expected {
        return Err(E::InvalidGcd);
    }
    if result
        .left_coefficient
        .multiply(&result.left)
        .add(&result.right_coefficient.multiply(&result.right))
        != result.gcd
    {
        return Err(E::InvalidBezout);
    }
    Ok(())
}

impl EvaluationContext {
    /// Extended Euclidean algorithm with exact, checked intermediate values.
    /// The recorded chain and final Bézout identity are verified independently.
    pub fn extended_gcd(
        &self,
        left: &RationalPolynomial,
        right: &RationalPolynomial,
    ) -> Result<ExtendedGcdResult, EvaluationError> {
        self.check_polynomial(left)?;
        self.check_polynomial(right)?;

        let mut a = left.clone();
        let mut b = right.clone();
        let mut s0 = one();
        let mut s1 = RationalPolynomial::zero();
        let mut t0 = RationalPolynomial::zero();
        let mut t1 = one();
        let mut divisions = Vec::new();
        while !b.is_zero() {
            let (q, remainder) = a.divide_with_remainder(&b)?;
            self.check_polynomial(&q)?;
            self.check_polynomial(&remainder)?;
            let s2 = s0.subtract(&q.multiply(&s1));
            let t2 = t0.subtract(&q.multiply(&t1));
            self.check_polynomial(&s2)?;
            self.check_polynomial(&t2)?;
            divisions.push(EuclideanStep {
                dividend: a.clone(),
                divisor: b.clone(),
                quotient: q,
                remainder: remainder.clone(),
            });
            a = b;
            b = remainder;
            s0 = s1;
            s1 = s2;
            t0 = t1;
            t1 = t2;
        }

        let (gcd, left_coefficient, right_coefficient) = if a.is_zero() {
            (
                RationalPolynomial::zero(),
                RationalPolynomial::zero(),
                RationalPolynomial::zero(),
            )
        } else {
            let leading = a
                .coefficients()
                .last()
                .expect("nonzero polynomial has a leading coefficient");
            let inverse = Rational::new(leading.denominator().clone(), leading.numerator().clone())
                .expect("leading coefficient is nonzero");
            (
                scale(&a, &inverse),
                scale(&s0, &inverse),
                scale(&t0, &inverse),
            )
        };

        for polynomial in [&gcd, &left_coefficient, &right_coefficient] {
            self.check_polynomial(polynomial)?;
        }
        let result = ExtendedGcdResult {
            left: left.clone(),
            right: right.clone(),
            gcd,
            left_coefficient,
            right_coefficient,
            divisions,
        };
        verify_extended_gcd(&result).map_err(EvaluationError::ExtendedGcdVerification)?;
        Ok(result)
    }

    /// Multiplicative inverse of a modulo a nonzero polynomial, if it exists.
    /// The inverse is reduced to degree less than the modulus.
    pub fn inverse_mod_polynomial(
        &self,
        value: &RationalPolynomial,
        modulus: &RationalPolynomial,
    ) -> Result<Option<RationalPolynomial>, EvaluationError> {
        if modulus.is_zero() {
            return Err(EvaluationError::DivisionByZero);
        }
        let evidence = self.extended_gcd(value, modulus)?;
        if evidence.gcd() != &one() {
            return Ok(None);
        }
        let (_, inverse) = evidence.left_coefficient().divide_with_remainder(modulus)?;
        self.check_polynomial(&inverse)?;
        Ok(Some(inverse))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(coefficients: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            coefficients
                .iter()
                .map(|&value| Rational::from(Integer::from(value)))
                .collect(),
        )
    }

    #[test]
    fn rejects_modified_euclidean_and_bezout_evidence() {
        let ctx = EvaluationContext::default();
        let result = ctx
            .extended_gcd(&p(&[-1, 0, 0, 1]), &p(&[-1, 0, 1]))
            .unwrap();
        assert_eq!(verify_extended_gcd(&result), Ok(()));

        let mut forged = result.clone();
        forged.divisions[0].quotient = p(&[42]);
        assert_eq!(
            verify_extended_gcd(&forged),
            Err(ExtendedGcdVerificationError::InvalidDivision)
        );

        let mut forged = result.clone();
        forged.divisions.pop();
        assert_eq!(
            verify_extended_gcd(&forged),
            Err(ExtendedGcdVerificationError::IncompleteChain)
        );

        let mut forged = result.clone();
        forged.left_coefficient = p(&[20]);
        assert_eq!(
            verify_extended_gcd(&forged),
            Err(ExtendedGcdVerificationError::InvalidBezout)
        );

        let mut forged = result;
        forged.gcd = p(&[2]);
        assert_eq!(
            verify_extended_gcd(&forged),
            Err(ExtendedGcdVerificationError::InvalidGcd)
        );
    }
}
