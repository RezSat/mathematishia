//! Exact rational functions as partial functions over Q, including removed poles.
//!
//! GCD cancellation never silently fills holes in the original domain. All
//! excluded zero sets are retained as monic polynomial nonzero obligations.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::RationalPolynomial;
use crate::poly_division::rational_quotient;

/// Errors for exact rational functions and evaluation outside their domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalFunctionError {
    ZeroDenominator,
    ZeroDivisor,
    UndefinedAtPoint,
    NonExactReduction,
}

impl fmt::Display for RationalFunctionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDenominator => f.write_str("rational-function denominator is zero"),
            Self::ZeroDivisor => f.write_str("cannot divide by the zero rational function"),
            Self::UndefinedAtPoint => f.write_str("rational function is undefined at this point"),
            Self::NonExactReduction => f.write_str("rational-function GCD reduction was not exact"),
        }
    }
}

impl Error for RationalFunctionError {}

fn one() -> Rational {
    Rational::from(Integer::one())
}

fn unit() -> RationalPolynomial {
    RationalPolynomial::new(vec![one()])
}

/// Canonical total order for nonzero-condition polynomials; not mathematical order.
pub(crate) fn condition_order(a: &RationalPolynomial, b: &RationalPolynomial) -> Ordering {
    a.coefficients()
        .len()
        .cmp(&b.coefficients().len())
        .then_with(|| a.coefficients().cmp(b.coefficients()))
}

fn monic(polynomial: &RationalPolynomial) -> RationalPolynomial {
    let Some(leading) = polynomial.coefficients().last() else {
        return RationalPolynomial::zero();
    };
    let inverse = rational_quotient(&one(), leading);
    RationalPolynomial::new(
        polynomial
            .coefficients()
            .iter()
            .map(|coefficient| coefficient * &inverse)
            .collect(),
    )
}

fn normalize_conditions(mut exclusions: Vec<RationalPolynomial>) -> Vec<RationalPolynomial> {
    exclusions.retain(|poly| poly.degree().is_some_and(|degree| degree > 0));
    for poly in &mut exclusions {
        *poly = monic(poly);
    }
    exclusions.sort_by(condition_order);
    exclusions.dedup();
    exclusions
}

fn divide_exact(
    numerator: &RationalPolynomial,
    divisor: &RationalPolynomial,
) -> Result<RationalPolynomial, RationalFunctionError> {
    let (quotient, remainder) = numerator
        .divide_with_remainder(divisor)
        .map_err(|_| RationalFunctionError::NonExactReduction)?;
    if !remainder.is_zero() {
        return Err(RationalFunctionError::NonExactReduction);
    }
    Ok(quotient)
}

/// A reduced p(x)/q(x) with all excluded zero sets explicitly retained.
///
/// Equality includes domain restrictions, so it is stronger than equality in
/// Q(x). Numerator and denominator are coprime; the denominator is monic.
/// Conditions are unique, sorted, monic, and nonconstant. A zero numerator has
/// denominator one but can retain holes inherited from its source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalFunction {
    numerator: RationalPolynomial,
    denominator: RationalPolynomial,
    excluded: Vec<RationalPolynomial>,
}

impl RationalFunction {
    /// Constructs a partial function, preserving zeros of the original q(x).
    pub fn new(
        numerator: RationalPolynomial,
        denominator: RationalPolynomial,
    ) -> Result<Self, RationalFunctionError> {
        let original = denominator.clone();
        Self::reduce(numerator, denominator, vec![original])
    }

    pub fn from_polynomial(numerator: RationalPolynomial) -> Self {
        Self {
            numerator,
            denominator: unit(),
            excluded: Vec::new(),
        }
    }

    pub fn numerator(&self) -> &RationalPolynomial {
        &self.numerator
    }

    pub fn denominator(&self) -> &RationalPolynomial {
        &self.denominator
    }

