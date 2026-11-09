//! Exact coprime-pair Hensel lifting over Z[x].
//!
//! One step upgrades f = g*h (mod m) to a factor pair modulo m*p, where
//! m is a positive power of p and gcd(g mod p, h mod p) = 1.
//! Both construction and verification use exact integer coefficients.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::{
    EvaluationContext, EvaluationError, FiniteFieldError, FiniteFieldPolynomial,
    RationalPolynomial, is_small_prime,
};

const MAX_HENSEL_DEGREE: usize = 8;
const MAX_LIFT_STEPS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HenselError {
    InvalidPrime,
    InvalidModulus,
    InvalidDegree,
    NonMonic,
    NonCoprime,
    InitialCongruence,
    InvalidEvidence,
    FiniteField(FiniteFieldError),
    Evaluation(EvaluationError),
}

impl fmt::Display for HenselError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for HenselError {}
impl From<FiniteFieldError> for HenselError {
    fn from(value: FiniteFieldError) -> Self {
        Self::FiniteField(value)
    }
}
impl From<EvaluationError> for HenselError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

/// Integer coefficients in ascending degree order, with trailing zeros removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegerPolynomial {
    coefficients: Vec<Integer>,
}

impl IntegerPolynomial {
    pub fn new(mut coefficients: Vec<Integer>) -> Self {
        while coefficients.last().is_some_and(Integer::is_zero) {
            coefficients.pop();
        }
        Self { coefficients }
    }

    pub fn coefficients(&self) -> &[Integer] {
        &self.coefficients
    }
    pub fn degree(&self) -> Option<usize> {
        self.coefficients.len().checked_sub(1)
    }

    fn coefficient(&self, index: usize) -> Integer {
        self.coefficients
            .get(index)
            .cloned()
            .unwrap_or_else(Integer::zero)
    }

    fn add(&self, other: &Self) -> Self {
        Self::new(
            (0..self.coefficients.len().max(other.coefficients.len()))
                .map(|i| &self.coefficient(i) + &other.coefficient(i))
                .collect(),
        )
    }

    fn subtract(&self, other: &Self) -> Self {
        Self::new(
            (0..self.coefficients.len().max(other.coefficients.len()))
                .map(|i| &self.coefficient(i) - &other.coefficient(i))
                .collect(),
        )
    }

    fn scale(&self, scalar: &Integer) -> Self {
        Self::new(self.coefficients.iter().map(|x| x * scalar).collect())
    }

    fn multiply(&self, other: &Self) -> Self {
        if self.coefficients.is_empty() || other.coefficients.is_empty() {
            return Self::new(Vec::new());
        }
        let mut result =
            vec![Integer::zero(); self.coefficients.len() + other.coefficients.len() - 1];
        for (i, left) in self.coefficients.iter().enumerate() {
            for (j, right) in other.coefficients.iter().enumerate() {
                result[i + j] = &result[i + j] + &(left * right);
            }
        }
        Self::new(result)
    }

    fn divided_exact(&self, divisor: &Integer) -> Option<Self> {
        self.coefficients
            .iter()
            .map(|x| x.divide_exact(divisor))
            .collect::<Option<Vec<_>>>()
            .map(Self::new)
    }

    fn divisible_by(&self, divisor: &Integer) -> bool {
        self.coefficients
            .iter()
            .all(|x| x.divide_exact(divisor).is_some())
    }

    fn modulo(&self, prime: u32) -> Result<FiniteFieldPolynomial, HenselError> {
        let coefficients = self
            .coefficients
            .iter()
            .map(|value| value.modulo_u32(prime).map(i64::from))
            .collect::<Option<Vec<_>>>()
            .ok_or(HenselError::InvalidPrime)?;
        Ok(FiniteFieldPolynomial::new(prime, coefficients)?)
    }

    fn from_modular(value: &FiniteFieldPolynomial) -> Self {
        Self::new(
            value
                .coefficients()
                .iter()
                .map(|&x| Integer::from(x))
                .collect(),
        )
    }

    fn check(&self, ctx: &EvaluationContext) -> Result<(), HenselError> {
        ctx.check_polynomial(&RationalPolynomial::new(
            self.coefficients
                .iter()
                .cloned()
                .map(Rational::from)
                .collect(),
        ))?;
        Ok(())
    }

    fn is_monic(&self) -> bool {
        self.coefficients.last().is_some_and(Integer::is_one)
    }
}

fn power_of_prime(modulus: &Integer, prime: u32) -> bool {
    let factor = Integer::from(prime);
    let mut value = modulus.clone();
    for _ in 0..MAX_LIFT_STEPS {
        let Some(next) = value.divide_exact(&factor) else {
            return false;
        };
        value = next;
        if value.is_one() {
            return true;
        }
    }
    false
}

