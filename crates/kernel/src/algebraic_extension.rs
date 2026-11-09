//! Algebraic quotient fields with a certified irreducible defining polynomial.
//!
//! Presentations are abstract Q[alpha]; no real/complex embedding is selected.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::poly_division::rational_quotient;
use crate::{
    EvaluationContext, EvaluationError, PolynomialResidue, RationalFactorization,
    RationalFactorizationError, RationalPolynomial, ResidueError, verify_rational_factorization,
};

fn one() -> Rational {
    Rational::from(Integer::one())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgebraicExtensionError {
    Evaluation(EvaluationError),
    Factorization(RationalFactorizationError),
    Residue(ResidueError),
    UnsupportedDegree,
    IrreducibilityNotCertified,
    DifferentFields,
    DivisionByZero,
    MinimalPolynomialNotCertified,
}

impl fmt::Display for AlgebraicExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for AlgebraicExtensionError {}
impl From<EvaluationError> for AlgebraicExtensionError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}
impl From<RationalFactorizationError> for AlgebraicExtensionError {
    fn from(value: RationalFactorizationError) -> Self {
        Self::Factorization(value)
    }
}
impl From<ResidueError> for AlgebraicExtensionError {
    fn from(value: ResidueError) -> Self {
        Self::Residue(value)
    }
}

/// A canonical abstract number field Q[alpha]/(f(alpha)) of degree 2 through 8.
/// Its factorization evidence proves f irreducible over Q.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertifiedAlgebraicField {
    modulus: RationalPolynomial,
    certificate: RationalFactorization,
}

impl CertifiedAlgebraicField {
    pub fn new(
        ctx: &EvaluationContext,
        defining: RationalPolynomial,
    ) -> Result<Self, AlgebraicExtensionError> {
        ctx.check_polynomial(&defining)?;
        if !defining.degree().is_some_and(|degree| (2..=8).contains(&degree)) {
            return Err(AlgebraicExtensionError::UnsupportedDegree);
        }
        let leading = defining.coefficients().last().expect("nonzero polynomial");
        let inverse = rational_quotient(&one(), leading);
        let monic = RationalPolynomial::new(
            defining
                .coefficients()
                .iter()
                .map(|c| c * &inverse)
                .collect(),
        );
        ctx.check_polynomial(&monic)?;
        let factorization = ctx.factor_rational_polynomial(&monic)?;
        let result = Self {
            modulus: monic,
            certificate: factorization,
        };
        if !verify_certified_algebraic_field(&result) {
            return Err(AlgebraicExtensionError::IrreducibilityNotCertified);
        }
        Ok(result)
    }

    pub fn modulus(&self) -> &RationalPolynomial {
        &self.modulus
    }
    pub fn certificate(&self) -> &RationalFactorization {
        &self.certificate
    }

    pub fn element(
        &self,
        ctx: &EvaluationContext,
        coefficients: RationalPolynomial,
    ) -> Result<AlgebraicExtensionElement, AlgebraicExtensionError> {
        AlgebraicExtensionElement::new(ctx, self.clone(), coefficients)
    }

    pub fn generator(
        &self,
        ctx: &EvaluationContext,
    ) -> Result<AlgebraicExtensionElement, AlgebraicExtensionError> {
        self.element(
            ctx,
            RationalPolynomial::new(vec![Rational::from(Integer::zero()), one()]),
        )
    }
}

/// Verifies the defining polynomial is exactly one certified irreducible factor.
pub fn verify_certified_algebraic_field(field: &CertifiedAlgebraicField) -> bool {
    if !field.modulus.degree().is_some_and(|degree| (2..=8).contains(&degree))
        || field.modulus.coefficients().last() != Some(&one())
        || verify_rational_factorization(&field.modulus, &field.certificate).is_err()
        || !field.certificate.is_complete()
        || field.certificate.factors().len() != 1
    {
        return false;
    }
    let factor = &field.certificate.factors()[0];
    factor.multiplicity() == 1 && factor.polynomial() == &field.modulus
}

/// Exact residue coordinates in a certified Q-algebraic field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicExtensionElement {
    field: CertifiedAlgebraicField,
    residue: PolynomialResidue,
}

impl AlgebraicExtensionElement {
    pub fn new(
        ctx: &EvaluationContext,
        field: CertifiedAlgebraicField,
        coefficients: RationalPolynomial,
    ) -> Result<Self, AlgebraicExtensionError> {
        let residue = PolynomialResidue::new(ctx, coefficients, field.modulus.clone())?;
        Ok(Self { field, residue })
    }

    pub fn field(&self) -> &CertifiedAlgebraicField {
        &self.field
    }
    pub fn coefficients(&self) -> &RationalPolynomial {
        self.residue.representative()
    }
    pub fn is_zero(&self) -> bool {
        self.residue.is_zero()
    }

    fn compatible(&self, other: &Self) -> Result<(), AlgebraicExtensionError> {
        if self.field.modulus == other.field.modulus {
            Ok(())
        } else {
            Err(AlgebraicExtensionError::DifferentFields)
        }
    }

    pub fn add(
        &self,
        other: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, AlgebraicExtensionError> {
        self.compatible(other)?;
        Ok(Self {
            field: self.field.clone(),
            residue: self.residue.add(&other.residue, ctx)?,
        })
    }

    pub fn subtract(
        &self,
        other: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, AlgebraicExtensionError> {
        self.compatible(other)?;
        Ok(Self {
            field: self.field.clone(),
            residue: self.residue.subtract(&other.residue, ctx)?,
        })
    }

    pub fn multiply(
        &self,
        other: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, AlgebraicExtensionError> {
        self.compatible(other)?;
        Ok(Self {
            field: self.field.clone(),
            residue: self.residue.multiply(&other.residue, ctx)?,
        })
    }

    pub fn inverse(&self, ctx: &EvaluationContext) -> Result<Self, AlgebraicExtensionError> {
        let Some(inverse) = self.residue.inverse(ctx)? else {
            return Err(AlgebraicExtensionError::DivisionByZero);
        };
        Ok(Self {
            field: self.field.clone(),
            residue: inverse,
        })
    }

    pub fn divide(
        &self,
        other: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, AlgebraicExtensionError> {
        self.compatible(other)?;
        self.multiply(&other.inverse(ctx)?, ctx)
    }

    pub fn pow_signed(
        &self,
        exponent: i64,
        ctx: &EvaluationContext,
    ) -> Result<Self, AlgebraicExtensionError> {
        let Some(value) = self.residue.pow_signed(exponent, ctx)? else {
            return Err(AlgebraicExtensionError::DivisionByZero);
        };
        Ok(Self {
            field: self.field.clone(),
            residue: value,
        })
    }
}