    /// Each polynomial represents the exact side condition p(x) != 0.
    pub fn excluded_zero_sets(&self) -> &[RationalPolynomial] {
        &self.excluded
    }

    pub fn is_zero(&self) -> bool {
        self.numerator.is_zero()
    }

    /// Algebraic equality in Q(x), deliberately ignoring domain holes.
    pub fn algebraically_equivalent(&self, other: &Self) -> bool {
        self.numerator == other.numerator && self.denominator == other.denominator
    }

    /// Caller must provide verified canonical parts and normalized conditions.
    pub(crate) fn from_canonical(
        numerator: RationalPolynomial,
        denominator: RationalPolynomial,
        excluded: Vec<RationalPolynomial>,
    ) -> Self {
        Self {
            numerator,
            denominator,
            excluded,
        }
    }

    fn reduce(
        numerator: RationalPolynomial,
        denominator: RationalPolynomial,
        excluded: Vec<RationalPolynomial>,
    ) -> Result<Self, RationalFunctionError> {
        if denominator.is_zero() {
            return Err(RationalFunctionError::ZeroDenominator);
        }
        let gcd = numerator.monic_gcd(&denominator);
        let n = divide_exact(&numerator, &gcd)?;
        let d = divide_exact(&denominator, &gcd)?;
        let leading = d
            .coefficients()
            .last()
            .expect("exact GCD division preserves nonzero denominator");
        let inverse = rational_quotient(&one(), leading);
        let scale = |poly: &RationalPolynomial| {
            RationalPolynomial::new(
                poly.coefficients()
                    .iter()
                    .map(|coefficient| coefficient * &inverse)
                    .collect(),
            )
        };
        Ok(Self {
            numerator: scale(&n),
            denominator: scale(&d),
            excluded: normalize_conditions(excluded),
        })
    }

    fn combined_conditions(&self, other: &Self) -> Vec<RationalPolynomial> {
        self.excluded
            .iter()
            .chain(other.excluded.iter())
            .cloned()
            .collect()
    }

    pub fn add(&self, other: &Self) -> Result<Self, RationalFunctionError> {
        let left = self.numerator.multiply(&other.denominator);
        let right = other.numerator.multiply(&self.denominator);
        Self::reduce(
            left.add(&right),
            self.denominator.multiply(&other.denominator),
            self.combined_conditions(other),
        )
    }

    pub fn subtract(&self, other: &Self) -> Result<Self, RationalFunctionError> {
        let left = self.numerator.multiply(&other.denominator);
        let right = other.numerator.multiply(&self.denominator);
        Self::reduce(
            left.subtract(&right),
            self.denominator.multiply(&other.denominator),
            self.combined_conditions(other),
        )
    }

    pub fn multiply(&self, other: &Self) -> Result<Self, RationalFunctionError> {
        Self::reduce(
            self.numerator.multiply(&other.numerator),
            self.denominator.multiply(&other.denominator),
            self.combined_conditions(other),
        )
    }

    pub fn divide(&self, other: &Self) -> Result<Self, RationalFunctionError> {
        if other.numerator.is_zero() {
            return Err(RationalFunctionError::ZeroDivisor);
        }
        let mut exclusions = self.combined_conditions(other);
        exclusions.push(other.numerator.clone());
        Self::reduce(
            self.numerator.multiply(&other.denominator),
            self.denominator.multiply(&other.numerator),
            exclusions,
        )
    }

    /// Quotient rule without filling any source-domain holes.
    pub fn differentiate(&self) -> Result<Self, RationalFunctionError> {
        let n = self.numerator.differentiate().multiply(&self.denominator);
        let d = self.numerator.multiply(&self.denominator.differentiate());
        Self::reduce(
            n.subtract(&d),
            self.denominator.multiply(&self.denominator),
            self.excluded.clone(),
        )
    }

