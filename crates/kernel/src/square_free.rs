//! Exact square-free decomposition of nonzero polynomials over Q[x].
//!
//! This separates multiplicity classes; it does not claim irreducibility or
//! compute roots. All factors are monic and sorted by increasing multiplicity.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::RationalPolynomial;
use crate::poly_division::rational_quotient;

/// Square-free decomposition is undefined for zero; exact divisions must hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquareFreeError {
    ZeroPolynomial,
    NonExactDivision,
}

impl fmt::Display for SquareFreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroPolynomial => f.write_str("square-free decomposition is undefined for zero"),
            Self::NonExactDivision => {
                f.write_str("square-free algorithm encountered an inexact division")
            }
        }
    }
}
impl Error for SquareFreeError {}

/// One monic, nonconstant, square-free polynomial at a single multiplicity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SquareFreeFactor {
    polynomial: RationalPolynomial,
    multiplicity: usize,
}

impl SquareFreeFactor {
    pub fn polynomial(&self) -> &RationalPolynomial {
        &self.polynomial
    }

    pub const fn multiplicity(&self) -> usize {
        self.multiplicity
    }

    pub(crate) fn from_parts(polynomial: RationalPolynomial, multiplicity: usize) -> Self {
        Self {
            polynomial,
            multiplicity,
        }
    }
}

/// Exact decomposition of a nonzero polynomial as unit * product(factor^m).
///
/// The unit is the original leading coefficient. Factors are monic,
/// nonconstant, individually square-free, pairwise coprime, and ordered by
/// strictly increasing multiplicity. Constants have no factors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SquareFreeDecomposition {
    unit: Rational,
    factors: Vec<SquareFreeFactor>,
}

impl SquareFreeDecomposition {
    pub fn unit(&self) -> &Rational {
        &self.unit
    }

    pub fn factors(&self) -> &[SquareFreeFactor] {
        &self.factors
    }

    pub(crate) fn from_parts(unit: Rational, factors: Vec<SquareFreeFactor>) -> Self {
        Self { unit, factors }
    }
}

fn one() -> RationalPolynomial {
    RationalPolynomial::new(vec![Rational::from(Integer::one())])
}

fn divide_exact(
    numerator: &RationalPolynomial,
    divisor: &RationalPolynomial,
) -> Result<RationalPolynomial, SquareFreeError> {
    let (quotient, remainder) = numerator
        .divide_with_remainder(divisor)
        .map_err(|_| SquareFreeError::NonExactDivision)?;
    if !remainder.is_zero() {
        return Err(SquareFreeError::NonExactDivision);
    }
    Ok(quotient)
}

impl RationalPolynomial {
    /// Separates distinct multiplicity classes over characteristic-zero Q[x].
    ///
    /// Uses repeated exact GCDs with the formal derivative. The algorithm is
    /// deliberately implemented over our established dense polynomial domain;
    /// it is not a full irreducible factorization procedure.
    pub fn square_free_decomposition(&self) -> Result<SquareFreeDecomposition, SquareFreeError> {
        let Some(leading) = self.coefficients().last() else {
            return Err(SquareFreeError::ZeroPolynomial);
        };
        let unit = leading.clone();
        let normalized = Self::new(
            self.coefficients()
                .iter()
                .map(|coefficient| rational_quotient(coefficient, &unit))
                .collect(),
        );
        let mut gcd = normalized.monic_gcd(&normalized.differentiate());
        let mut remaining = divide_exact(&normalized, &gcd)?;
        let mut multiplicity = 1usize;
        let mut factors = Vec::new();

        while remaining.degree().is_some_and(|degree| degree > 0) {
            let common = remaining.monic_gcd(&gcd);
            let factor = divide_exact(&remaining, &common)?;
            if factor.degree().is_some_and(|degree| degree > 0) {
                factors.push(SquareFreeFactor {
                    polynomial: factor,
                    multiplicity,
                });
            }
            remaining = common.clone();
            gcd = divide_exact(&gcd, &common)?;
            multiplicity += 1;
        }
        Ok(SquareFreeDecomposition { unit, factors })
    }
}

/// Reasons why a proposed square-free decomposition is not a valid certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquareFreeVerificationError {
    ZeroPolynomial,
    InvalidUnit,
    InvalidFactor,
    InvalidMultiplicity,
    NotSquareFree,
    NotCoprime,
    WrongDegree,
    ProductMismatch,
}

impl fmt::Display for SquareFreeVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroPolynomial => "cannot verify a square-free decomposition of zero",
            Self::InvalidUnit => "decomposition has an incorrect or zero leading coefficient",
            Self::InvalidFactor => "decomposition contains a constant or nonmonic factor",
            Self::InvalidMultiplicity => "multiplicities must be positive and strictly increasing",
            Self::NotSquareFree => "a reported factor contains repeated irreducible factors",
            Self::NotCoprime => "factors at different multiplicities are not coprime",
            Self::WrongDegree => "factor multiplicities do not reconstruct the original degree",
            Self::ProductMismatch => "factor product does not reconstruct the original polynomial",
        };
        f.write_str(message)
    }
}

impl Error for SquareFreeVerificationError {}

