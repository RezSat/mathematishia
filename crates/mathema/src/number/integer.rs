use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

use num_bigint::{BigInt, Sign};

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
