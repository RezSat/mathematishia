//! Canonical arithmetic in Q[x]/(m), without assuming m is irreducible.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::poly_division::rational_quotient;
use crate::{EvaluationContext, EvaluationError, RationalPolynomial};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidueError {
    InvalidModulus,
    MismatchedModulus,
    Evaluation(EvaluationError),
}

impl fmt::Display for ResidueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidModulus => f.write_str("polynomial modulus must have positive degree"),
            Self::MismatchedModulus => f.write_str("polynomial residues have different moduli"),
            Self::Evaluation(error) => write!(f, "{error}"),
        }
    }
}

impl Error for ResidueError {}

impl From<EvaluationError> for ResidueError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

/// A canonical coset of Q[x]/(m). A positive-degree modulus is stored monic,
/// and the representative has strictly smaller degree than the modulus.
/// This quotient is a ring; a field is obtained only for irreducible moduli.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialResidue {
    modulus: RationalPolynomial,
    representative: RationalPolynomial,
}

impl PolynomialResidue {
    pub fn new(
        ctx: &EvaluationContext,
        representative: RationalPolynomial,
        modulus: RationalPolynomial,
    ) -> Result<Self, ResidueError> {
        ctx.check_polynomial(&representative)?;
        ctx.check_polynomial(&modulus)?;
        if modulus.degree().is_none_or(|degree| degree == 0) {
            return Err(ResidueError::InvalidModulus);
        }
        let lead = modulus
            .coefficients()
            .last()
            .expect("nonzero polynomial has a leading coefficient");
        let inverse = rational_quotient(&Rational::from(Integer::one()), lead);
        let monic = RationalPolynomial::new(
            modulus
                .coefficients()
                .iter()
                .map(|coefficient| coefficient * &inverse)
                .collect(),
        );
        ctx.check_polynomial(&monic)?;
        let (_, reduced) = ctx.divide(&representative, &monic)?;
        Ok(Self {
            modulus: monic,
            representative: reduced,
        })
    }

    pub fn modulus(&self) -> &RationalPolynomial {
        &self.modulus
    }

    pub fn representative(&self) -> &RationalPolynomial {
        &self.representative
    }

    pub fn is_zero(&self) -> bool {
        self.representative.is_zero()
    }

    fn check_modulus(&self, other: &Self) -> Result<(), ResidueError> {
        if self.modulus == other.modulus {
            Ok(())
        } else {
            Err(ResidueError::MismatchedModulus)
        }
    }

    pub fn add(&self, other: &Self, ctx: &EvaluationContext) -> Result<Self, ResidueError> {
        self.check_modulus(other)?;
        Self::new(ctx, self.representative.add(&other.representative), self.modulus.clone())
    }

    pub fn subtract(&self, other: &Self, ctx: &EvaluationContext) -> Result<Self, ResidueError> {
        self.check_modulus(other)?;
        Self::new(
            ctx,
            self.representative.subtract(&other.representative),
            self.modulus.clone(),
        )
    }

    pub fn multiply(&self, other: &Self, ctx: &EvaluationContext) -> Result<Self, ResidueError> {
        self.check_modulus(other)?;
        Self::new(
            ctx,
            self.representative.multiply(&other.representative),
            self.modulus.clone(),
        )
    }

    /// Returns None for nonunits, including zero divisors in reducible rings.
    /// Polynomial inversion is supplied by the checked Bézout implementation.
    pub fn inverse(&self, ctx: &EvaluationContext) -> Result<Option<Self>, ResidueError> {
        let Some(inverse) = ctx.inverse_mod_polynomial(&self.representative, &self.modulus)? else {
            return Ok(None);
        };
        let value = Self::new(ctx, inverse, self.modulus.clone())?;
        let product = self.multiply(&value, ctx)?;
        if product.representative != RationalPolynomial::new(vec![Rational::from(Integer::one())]) {
            return Err(ResidueError::Evaluation(EvaluationError::ExtendedGcdVerification(
                crate::ExtendedGcdVerificationError::InvalidBezout,
            )));
        }
        Ok(Some(value))
    }

    /// Exponentiation by repeated squaring. A negative exponent needs a unit.
    /// None is returned when the inverse does not exist.
    pub fn pow_signed(
        &self,
        power: i64,
        ctx: &EvaluationContext,
    ) -> Result<Option<Self>, ResidueError> {
        let base = if power < 0 {
            let Some(inverse) = self.inverse(ctx)? else {
                return Ok(None);
            };
            inverse
        } else {
            self.clone()
        };
        let mut n = power.unsigned_abs();
        let mut factor = base;
        let mut result = Self::new(
            ctx,
            RationalPolynomial::new(vec![Rational::from(Integer::one())]),
            self.modulus.clone(),
        )?;
        while n > 0 {
            if n & 1 == 1 {
                result = result.multiply(&factor, ctx)?;
            }
            n >>= 1;
            if n > 0 {
                factor = factor.multiply(&factor, ctx)?;
            }
        }
        Ok(Some(result))
    }
}

/// Checks congruence independently by exact Euclidean division.
/// The inputs and proposed result must use the same canonical modulus.
pub fn verify_residue_product(
    ctx: &EvaluationContext,
    left: &PolynomialResidue,
    right: &PolynomialResidue,
    result: &PolynomialResidue,
) -> Result<bool, ResidueError> {
    left.check_modulus(right)?;
    left.check_modulus(result)?;
    let difference = left
        .representative
        .multiply(&right.representative)
        .subtract(&result.representative);
    let (_, remainder) = ctx.divide(&difference, &left.modulus)?;
    Ok(remainder.is_zero())
}
