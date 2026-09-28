use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use num_bigint::Sign;
use num_integer::Integer as _;

use super::Integer;

/// A rational constructor received a zero denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZeroDenominator;

impl fmt::Display for ZeroDenominator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a rational denominator must be nonzero")
    }
}

impl std::error::Error for ZeroDenominator {}

/// An exact rational in lowest terms with a positive denominator.
///
/// Zero is always stored as `0/1`. Equality and hashing use this canonical
/// representation; ordering is numerical. Arithmetic accepts two owned values
/// or two borrowed values. `Display` is not a stable serialization format.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rational {
    numerator: Integer,
    denominator: Integer,
}

impl Rational {
    /// Reduces the fraction and normalizes its sign.
    ///
    /// Returns [`ZeroDenominator`] if `denominator` is zero.
    ///
    /// ```
    /// use mathema::{Integer, Rational, ZeroDenominator};
    ///
    /// let half = Rational::new(Integer::from(2), Integer::from(4))?;
    /// assert_eq!(half.to_string(), "1/2");
    /// assert_eq!((&half + &half).to_string(), "1");
    /// assert_eq!(Rational::new(Integer::one(), Integer::zero()), Err(ZeroDenominator));
    /// # Ok::<(), ZeroDenominator>(())
    /// ```
    pub fn new(numerator: Integer, denominator: Integer) -> Result<Self, ZeroDenominator> {
        if denominator.is_zero() {
            return Err(ZeroDenominator);
        }
        Ok(Self::from_nonzero_denominator(numerator, denominator))
    }

    // Callers have either checked the denominator or formed a product of
    // positive canonical denominators, so the gcd is nonzero. These divisions
    // are exact internal reductions, not public integer division semantics.
    fn from_nonzero_denominator(mut numerator: Integer, mut denominator: Integer) -> Self {
        let divisor = numerator.0.gcd(&denominator.0);
        numerator.0 /= &divisor;
        denominator.0 /= divisor;
        if denominator.0.sign() == Sign::Minus {
            numerator = -numerator;
            denominator = -denominator;
        }
        Self {
            numerator,
            denominator,
        }
    }

    pub fn numerator(&self) -> &Integer {
        &self.numerator
    }

    pub fn denominator(&self) -> &Integer {
        &self.denominator
    }
}

impl From<Integer> for Rational {
    fn from(numerator: Integer) -> Self {
        Self {
            numerator,
            denominator: Integer::one(),
        }
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        // Positive denominators preserve the order under cross multiplication.
        (&self.numerator * &other.denominator).cmp(&(&other.numerator * &self.denominator))
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator.is_one() {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

impl Neg for Rational {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            numerator: -self.numerator,
            denominator: self.denominator,
        }
    }
}

impl Neg for &Rational {
    type Output = Rational;

    fn neg(self) -> Rational {
        Rational {
            numerator: -&self.numerator,
            denominator: self.denominator.clone(),
        }
    }
}

impl Add for Rational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        &self + &rhs
    }
}

impl Add for &Rational {
    type Output = Rational;

    fn add(self, rhs: Self) -> Rational {
        Rational::from_nonzero_denominator(
            &self.numerator * &rhs.denominator + &rhs.numerator * &self.denominator,
            &self.denominator * &rhs.denominator,
        )
    }
}

impl Sub for Rational {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        &self - &rhs
    }
}

impl Sub for &Rational {
    type Output = Rational;

    fn sub(self, rhs: Self) -> Rational {
        Rational::from_nonzero_denominator(
            &self.numerator * &rhs.denominator - &rhs.numerator * &self.denominator,
            &self.denominator * &rhs.denominator,
        )
    }
}

impl Mul for Rational {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        &self * &rhs
    }
}

impl Mul for &Rational {
    type Output = Rational;

    fn mul(self, rhs: Self) -> Rational {
        Rational::from_nonzero_denominator(
            &self.numerator * &rhs.numerator,
            &self.denominator * &rhs.denominator,
        )
    }
}
