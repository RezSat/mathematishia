//! Exact polynomial arithmetic over a bounded prime field F_p.
//!
//! The modulus is checked for primality before values are constructed.
//! Arithmetic is integer-only, with canonical coefficients in [0, p).

use std::error::Error;
use std::fmt;

const MAX_PRIME: u32 = 251;
const MAX_DEGREE: usize = 32;
const MAX_INTERMEDIATE_DEGREE: usize = 2 * MAX_DEGREE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiniteFieldError {
    InvalidPrime,
    DegreeLimit,
    DifferentFields,
    ZeroDivisor,
}

impl fmt::Display for FiniteFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrime => f.write_str("modulus must be a prime no greater than 251"),
            Self::DegreeLimit => f.write_str("finite-field polynomial degree limit exceeded"),
            Self::DifferentFields => f.write_str("finite-field polynomials have different primes"),
            Self::ZeroDivisor => f.write_str("cannot divide by the zero polynomial"),
        }
    }
}
impl Error for FiniteFieldError {}

pub fn is_small_prime(value: u32) -> bool {
    if !(2..=MAX_PRIME).contains(&value) {
        return false;
    }
    let mut d = 2;
    while d <= value / d {
        if value.is_multiple_of(d) {
            return false;
        }
        d += 1;
    }
    true
}

fn trim(coefficients: &mut Vec<u32>) {
    while coefficients.last() == Some(&0) {
        coefficients.pop();
    }
}