/// Checks an independently supplied decomposition using exact polynomial math.
///
/// Checks the unit, factor monicity, multiplicities, exact degree, square-free
/// property, pairwise coprimality and full reconstruction. No floating-point
/// evaluation or trust in the factor generator is involved.
pub fn verify_square_free_decomposition(
    original: &RationalPolynomial,
    result: &SquareFreeDecomposition,
) -> Result<(), SquareFreeVerificationError> {
    let Some(leading) = original.coefficients().last() else {
        return Err(SquareFreeVerificationError::ZeroPolynomial);
    };
    if result.unit.numerator().is_zero() || &result.unit != leading {
        return Err(SquareFreeVerificationError::InvalidUnit);
    }

    let one = one();
    let mut degree = 0usize;
    let mut previous = 0usize;
    let max_degree = original.degree().unwrap_or(0);

    for item in &result.factors {
        let factor = &item.polynomial;
        let Some(factor_degree) = factor.degree() else {
            return Err(SquareFreeVerificationError::InvalidFactor);
        };
        if factor_degree == 0 || factor.coefficients().last() != one.coefficients().last() {
            return Err(SquareFreeVerificationError::InvalidFactor);
        }
        if item.multiplicity <= previous {
            return Err(SquareFreeVerificationError::InvalidMultiplicity);
        }
        previous = item.multiplicity;
        degree = factor_degree
            .checked_mul(item.multiplicity)
            .and_then(|size| degree.checked_add(size))
            .ok_or(SquareFreeVerificationError::WrongDegree)?;
        if degree > max_degree {
            return Err(SquareFreeVerificationError::WrongDegree);
        }
        if factor.monic_gcd(&factor.differentiate()) != one {
            return Err(SquareFreeVerificationError::NotSquareFree);
        }
    }
    if degree != max_degree {
        return Err(SquareFreeVerificationError::WrongDegree);
    }

    for (index, first) in result.factors.iter().enumerate() {
        for second in result.factors.iter().skip(index + 1) {
            if first.polynomial.monic_gcd(&second.polynomial) != one {
                return Err(SquareFreeVerificationError::NotCoprime);
            }
        }
    }

    let mut reconstructed = one;
    for entry in &result.factors {
        for _ in 0..entry.multiplicity {
            reconstructed = reconstructed.multiply(&entry.polynomial);
        }
    }
    let reconstructed = RationalPolynomial::new(
        reconstructed
            .coefficients()
            .iter()
            .map(|coefficient| coefficient * &result.unit)
            .collect(),
    );
    if &reconstructed != original {
        return Err(SquareFreeVerificationError::ProductMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    fn p(coeffs: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            coeffs
                .iter()
                .map(|&value| Rational::from(Integer::from(value)))
                .collect(),
        )
    }

    #[test]
    fn verifier_rejects_forged_unit_and_multiplicity() {
        let original = p(&[1, 2, 1]);
        let result = original.square_free_decomposition().unwrap();
        let mut forged = result.clone();
        forged.unit = Rational::from(Integer::from(2));
        assert_eq!(
            verify_square_free_decomposition(&original, &forged),
            Err(SquareFreeVerificationError::InvalidUnit)
        );
        let mut forged = result.clone();
        forged.factors[0].multiplicity = 3;
        assert_eq!(
            verify_square_free_decomposition(&original, &forged),
            Err(SquareFreeVerificationError::WrongDegree)
        );
    }

    #[test]
    fn verifier_rejects_nonmonic_and_repeated_factors() {
        let original = p(&[1, 2, 1]);
        let mut forged = original.square_free_decomposition().unwrap();
        forged.factors[0].polynomial = p(&[2, 2]);
        assert_eq!(
            verify_square_free_decomposition(&original, &forged),
            Err(SquareFreeVerificationError::InvalidFactor)
        );
        let mut forged = original.square_free_decomposition().unwrap();
        forged.factors[0].polynomial = original.clone();
        forged.factors[0].multiplicity = 1;
        assert_eq!(
            verify_square_free_decomposition(&original, &forged),
            Err(SquareFreeVerificationError::NotSquareFree)
        );
    }

    #[test]
    fn verifier_rejects_non_coprime_multiplicity_classes() {
        let first = p(&[1, 1]);
        let second = p(&[2, 1]);
        let original = first
            .multiply(&first)
            .multiply(&second)
            .multiply(&second)
            .multiply(&second);
        let mut forged = original.square_free_decomposition().unwrap();
        forged.factors[1].polynomial = first;
        assert_eq!(
            verify_square_free_decomposition(&original, &forged),
            Err(SquareFreeVerificationError::NotCoprime)
        );
    }

    #[test]
    fn verifier_rejects_equal_multiplicity_labels() {
        let first = p(&[1, 1]);
        let second = p(&[2, 1]);
        let original = first
            .multiply(&first)
            .multiply(&second)
            .multiply(&second)
            .multiply(&second);
        let mut forged = original.square_free_decomposition().unwrap();
        forged.factors[1].multiplicity = forged.factors[0].multiplicity;
        assert_eq!(
            verify_square_free_decomposition(&original, &forged),
            Err(SquareFreeVerificationError::InvalidMultiplicity)
        );
    }

    #[test]
    fn verifier_rejects_wrong_product_without_approximation() {
        let original = p(&[1, 2, 1]);
        let mut forged = original.square_free_decomposition().unwrap();
        forged.factors[0].polynomial = p(&[2, 1]);
        assert_eq!(
            verify_square_free_decomposition(&original, &forged),
            Err(SquareFreeVerificationError::ProductMismatch)
        );
    }
}
