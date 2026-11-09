use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::str::FromStr;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;

/// An exact, arbitrary-precision integer.
///
/// Arithmetic accepts two owned values or two borrowed values. Integer division
/// is intentionally absent until its semantics are defined.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Integer(pub(super) BigInt);

impl Integer {
    pub fn zero() -> Self {
        Self(BigInt::ZERO)
    }

    pub fn one() -> Self {
        Self(BigInt::from(1u8))
    }

    pub fn is_zero(&self) -> bool {
        self.0.sign() == Sign::NoSign
    }

    pub fn is_one(&self) -> bool {
        self.0 == BigInt::from(1u8)
    }

    /// Magnitude bit length, excluding the sign (zero has length zero).
    pub fn bit_len(&self) -> u64 {
        self.0.bits()
    }

    /// Nonnegative greatest common divisor, including gcd(0,0)=0.
    pub fn gcd(&self, other: &Self) -> Self {
        Self(self.0.gcd(&other.0))
    }

    /// Nonnegative least common multiple; zero if either operand is zero.
    pub fn lcm(&self, other: &Self) -> Self {
        Self(self.0.lcm(&other.0))
    }

    /// Exact integer quotient. Returns None on zero divisors or a remainder.
    pub fn divide_exact(&self, divisor: &Self) -> Option<Self> {
        if divisor.is_zero() {
            return None;
        }
        let (quotient, remainder) = self.0.div_rem(&divisor.0);
        (remainder.sign() == Sign::NoSign).then_some(Self(quotient))
    }

    /// Narrowing conversion for bounded algorithms; never truncates.
    pub fn try_i64(&self) -> Option<i64> {
        self.to_string().parse::<i64>().ok()
    }

    /// Returns the nonnegative exact integer square root, if one exists.
    /// A negative integer has no real integer square root.
    pub fn exact_sqrt(&self) -> Option<Self> {
        if self.0.sign() == Sign::Minus {
            return None;
        }
        let root = self.0.sqrt();
        if &root * &root == self.0 {
            Some(Self(root))
        } else {
            None
        }
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<i8> for Integer {
    fn from(value: i8) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<i16> for Integer {
    fn from(value: i16) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<i32> for Integer {
    fn from(value: i32) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<i128> for Integer {
    fn from(value: i128) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<isize> for Integer {
    fn from(value: isize) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<u8> for Integer {
    fn from(value: u8) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<u16> for Integer {
    fn from(value: u16) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<u32> for Integer {
    fn from(value: u32) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<u128> for Integer {
    fn from(value: u128) -> Self {
        Self(BigInt::from(value))
    }
}

impl From<usize> for Integer {
    fn from(value: usize) -> Self {
        Self(BigInt::from(value))
    }
}

impl Neg for Integer {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Neg for &Integer {
    type Output = Integer;

    fn neg(self) -> Integer {
        Integer(-&self.0)
    }
}

impl Add for Integer {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Add for &Integer {
    type Output = Integer;

    fn add(self, rhs: Self) -> Integer {
        Integer(&self.0 + &rhs.0)
    }
}

impl Sub for Integer {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Sub for &Integer {
    type Output = Integer;

    fn sub(self, rhs: Self) -> Integer {
        Integer(&self.0 - &rhs.0)
    }
}

impl Mul for Integer {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0)
    }
}

impl Mul for &Integer {
    type Output = Integer;

    fn mul(self, rhs: Self) -> Integer {
        Integer(&self.0 * &rhs.0)
    }
}

/// Parses an arbitrary-precision decimal integer without using a fixed-width intermediate.
impl FromStr for Integer {
    type Err = num_bigint::ParseBigIntError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse::<BigInt>().map(Self)
    }
}

#[cfg(test)]
mod arithmetic_helpers_tests {
    use super::Integer;

    #[test]
    fn exact_division_and_sign_invariants() {
        let a = Integer::from(-12);
        let b = Integer::from(18);
        assert_eq!(a.gcd(&b), Integer::from(6));
        assert_eq!(a.lcm(&b), Integer::from(36));
        assert_eq!(a.divide_exact(&Integer::from(3)), Some(Integer::from(-4)));
        assert_eq!(a.divide_exact(&Integer::from(5)), None);
        assert_eq!(a.divide_exact(&Integer::zero()), None);
        assert_eq!(Integer::zero().lcm(&a), Integer::zero());
        assert_eq!(Integer::zero().gcd(&a), Integer::from(12));
    }

    #[test]
    fn narrowing_does_not_truncate_big_integers() {
        let huge = "340282366920938463463374607431768211456"
            .parse::<Integer>()
            .unwrap();
        assert_eq!(huge.try_i64(), None);
        assert_eq!(Integer::from(-123).try_i64(), Some(-123));
    }
}