fn inv_mod(value: u32, prime: u32) -> u32 {
    // Fermat's little theorem. The input is a nonzero element of F_p.
    let mut n = prime - 2;
    let mut factor = value as u64;
    let mut output = 1_u64;
    let modulus = u64::from(prime);
    while n > 0 {
        if n & 1 == 1 {
            output = output * factor % modulus;
        }
        n >>= 1;
        factor = factor * factor % modulus;
    }
    output as u32
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteFieldPolynomial {
    prime: u32,
    coefficients: Vec<u32>,
}

impl FiniteFieldPolynomial {
    pub fn new(prime: u32, coefficients: Vec<i64>) -> Result<Self, FiniteFieldError> {
        if !is_small_prime(prime) {
            return Err(FiniteFieldError::InvalidPrime);
        }
        if coefficients.len().saturating_sub(1) > MAX_DEGREE {
            return Err(FiniteFieldError::DegreeLimit);
        }
        let modulus = i64::from(prime);
        let mut coefficients = coefficients
            .into_iter()
            .map(|value| value.rem_euclid(modulus) as u32)
            .collect::<Vec<_>>();
        trim(&mut coefficients);
        Ok(Self {
            prime,
            coefficients,
        })
    }

    fn from_canonical(prime: u32, mut coefficients: Vec<u32>) -> Self {
        trim(&mut coefficients);
        Self {
            prime,
            coefficients,
        }
    }

    fn zero(prime: u32) -> Self {
        Self {
            prime,
            coefficients: Vec::new(),
        }
    }

    fn one(prime: u32) -> Self {
        Self {
            prime,
            coefficients: vec![1],
        }
    }

    fn x(prime: u32) -> Self {
        Self {
            prime,
            coefficients: vec![0, 1],
        }
    }

    pub fn prime(&self) -> u32 {
        self.prime
    }

    pub fn coefficients(&self) -> &[u32] {
        &self.coefficients
    }

    pub fn degree(&self) -> Option<usize> {
        self.coefficients.len().checked_sub(1)
    }

    pub fn is_zero(&self) -> bool {
        self.coefficients.is_empty()
    }

    fn compatible(&self, other: &Self) -> Result<(), FiniteFieldError> {
        if self.prime != other.prime {
            Err(FiniteFieldError::DifferentFields)
        } else {
            Ok(())
        }
    }

    pub fn add(&self, other: &Self) -> Result<Self, FiniteFieldError> {
        self.compatible(other)?;
        let mut result = vec![0; self.coefficients.len().max(other.coefficients.len())];
        for (index, coefficient) in result.iter_mut().enumerate() {
            *coefficient = (self.coefficients.get(index).copied().unwrap_or(0)
                + other.coefficients.get(index).copied().unwrap_or(0))
                % self.prime;
        }
        Ok(Self::from_canonical(self.prime, result))
    }

    pub fn subtract(&self, other: &Self) -> Result<Self, FiniteFieldError> {
        self.compatible(other)?;
        let mut result = vec![0; self.coefficients.len().max(other.coefficients.len())];
        for (index, coefficient) in result.iter_mut().enumerate() {
            *coefficient = (self.coefficients.get(index).copied().unwrap_or(0) + self.prime
                - other.coefficients.get(index).copied().unwrap_or(0))
                % self.prime;
        }
        Ok(Self::from_canonical(self.prime, result))
    }

    pub fn multiply(&self, other: &Self) -> Result<Self, FiniteFieldError> {
        self.compatible(other)?;
        if self.is_zero() || other.is_zero() {
            return Ok(Self::zero(self.prime));
        }
        let degree = self.degree().unwrap_or(0) + other.degree().unwrap_or(0);
        if degree > MAX_INTERMEDIATE_DEGREE {
            return Err(FiniteFieldError::DegreeLimit);
        }
        let mut result = vec![0_u32; degree + 1];
        for (i, &left) in self.coefficients.iter().enumerate() {
            for (j, &right) in other.coefficients.iter().enumerate() {
                result[i + j] = (result[i + j] + left * right) % self.prime;
            }
        }
        Ok(Self::from_canonical(self.prime, result))
    }

    pub fn scale(&self, coefficient: u32) -> Self {
        Self::from_canonical(
            self.prime,
            self.coefficients
                .iter()
                .map(|&value| {
                    ((value as u64 * u64::from(coefficient % self.prime)) % u64::from(self.prime))
                        as u32
                })
                .collect(),
        )
    }

    pub fn monic(&self) -> Self {
        match self.coefficients.last() {
            Some(&leading) => self.scale(inv_mod(leading, self.prime)),
            None => self.clone(),
        }
    }

    pub fn divide_with_remainder(&self, divisor: &Self) -> Result<(Self, Self), FiniteFieldError> {
        self.compatible(divisor)?;
        if divisor.is_zero() {
            return Err(FiniteFieldError::ZeroDivisor);
        }
        let Some(divisor_degree) = divisor.degree() else {
            return Err(FiniteFieldError::ZeroDivisor);
        };
        let Some(mut remainder_degree) = self.degree() else {
            return Ok((Self::zero(self.prime), self.clone()));
        };
        if remainder_degree < divisor_degree {
            return Ok((Self::zero(self.prime), self.clone()));
        }
        let inverse = inv_mod(*divisor.coefficients.last().unwrap_or(&1), self.prime);
        let mut remainder = self.coefficients.clone();
        let mut quotient = vec![0; remainder_degree - divisor_degree + 1];
        while !remainder.is_empty() && remainder_degree >= divisor_degree {
            let shift = remainder_degree - divisor_degree;
            let coefficient = remainder[remainder_degree] * inverse % self.prime;
            quotient[shift] = coefficient;
            for (index, &value) in divisor.coefficients.iter().enumerate() {
                let position = index + shift;
                remainder[position] = (remainder[position] + self.prime
                    - coefficient * value % self.prime)
                    % self.prime;
            }
            trim(&mut remainder);
            remainder_degree = remainder.len().saturating_sub(1);
        }
        Ok((
            Self::from_canonical(self.prime, quotient),
            Self::from_canonical(self.prime, remainder),
        ))
    }

    pub fn gcd(&self, other: &Self) -> Result<Self, FiniteFieldError> {
        self.compatible(other)?;
        let mut a = self.clone();
        let mut b = other.clone();
        while !b.is_zero() {
            let (_, remainder) = a.divide_with_remainder(&b)?;
            a = b;
            b = remainder;
        }
        Ok(a.monic())
    }

    /// Computes a^exponent mod modulus, reducing after every multiplication.
    pub fn pow_mod(&self, mut exponent: u64, modulus: &Self) -> Result<Self, FiniteFieldError> {
        self.compatible(modulus)?;
        if modulus.is_zero() {
            return Err(FiniteFieldError::ZeroDivisor);
        }
        let (_, mut factor) = self.divide_with_remainder(modulus)?;
        let mut result = Self::one(self.prime);
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = result.multiply(&factor)?.divide_with_remainder(modulus)?.1;
            }
            exponent >>= 1;
            if exponent > 0 {
                factor = factor.multiply(&factor)?.divide_with_remainder(modulus)?.1;
            }
        }
        Ok(result.divide_with_remainder(modulus)?.1)
    }

    pub fn derivative(&self) -> Self {
        Self::from_canonical(
            self.prime,
            self.coefficients
                .iter()
                .enumerate()
                .skip(1)
                .map(|(index, coefficient)| {
                    ((index as u64 * u64::from(*coefficient)) % u64::from(self.prime)) as u32
                })
                .collect(),
        )
    }

    /// Rabin's exact irreducibility criterion using Frobenius powers.
    pub fn is_irreducible(&self) -> Result<bool, FiniteFieldError> {
        let Some(degree) = self.degree() else {
            return Ok(false);
        };
        if degree == 0 {
            return Ok(false);
        }
        let polynomial = self.monic();
        let x = Self::x(self.prime);
        let (_, x_mod) = x.divide_with_remainder(&polynomial)?;
        let mut power = x_mod.clone();
        for n in 1..=degree {
            power = power.pow_mod(u64::from(self.prime), &polynomial)?;
            if degree.is_multiple_of(n)
                && degree / n > 1
                && is_prime_degree(degree / n)
                && polynomial.gcd(&power.subtract(&x_mod)?)?.degree() != Some(0)
            {
                return Ok(false);
            }
        }
        Ok(power == x_mod)
    }
}

