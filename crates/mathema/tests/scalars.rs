use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use mathema::{Integer, Rational, ZeroDenominator};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed};

fn rational(numerator: i128, denominator: i128) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// Property inputs and their arithmetic fit in i128. Check canonicality with an
// independent primitive Euclidean algorithm through only the public API.
fn assert_canonical(value: &Rational) {
    let numerator: i128 = value.numerator().to_string().parse().unwrap();
    let denominator: i128 = value.denominator().to_string().parse().unwrap();
    assert!(denominator > 0);
    let (mut a, mut b) = (numerator.unsigned_abs(), denominator as u128);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    assert_eq!(a, 1);
    if numerator == 0 {
        assert_eq!(denominator, 1);
    }
}

#[test]
fn integer_primitive_construction() {
    let signed = [
        Integer::from(-7i8),
        Integer::from(-7i16),
        Integer::from(-7i32),
        Integer::from(-7i64),
        Integer::from(-7i128),
        Integer::from(-7isize),
    ];
    let unsigned = [
        Integer::from(7u8),
        Integer::from(7u16),
        Integer::from(7u32),
        Integer::from(7u64),
        Integer::from(7u128),
        Integer::from(7usize),
    ];
    for value in signed {
        assert_eq!(value, Integer::from(-7));
        assert_eq!(value.to_string(), "-7");
    }
    for value in unsigned {
        assert_eq!(value, Integer::from(7));
        assert_eq!(value.to_string(), "7");
    }
    assert_eq!(Integer::from(i128::MIN).to_string(), i128::MIN.to_string());
    assert_eq!(Integer::from(u128::MAX).to_string(), u128::MAX.to_string());
}

#[test]
fn integer_identity_order_and_hash() {
    assert!(Integer::zero().is_zero());
    assert!(!Integer::one().is_zero());
    assert!(Integer::one().is_one());
    assert!(!Integer::zero().is_one());
    assert!(!Integer::from(-1).is_one());
    assert_eq!(Integer::zero().to_string(), "0");
    assert!(Integer::from(-2) < Integer::from(-1));
    assert!(Integer::from(-1) < Integer::zero());
    assert!(Integer::zero() < Integer::one());
    let value = Integer::from(7);
    assert_eq!(value.clone(), value);
    assert_eq!(hash(&value), hash(&Integer::from(7u64)));
}

#[test]
fn integer_arithmetic() {
    let a = Integer::from(-7);
    let b = Integer::from(3);
    assert_eq!(&a + &b, Integer::from(-4));
    assert_eq!(a.clone() + b.clone(), Integer::from(-4));
    assert_eq!(&a - &b, Integer::from(-10));
    assert_eq!(a.clone() - b.clone(), Integer::from(-10));
    assert_eq!(&a * &b, Integer::from(-21));
    assert_eq!(a.clone() * b, Integer::from(-21));
    assert_eq!(-&a, Integer::from(7));
    assert_eq!(-a, Integer::from(7));
    assert_eq!(-Integer::zero(), Integer::zero());
}

#[test]
fn integers_exceed_primitive_widths() {
    let large = Integer::from(u128::MAX) + Integer::one();
    assert_eq!(large.to_string(), "340282366920938463463374607431768211456");
    assert!(large > Integer::from(u128::MAX));
    assert_eq!(
        (&large * &large).to_string(),
        "115792089237316195423570985008687907853269984665640564039457584007913129639936"
    );
    assert_eq!(&large - &Integer::one(), Integer::from(u128::MAX));
    assert_eq!(&large + &(-&large), Integer::zero());
}

#[test]
fn rational_construction_and_signs() {
    for (n, d, expected_n, expected_d) in [
        (3, 7, 3, 7),
        (2, 4, 1, 2),
        (-2, 4, -1, 2),
        (2, -4, -1, 2),
        (-2, -4, 1, 2),
        (0, 37, 0, 1),
        (0, -37, 0, 1),
        (0, 1, 0, 1),
        (6, 3, 2, 1),
    ] {
        let value = rational(n, d);
        assert_eq!(value.numerator(), &Integer::from(expected_n));
        assert_eq!(value.denominator(), &Integer::from(expected_d));
        assert_canonical(&value);
    }
}

#[test]
fn rational_rejects_zero_denominators() {
    for numerator in [-1, 0, 1, i128::MIN, i128::MAX] {
        assert_eq!(
            Rational::new(Integer::from(numerator), Integer::zero()),
            Err(ZeroDenominator)
        );
    }
    let error: &dyn std::error::Error = &ZeroDenominator;
    assert_eq!(error.to_string(), "a rational denominator must be nonzero");
}

#[test]
fn rational_handles_large_values_and_primitive_minimum() {
    assert_eq!(rational(i128::MIN, i128::MIN), rational(1, 1));
    assert_eq!(
        rational(1, i128::MIN).denominator().to_string(),
        "170141183460469231731687303715884105728"
    );
    let large = Integer::from(u128::MAX) + Integer::one();
    let reduced = Rational::new(&large * &Integer::from(3), &large * &Integer::from(7)).unwrap();
    assert_eq!(reduced, rational(3, 7));
    let whole = Rational::from(large.clone());
    let larger = Rational::from(&large + &Integer::one());
    assert!(whole < larger);
    assert_eq!(&larger - &whole, rational(1, 1));
    assert_eq!(&whole * &whole, Rational::from(&large * &large));
}