fn finite_one(prime: u32) -> FiniteFieldPolynomial {
    FiniteFieldPolynomial::new(prime, vec![1]).expect("validated prime")
}
fn finite_zero(prime: u32) -> FiniteFieldPolynomial {
    FiniteFieldPolynomial::new(prime, vec![]).expect("validated prime")
}

/// Computes s*a+t*b=1 over F_p[x], or rejects a noncoprime pair.
fn bezout(
    left: &FiniteFieldPolynomial,
    right: &FiniteFieldPolynomial,
) -> Result<(FiniteFieldPolynomial, FiniteFieldPolynomial), HenselError> {
    let prime = left.prime();
    let (mut a, mut b) = (left.clone(), right.clone());
    let (mut s0, mut s1) = (finite_one(prime), finite_zero(prime));
    let (mut t0, mut t1) = (finite_zero(prime), finite_one(prime));
    while !b.is_zero() {
        let (q, r) = a.divide_with_remainder(&b)?;
        let s2 = s0.subtract(&q.multiply(&s1)?)?;
        let t2 = t0.subtract(&q.multiply(&t1)?)?;
        (a, b) = (b, r);
        (s0, s1) = (s1, s2);
        (t0, t1) = (t1, t2);
    }
    if a.degree() != Some(0) {
        return Err(HenselError::NonCoprime);
    }
    let leading = a.coefficients()[0];
    let inverse = (1..prime)
        .find(|&v| v * leading % prime == 1)
        .ok_or(HenselError::NonCoprime)?;
    let s = s0.scale(inverse);
    let t = t0.scale(inverse);
    if s.multiply(left)?.add(&t.multiply(right)?)? != finite_one(prime) {
        return Err(HenselError::InvalidEvidence);
    }
    Ok((s, t))
}

/// Recorded exact one-step lift; all fields are verified by
/// `verify_hensel_pair` before this result is returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HenselPair {
    prime: u32,
    modulus_before: Integer,
    modulus_after: Integer,
    target: IntegerPolynomial,
    left_before: IntegerPolynomial,
    right_before: IntegerPolynomial,
    left_after: IntegerPolynomial,
    right_after: IntegerPolynomial,
}

impl HenselPair {
    pub fn prime(&self) -> u32 {
        self.prime
    }
    pub fn modulus_before(&self) -> &Integer {
        &self.modulus_before
    }
    pub fn modulus_after(&self) -> &Integer {
        &self.modulus_after
    }
    pub fn left(&self) -> &IntegerPolynomial {
        &self.left_after
    }
    pub fn right(&self) -> &IntegerPolynomial {
        &self.right_after
    }
    pub fn target(&self) -> &IntegerPolynomial {
        &self.target
    }
}

fn check_hensel_inputs(
    ctx: &EvaluationContext,
    target: &IntegerPolynomial,
    left: &IntegerPolynomial,
    right: &IntegerPolynomial,
    prime: u32,
    modulus: &Integer,
) -> Result<(), HenselError> {
    if !is_small_prime(prime) {
        return Err(HenselError::InvalidPrime);
    }
    if !power_of_prime(modulus, prime) {
        return Err(HenselError::InvalidModulus);
    }
    if target.degree().is_none_or(|d| d > MAX_HENSEL_DEGREE)
        || left.degree().is_none_or(|d| d == 0)
        || right.degree().is_none_or(|d| d == 0)
        || left.degree().unwrap_or(0) + right.degree().unwrap_or(0) != target.degree().unwrap_or(0)
    {
        return Err(HenselError::InvalidDegree);
    }
    if !target.is_monic() || !left.is_monic() || !right.is_monic() {
        return Err(HenselError::NonMonic);
    }
    for polynomial in [target, left, right] {
        polynomial.check(ctx)?;
    }
    if !target.subtract(&left.multiply(right)).divisible_by(modulus) {
        return Err(HenselError::InitialCongruence);
    }
    Ok(())
}

/// Verifies the original congruence, coprimality, degree, monicity, preservation
/// modulo m, and the strengthened congruence modulo m*p.
pub fn verify_hensel_pair(
    ctx: &EvaluationContext,
    result: &HenselPair,
) -> Result<bool, HenselError> {
    check_hensel_inputs(
        ctx,
        &result.target,
        &result.left_before,
        &result.right_before,
        result.prime,
        &result.modulus_before,
    )?;
    if result.modulus_after != &result.modulus_before * &Integer::from(result.prime)
        || !result.left_after.is_monic()
        || !result.right_after.is_monic()
        || result.left_before.degree() != result.left_after.degree()
        || result.right_before.degree() != result.right_after.degree()
    {
        return Ok(false);
    }
    for polynomial in [&result.left_after, &result.right_after] {
        polynomial.check(ctx)?;
    }
    if !result
        .left_after
        .subtract(&result.left_before)
        .divisible_by(&result.modulus_before)
        || !result
            .right_after
            .subtract(&result.right_before)
            .divisible_by(&result.modulus_before)
    {
        return Ok(false);
    }
    if bezout(
        &result.left_before.modulo(result.prime)?,
        &result.right_before.modulo(result.prime)?,
    )
    .is_err()
    {
        return Ok(false);
    }
    Ok(result
        .target
        .subtract(&result.left_after.multiply(&result.right_after))
        .divisible_by(&result.modulus_after))
}