    pub fn evaluate(&self, at: &Rational) -> Result<Rational, RationalFunctionError> {
        if self
            .excluded
            .iter()
            .any(|condition| condition.evaluate(at).numerator().is_zero())
        {
            return Err(RationalFunctionError::UndefinedAtPoint);
        }
        let n = self.numerator.evaluate(at);
        let d = self.denominator.evaluate(at);
        if d.numerator().is_zero() {
            return Err(RationalFunctionError::UndefinedAtPoint);
        }
        Ok(rational_quotient(&n, &d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            values
                .iter()
                .map(|&v| Rational::from(Integer::from(v)))
                .collect(),
        )
    }

    fn at(x: i64) -> Rational {
        Rational::from(Integer::from(x))
    }

    #[test]
    fn canceled_holes_are_preserved() {
        let f = RationalFunction::new(p(&[-1, 0, 1]), p(&[-1, 1])).unwrap();
        assert_eq!(f.numerator(), &p(&[1, 1]));
        assert_eq!(f.denominator(), &p(&[1]));
        assert_eq!(f.excluded_zero_sets(), &[p(&[-1, 1])]);
        assert_eq!(
            f.evaluate(&at(1)),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        assert_eq!(f.evaluate(&at(2)), Ok(at(3)));
        let filled = RationalFunction::from_polynomial(p(&[1, 1]));
        assert_ne!(f, filled);
        assert!(f.algebraically_equivalent(&filled));
    }

    #[test]
    fn zero_keeps_source_domain_restrictions() {
        let f = RationalFunction::new(p(&[]), p(&[-2, 1])).unwrap();
        assert!(f.is_zero());
        assert_eq!(f.denominator(), &p(&[1]));
        assert_eq!(f.evaluate(&at(2)), Err(RationalFunctionError::UndefinedAtPoint));
    }

    #[test]
    fn denominator_is_monic_and_conditions_ignore_nonzero_scalars() {
        let f = RationalFunction::new(p(&[2, 2]), p(&[4, 4])).unwrap();
        let half = Rational::new(Integer::from(1), Integer::from(2)).unwrap();
        assert_eq!(f.numerator().coefficients(), &[half]);
        assert_eq!(f.denominator(), &p(&[1]));
        assert_eq!(f.excluded_zero_sets(), &[p(&[1, 1])]);
    }

    #[test]
    fn arithmetic_preserves_divisor_zeros_after_cancellation() {
        let a = RationalFunction::new(p(&[1]), p(&[0, 1])).unwrap();
        let b = RationalFunction::new(p(&[-1, 1]), p(&[0, 1])).unwrap();
        let quotient = a.divide(&b).unwrap();
        assert_eq!(quotient.numerator(), &p(&[1]));
        assert_eq!(quotient.denominator(), &p(&[-1, 1]));
        assert_eq!(quotient.excluded_zero_sets(), &[p(&[0, 1]), p(&[-1, 1])]);
        assert_eq!(
            quotient.evaluate(&at(0)),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        assert_eq!(
            quotient.evaluate(&at(1)),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
    }

    #[test]
    fn differentiating_canceled_one_does_not_fill_holes() {
        let f = RationalFunction::new(p(&[-1, 1]), p(&[-1, 1])).unwrap();
        let derivative = f.differentiate().unwrap();
        assert!(derivative.is_zero());
        assert_eq!(derivative.excluded_zero_sets(), &[p(&[-1, 1])]);
        assert_eq!(derivative.evaluate(&at(0)), Ok(at(0)));
        assert_eq!(
            derivative.evaluate(&at(1)),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
    }

    #[test]
    fn zero_denominators_and_division_by_zero_are_typed() {
        assert_eq!(
            RationalFunction::new(p(&[1]), p(&[])),
            Err(RationalFunctionError::ZeroDenominator)
        );
        let a = RationalFunction::from_polynomial(p(&[1, 1]));
        let zero = RationalFunction::from_polynomial(p(&[]));
        assert_eq!(a.divide(&zero), Err(RationalFunctionError::ZeroDivisor));
    }
}