#[test]
fn rational_integer_conversion() {
    for n in [-5, 0, 5] {
        let value = Rational::from(Integer::from(n));
        assert_eq!(value, rational(n, 1));
        assert_canonical(&value);
    }
}

#[test]
fn rational_arithmetic() {
    let a = rational(1, 6);
    let b = rational(-1, 4);
    assert_eq!(&a + &b, rational(-1, 12));
    assert_eq!(a.clone() + b.clone(), rational(-1, 12));
    assert_eq!(&a - &b, rational(5, 12));
    assert_eq!(a.clone() - b.clone(), rational(5, 12));
    assert_eq!(&a * &b, rational(-1, 24));
    assert_eq!(a.clone() * b.clone(), rational(-1, 24));
    assert_eq!(-&b, rational(1, 4));
    assert_eq!(-b, rational(1, 4));
    assert_eq!(&a - &a, rational(0, 1));
    assert_eq!(rational(1, 6) + rational(1, 3), rational(1, 2));
    assert_eq!(rational(2, 3) * rational(9, 4), rational(3, 2));
    assert_eq!(rational(0, 7) * a, rational(0, 1));
    assert_eq!(-rational(0, -7), rational(0, 1));
}

#[test]
fn rational_equality_order_and_hash() {
    let half = rational(1, 2);
    for value in [rational(2, 4), rational(-2, -4), half.clone()] {
        assert_eq!(value, half);
        assert_eq!(value.cmp(&half), std::cmp::Ordering::Equal);
        assert_eq!(hash(&value), hash(&half));
    }
    assert_eq!(hash(&rational(0, -37)), hash(&rational(0, 1)));
    let ordered = [
        rational(-3, 1),
        rational(-7, 3),
        rational(-1, 2),
        rational(0, 1),
        rational(1, 3),
        rational(1, 2),
        rational(1, 1),
        rational(3, 2),
        rational(2, 1),
    ];
    for pair in ordered.windows(2) {
        assert!(pair[0] < pair[1]);
        assert_eq!(
            pair[0].partial_cmp(&pair[1]),
            Some(std::cmp::Ordering::Less)
        );
    }
}

#[test]
fn rational_display() {
    for (n, d, expected) in [(10, 2, "5"), (3, 7, "3/7"), (3, -7, "-3/7"), (0, -37, "0")] {
        assert_eq!(rational(n, d).to_string(), expected);
    }
}

fn nonzero() -> impl Strategy<Value = i32> {
    any::<i32>().prop_filter("denominator or scale must be nonzero", |value| *value != 0)
}

proptest! {
    #![proptest_config(Config {
        cases: 128,
        rng_seed: RngSeed::Fixed(0x4d415448454d41),
        rng_algorithm: RngAlgorithm::ChaCha,
        ..Config::default()
    })]

    #[test]
    fn accepted_rationals_are_canonical(n in any::<i32>(), d in nonzero()) {
        let value = rational(i128::from(n), i128::from(d));
        assert_canonical(&value);
        let stored_n: i128 = value.numerator().to_string().parse().unwrap();
        let stored_d: i128 = value.denominator().to_string().parse().unwrap();
        prop_assert_eq!(stored_n * i128::from(d), i128::from(n) * stored_d);
    }

    #[test]
    fn zero_has_unique_representation(d in nonzero()) {
        let value = rational(0, i128::from(d));
        prop_assert_eq!(value.numerator(), &Integer::zero());
        prop_assert_eq!(value.denominator(), &Integer::one());
    }

    #[test]
    fn signs_normalize(n in any::<i32>(), d in nonzero()) {
        let (n, d) = (i128::from(n), i128::from(d));
        prop_assert_eq!(rational(n, d), rational(-n, -d));
    }

    #[test]
    fn scaling_preserves_value(n in any::<i32>(), d in nonzero(), k in nonzero()) {
        let (n, d, k) = (i128::from(n), i128::from(d), i128::from(k));
        let original = rational(n, d);
        let scaled = rational(n * k, d * k);
        prop_assert_eq!(hash(&original), hash(&scaled));
        prop_assert_eq!(original, scaled);
    }

    #[test]
    fn arithmetic_is_exact_and_canonical(
        n in any::<i32>(), d in nonzero(), m in any::<i32>(), e in nonzero()
    ) {
        let (n, d, m, e) = (i128::from(n), i128::from(d), i128::from(m), i128::from(e));
        let a = rational(n, d);
        let b = rational(m, e);
        for (actual, expected) in [
            (&a + &b, rational(n * e + m * d, d * e)),
            (&a - &b, rational(n * e - m * d, d * e)),
            (&a * &b, rational(n * m, d * e)),
            (-&a, rational(-n, d)),
        ] {
            assert_canonical(&actual);
            prop_assert_eq!(actual, expected);
        }
        prop_assert_eq!((&a + &b) - b, a);
    }

    #[test]
    fn ordering_agrees_with_exact_cross_products(
        n in any::<i32>(), d in nonzero(), m in any::<i32>(), e in nonzero()
    ) {
        let (n, d, m, e) = (i128::from(n), i128::from(d), i128::from(m), i128::from(e));
        let a = rational(n, d);
        let b = rational(m, e);
        let expected = (n * e).cmp(&(m * d));
        let expected = if d * e < 0 { expected.reverse() } else { expected };
        prop_assert_eq!(a.cmp(&b), expected);
        prop_assert_eq!(a == b, expected == std::cmp::Ordering::Equal);
    }
}