/// Performs one exact Hensel step from modulus p^k to p^(k+1).
/// Requires monic integer factor candidates whose reductions are coprime.
pub fn hensel_lift_pair(
    ctx: &EvaluationContext,
    target: &IntegerPolynomial,
    left: &IntegerPolynomial,
    right: &IntegerPolynomial,
    prime: u32,
    modulus: &Integer,
) -> Result<HenselPair, HenselError> {
    check_hensel_inputs(ctx, target, left, right, prime, modulus)?;
    let g = left.modulo(prime)?;
    let h = right.modulo(prime)?;
    let (s, t) = bezout(&g, &h)?;
    let error = target
        .subtract(&left.multiply(right))
        .divided_exact(modulus)
        .ok_or(HenselError::InitialCongruence)?;
    let e = error.modulo(prime)?;
    let delta_g = e.multiply(&t)?.divide_with_remainder(&g)?.1;
    let delta_h = e.multiply(&s)?.divide_with_remainder(&h)?.1;
    let lifted_left = left.add(&IntegerPolynomial::from_modular(&delta_g).scale(modulus));
    let lifted_right = right.add(&IntegerPolynomial::from_modular(&delta_h).scale(modulus));
    let result = HenselPair {
        prime,
        modulus_before: modulus.clone(),
        modulus_after: modulus * &Integer::from(prime),
        target: target.clone(),
        left_before: left.clone(),
        right_before: right.clone(),
        left_after: lifted_left,
        right_after: lifted_right,
    };
    if !verify_hensel_pair(ctx, &result)? {
        return Err(HenselError::InvalidEvidence);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(values: &[i64]) -> IntegerPolynomial {
        IntegerPolynomial::new(values.iter().map(|&v| Integer::from(v)).collect())
    }

    #[test]
    fn lifts_coprime_quadratic_pair_twice() {
        let ctx = EvaluationContext::default();
        let f = p(&[4, 0, 0, 0, 1]);
        let g = p(&[2, 1, 1]);
        let h = p(&[2, 2, 1]);
        let first = hensel_lift_pair(&ctx, &f, &g, &h, 3, &Integer::from(3)).unwrap();
        assert!(verify_hensel_pair(&ctx, &first).unwrap());
        assert_eq!(first.modulus_after(), &Integer::from(9));
        let second = hensel_lift_pair(
            &ctx,
            &f,
            first.left(),
            first.right(),
            3,
            first.modulus_after(),
        )
        .unwrap();
        assert_eq!(second.modulus_after(), &Integer::from(27));
        assert!(verify_hensel_pair(&ctx, &second).unwrap());
    }

    #[test]
    fn forged_lift_and_noncoprime_pairs_are_rejected() {
        let ctx = EvaluationContext::default();
        let f = p(&[4, 0, 0, 0, 1]);
        let first = hensel_lift_pair(
            &ctx,
            &f,
            &p(&[2, 1, 1]),
            &p(&[2, 2, 1]),
            3,
            &Integer::from(3),
        )
        .unwrap();
        let mut forged = first.clone();
        forged.left_after = forged.left_after.add(&p(&[3]));
        assert!(!verify_hensel_pair(&ctx, &forged).unwrap());

        let repeated = p(&[1, 0, 1]);
        assert_eq!(
            hensel_lift_pair(
                &ctx,
                &repeated.multiply(&repeated),
                &repeated,
                &repeated,
                3,
                &Integer::from(3)
            ),
            Err(HenselError::NonCoprime),
        );
    }

    #[test]
    fn invalid_prime_power_and_wrong_initial_congruence_are_rejected() {
        let ctx = EvaluationContext::default();
        let f = p(&[4, 0, 0, 0, 1]);
        let g = p(&[2, 1, 1]);
        let h = p(&[2, 2, 1]);
        assert_eq!(
            hensel_lift_pair(&ctx, &f, &g, &h, 4, &Integer::from(4)),
            Err(HenselError::InvalidPrime),
        );
        assert_eq!(
            hensel_lift_pair(&ctx, &f, &g, &h, 3, &Integer::from(6)),
            Err(HenselError::InvalidModulus),
        );
        assert_eq!(
            hensel_lift_pair(&ctx, &f, &g, &p(&[1, 2, 1]), 3, &Integer::from(3)),
            Err(HenselError::InitialCongruence),
        );
    }
}