fn is_prime_degree(n: usize) -> bool {
    if n < 2 {
        return false;
    }
    (2..)
        .take_while(|d| d * d <= n)
        .all(|d| !n.is_multiple_of(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primality_and_canonical_coefficients_are_enforced() {
        for composite in [0, 1, 4, 9, 251 + 2] {
            assert_eq!(
                FiniteFieldPolynomial::new(composite, vec![1]),
                Err(FiniteFieldError::InvalidPrime)
            );
        }
        assert_eq!(
            FiniteFieldPolynomial::new(5, vec![-1, 6, 5])
                .unwrap()
                .coefficients(),
            &[4, 1]
        );
    }

    #[test]
    fn degree_limit_supports_modular_products_of_degree_32() {
        let polynomial = FiniteFieldPolynomial::new(2, vec![1; 33]).unwrap();
        let square = polynomial.multiply(&polynomial).unwrap();
        assert_eq!(square.degree(), Some(64));
        assert!(square.divide_with_remainder(&polynomial).unwrap().1.is_zero());
        assert!(FiniteFieldPolynomial::new(2, vec![1; 34]).is_err());
    }

    #[test]
    fn division_gcd_and_frobenius_are_exact() {
        let x2 = FiniteFieldPolynomial::new(2, vec![1, 1, 1]).unwrap();
        let x = FiniteFieldPolynomial::new(2, vec![0, 1]).unwrap();
        assert!(x2.is_irreducible().unwrap());
        assert_eq!(x.pow_mod(4, &x2).unwrap(), x);
        let (quotient, remainder) = x2.multiply(&x).unwrap().divide_with_remainder(&x2).unwrap();
        assert_eq!(quotient, x);
        assert!(remainder.is_zero());
        assert_eq!(x2.gcd(&x).unwrap().degree(), Some(0));
    }
}
