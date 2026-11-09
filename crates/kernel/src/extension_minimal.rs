//! Minimal polynomials in certified finite-dimensional algebraic fields.
//!
//! Dependence of 1,a,a^2,... produces a candidate; a separate verifier
//! checks annihilation and exact irreducibility over Q.

use mathema::{Integer, Rational};

use crate::poly_division::rational_quotient;
use crate::{
    AlgebraicExtensionElement, AlgebraicExtensionError, EvaluationContext, RationalPolynomial,
};

fn zero() -> Rational {
    Rational::from(Integer::zero())
}
fn one() -> Rational {
    Rational::from(Integer::one())
}

fn coefficients(value: &RationalPolynomial, dimension: usize) -> Vec<Rational> {
    (0..dimension)
        .map(|index| value.coefficients().get(index).cloned().unwrap_or_else(zero))
        .collect()
}

/// Solves the exact system c0*1 + ... + c{k-1}*a^{k-1} = -a^k.
/// No numerical pivot threshold or floating-point arithmetic is involved.
fn dependence(
    ctx: &EvaluationContext,
    powers: &[AlgebraicExtensionElement],
    dimension: usize,
    k: usize,
) -> Result<Option<Vec<Rational>>, AlgebraicExtensionError> {
    let mut matrix = (0..dimension)
        .map(|row| {
            let mut values = (0..k)
                .map(|col| coefficients(powers[col].coefficients(), dimension)[row].clone())
                .collect::<Vec<_>>();
            values.push(-&coefficients(powers[k].coefficients(), dimension)[row]);
            values
        })
        .collect::<Vec<_>>();

    for col in 0..k {
        let Some(pivot) = (col..dimension)
            .find(|&row| !matrix[row][col].numerator().is_zero())
        else {
            return Ok(None);
        };
        matrix.swap(col, pivot);
        let inverse = rational_quotient(&one(), &matrix[col][col]);
        for entry in col..=k {
            matrix[col][entry] = &matrix[col][entry] * &inverse;
            ctx.check_rational(&matrix[col][entry])?;
        }
        for row in 0..dimension {
            if row == col || matrix[row][col].numerator().is_zero() {
                continue;
            }
            let factor = matrix[row][col].clone();
            for entry in col..=k {
                let subtracted = &factor * &matrix[col][entry];
                matrix[row][entry] = &matrix[row][entry] - &subtracted;
                ctx.check_rational(&matrix[row][entry])?;
            }
        }
    }
    if matrix.iter().skip(k).any(|row| !row[k].numerator().is_zero()) {
        return Ok(None);
    }
    Ok(Some(matrix[..k].iter().map(|row| row[k].clone()).collect()))
}

/// Checks a proposed monic irreducible annihilator in the specified field.
/// This does not rely on the coefficient-dependence algorithm that generated it.
pub fn verify_extension_minimal_polynomial(
    ctx: &EvaluationContext,
    value: &AlgebraicExtensionElement,
    candidate: &RationalPolynomial,
) -> Result<bool, AlgebraicExtensionError> {
    ctx.check_polynomial(candidate)?;
    let Some(degree) = candidate.degree() else {
        return Ok(false);
    };
    if degree == 0
        || degree > value.field().modulus().degree().unwrap_or(0)
        || candidate.coefficients().last() != Some(&one())
    {
        return Ok(false);
    }

    let field = value.field();
    let mut result = field.element(ctx, RationalPolynomial::zero())?;
    for coefficient in candidate.coefficients().iter().rev() {
        let scalar = field.element(ctx, RationalPolynomial::new(vec![coefficient.clone()]))?;
        result = result.multiply(value, ctx)?.add(&scalar, ctx)?;
    }
    if !result.is_zero() {
        return Ok(false);
    }
    let factorization = ctx.factor_rational_polynomial(candidate)?;
    Ok(factorization.is_complete()
        && factorization.factors().len() == 1
        && factorization.factors()[0].multiplicity() == 1
        && factorization.factors()[0].polynomial() == candidate)
}

impl AlgebraicExtensionElement {
    /// Computes the monic minimal polynomial over Q (degree at most three).
    /// Unsupported large-coefficient irreducibility checks fail explicitly.
    pub fn minimal_polynomial(
        &self,
        ctx: &EvaluationContext,
    ) -> Result<RationalPolynomial, AlgebraicExtensionError> {
        let dimension = self.field().modulus().degree().expect("certified field degree");
        let one_element = self.field().element(
            ctx,
            RationalPolynomial::new(vec![one()]),
        )?;
        let mut powers = vec![one_element];
        for k in 1..=dimension {
            let next = powers[k - 1].multiply(self, ctx)?;
            powers.push(next);
            if let Some(mut coefficients) = dependence(ctx, &powers, dimension, k)? {
                coefficients.push(one());
                let polynomial = RationalPolynomial::new(coefficients);
                if verify_extension_minimal_polynomial(ctx, self, &polynomial)? {
                    return Ok(polynomial);
                }
                return Err(AlgebraicExtensionError::MinimalPolynomialNotCertified);
            }
        }
        Err(AlgebraicExtensionError::MinimalPolynomialNotCertified)
    }
}
