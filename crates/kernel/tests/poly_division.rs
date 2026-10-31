use mathema::{Integer, Rational};
use mathematishia_kernel::{RationalPolynomial, ZeroPolynomialDivisor};

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn p(terms: &[(i64, i64)]) -> RationalPolynomial {
    RationalPolynomial::new(terms.iter().map(|&(n, d)| q(n, d)).collect())
}

fn assert_division_invariants(dividend: &RationalPolynomial, divisor: &RationalPolynomial) {
    let (quotient, remainder) = dividend.divide_with_remainder(divisor).unwrap();
    assert_eq!(divisor.multiply(&quotient).add(&remainder), *dividend);
    assert!(remainder.is_zero() || remainder.degree() < divisor.degree());
}

#[test]
fn rejects_zero_divisor() {
    let zero = RationalPolynomial::zero();
    let x = p(&[(0, 1), (1, 1)]);
    assert_eq!(x.divide_with_remainder(&zero), Err(ZeroPolynomialDivisor));
    assert_eq!(
        zero.divide_with_remainder(&zero),
        Err(ZeroPolynomialDivisor)
    );
}

#[test]
fn divides_exactly_and_returns_zero_remainder() {
    let dividend = p(&[(-1, 1), (0, 1), (1, 1)]);
    let divisor = p(&[(-1, 1), (1, 1)]);
    let (quotient, remainder) = dividend.divide_with_remainder(&divisor).unwrap();
    assert_eq!(quotient, p(&[(1, 1), (1, 1)]));
    assert!(remainder.is_zero());
    assert_division_invariants(&dividend, &divisor);
}

#[test]
fn divides_with_nonzero_fractional_remainder() {
    let dividend = p(&[(1, 2), (0, 1), (1, 1)]);
    let divisor = p(&[(1, 2), (1, 1)]);
    let (quotient, remainder) = dividend.divide_with_remainder(&divisor).unwrap();
    assert_eq!(quotient, p(&[(-1, 2), (1, 1)]));
    assert_eq!(remainder, p(&[(3, 4)]));
    assert_division_invariants(&dividend, &divisor);
}

#[test]
fn lower_degree_and_zero_dividends_return_unchanged_remainder() {
    let dividend = p(&[(1, 1), (2, 1)]);
    let divisor = p(&[(1, 1), (0, 1), (1, 1)]);
    let (quotient, remainder) = dividend.divide_with_remainder(&divisor).unwrap();
    assert_eq!(quotient, RationalPolynomial::zero());
    assert_eq!(remainder, dividend);
    let (q, r) = RationalPolynomial::zero()
        .divide_with_remainder(&divisor)
        .unwrap();
    assert!(q.is_zero());
    assert!(r.is_zero());
}

#[test]
fn rational_constant_division_is_exact() {
    let dividend = p(&[(1, 2), (3, 4), (2, 3)]);
    let divisor = p(&[(2, 3)]);
    let (quotient, remainder) = dividend.divide_with_remainder(&divisor).unwrap();
    assert_eq!(quotient, p(&[(3, 4), (9, 8), (1, 1)]));
    assert!(remainder.is_zero());
}

#[test]
fn gcd_is_monic_and_handles_scaling() {
    let common = p(&[(-1, 1), (1, 1)]);
    let a = common.multiply(&p(&[(1, 1), (1, 1)]));
    let b = common.multiply(&p(&[(2, 1), (1, 1)]));
    assert_eq!(a.monic_gcd(&b), common);
    assert_eq!(b.monic_gcd(&a), common);

    let scaled_a = a.multiply(&p(&[(-7, 3)]));
    let scaled_b = b.multiply(&p(&[(11, 4)]));
    assert_eq!(scaled_a.monic_gcd(&scaled_b), common);
}

#[test]
fn gcd_zero_and_coprime_cases() {
    let zero = RationalPolynomial::zero();
    let a = p(&[(2, 1), (2, 1)]);
    let expected = p(&[(1, 1), (1, 1)]);
    assert_eq!(a.monic_gcd(&zero), expected);
    assert_eq!(zero.monic_gcd(&a), expected);
    assert_eq!(zero.monic_gcd(&zero), zero);
    assert_eq!(
        p(&[(1, 1), (1, 1)]).monic_gcd(&p(&[(2, 1), (1, 1)])),
        p(&[(1, 1)])
    );
}

#[test]
fn gcd_divides_both_operands() {
    let samples = [
        p(&[(2, 1), (3, 1), (1, 1)]),
        p(&[(0, 1), (0, 1), (1, 1)]),
        p(&[(1, 2), (3, 4), (0, 1), (1, 3)]),
        p(&[(3, 1)]),
    ];
    for a in &samples {
        for b in &samples {
            let gcd = a.monic_gcd(b);
            assert!(!gcd.is_zero());
            let (_, rem_a) = a.divide_with_remainder(&gcd).unwrap();
            let (_, rem_b) = b.divide_with_remainder(&gcd).unwrap();
            assert!(rem_a.is_zero());
            assert!(rem_b.is_zero());
        }
    }
}

#[test]
fn long_division_preserves_exact_large_coefficients_and_inputs() {
    let large = Rational::from(Integer::from(u128::MAX));
    let dividend = RationalPolynomial::new(vec![
        large.clone(),
        Rational::from(Integer::zero()),
        large.clone(),
    ]);
    let divisor = RationalPolynomial::new(vec![large.clone()]);
    let old_dividend = dividend.clone();
    let old_divisor = divisor.clone();
    assert_division_invariants(&dividend, &divisor);
    let (quotient, remainder) = dividend.divide_with_remainder(&divisor).unwrap();
    assert_eq!(quotient, p(&[(1, 1), (0, 1), (1, 1)]));
    assert!(remainder.is_zero());
    assert_eq!(dividend, old_dividend);
    assert_eq!(divisor, old_divisor);
}

#[test]
fn exact_division_identity_for_varied_polynomials() {
    let cases = [
        p(&[]),
        p(&[(1, 2)]),
        p(&[(1, 1), (1, 1)]),
        p(&[(2, 1), (0, 1), (-3, 1), (1, 1)]),
    ];
    for dividend in &cases {
        for divisor in &cases[1..] {
            assert_division_invariants(dividend, divisor);
        }
    }
}
