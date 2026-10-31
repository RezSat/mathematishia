use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::RationalPolynomial;

/// Polynomial division requires a nonzero divisor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroPolynomialDivisor;

impl fmt::Display for ZeroPolynomialDivisor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("cannot divide by the zero polynomial")
    }
}

impl Error for ZeroPolynomialDivisor {}

fn rational_quotient(numerator: &Rational, denominator: &Rational) -> Rational {
    Rational::new(
        numerator.numerator() * denominator.denominator(),
        numerator.denominator() * denominator.numerator(),
    )
    .expect("the leading coefficient of a nonzero polynomial is nonzero")
}

impl RationalPolynomial {
    /// Divides exactly over Q[x], returning quotient and remainder.
    ///
    /// Guarantees self = divisor*quotient + remainder, where the remainder is
    /// zero or has smaller degree than the divisor.
    pub fn divide_with_remainder(
        &self,
        divisor: &Self,
    ) -> Result<(Self, Self), ZeroPolynomialDivisor> {
        let Some(divisor_degree) = divisor.degree() else {
            return Err(ZeroPolynomialDivisor);
        };

        let Some(dividend_degree) = self.degree() else {
            return Ok((Self::zero(), Self::zero()));
        };
        if dividend_degree < divisor_degree {
            return Ok((Self::zero(), self.clone()));
        }

        let mut quotient = vec![
            Rational::from(Integer::zero());
            dividend_degree - divisor_degree + 1
        ];
        let mut remainder = self.clone();
        let divisor_lead = &divisor.coefficients()[divisor_degree];

        while let Some(remainder_degree) = remainder.degree() {
            if remainder_degree < divisor_degree {
                break;
            }
            let offset = remainder_degree - divisor_degree;
            let leading = &remainder.coefficients()[remainder_degree];
            let factor = rational_quotient(leading, divisor_lead);
            quotient[offset] = &quotient[offset] + &factor;

            let mut subtraction = vec![
                Rational::from(Integer::zero());
                offset + divisor_degree + 1
            ];
            for (index, coefficient) in divisor.coefficients().iter().enumerate() {
                subtraction[offset + index] = coefficient * &factor;
            }
            remainder = remainder.subtract(&Self::new(subtraction));
        }

        Ok((Self::new(quotient), remainder))
    }

    /// Returns the monic greatest common divisor over Q[x].
    ///
    /// GCD(0, 0) is defined as zero. Every other nonzero GCD has leading
    /// coefficient one; nonzero constant GCDs therefore become one.
    pub fn monic_gcd(&self, other: &Self) -> Self {
        let mut a = self.clone();
        let mut b = other.clone();

        while !b.is_zero() {
            let (_, remainder) = a
                .divide_with_remainder(&b)
                .expect("Euclidean division uses a nonzero polynomial");
            a = b;
            b = remainder;
        }

        let Some(degree) = a.degree() else {
            return Self::zero();
        };
        let leading = &a.coefficients()[degree];
        let inverse = rational_quotient(&Rational::from(Integer::one()), leading);
        Self::new(
            a.coefficients()
                .iter()
                .map(|coefficient| coefficient * &inverse)
                .collect(),
        )
    }
}
