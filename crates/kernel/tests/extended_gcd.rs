use mathema::{Integer, Rational};
use mathematishia_kernel::{EvaluationContext, RationalPolynomial, verify_extended_gcd};

fn poly(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients.iter().map(|&n| Rational::from(Integer::from(n))).collect()
    )
}

#[test]
fn bezout_gcd_handles_zero_and_coprime_cases() {
    let ctx = EvaluationContext::default();
    let cases = [
        (poly(&[]), poly(&[]), poly(&[])),
        (poly(&[-2, 2]), poly(&[]), poly(&[-1, 1])),
        (poly(&[]), poly(&[0, 2]), poly(&[0, 1])),
        (poly(&[-1, 0, 0, 1]), poly(&[-1, 0, 1]), poly(&[-1, 1])),
        (poly(&[1, 0, 1]), poly(&[1, 1]), poly(&[1])),
    ];
    for (left, right, expected) in cases {
        let result = ctx.extended_gcd(&left, &right).unwrap();
        assert_eq!(result.gcd(), &expected);
        assert_eq!(verify_extended_gcd(&result), Ok(()));
        assert_eq!(
            result.left_coefficient().multiply(&left)
                .add(&result.right_coefficient().multiply(&right)),
            expected,
        );
    }
}

#[test]
fn deterministic_bezout_witnesses_for_many_polynomial_pairs() {
    let ctx = EvaluationContext::default();
    let x = poly(&[0, 1]);
    for a in -3..=3 {
        for b in -3..=3 {
            let first = x.subtract(&poly(&[a])).multiply(&x.subtract(&poly(&[b])));
            let second = x.subtract(&poly(&[a])).multiply(&x.add(&poly(&[2])));
            let result = ctx.extended_gcd(&first, &second).unwrap();
            assert_eq!(verify_extended_gcd(&result), Ok(()));
            assert_eq!(
                result,
                ctx.extended_gcd(&first, &second).unwrap(),
            );
        }
    }
}

#[test]
fn modular_inverse_exists_exactly_for_coprime_inputs() {
    let ctx = EvaluationContext::default();
    let modulus = poly(&[1, 0, 1]);
    let x = poly(&[0, 1]);
    let inverse = ctx.inverse_mod_polynomial(&x, &modulus).unwrap().unwrap();
    let (_, residue) = inverse.multiply(&x).divide_with_remainder(&modulus).unwrap();
    assert_eq!(residue, poly(&[1]));

    let not_coprime = poly(&[-1, 0, 1]);
    assert_eq!(ctx.inverse_mod_polynomial(&poly(&[-1, 1]), &not_coprime).unwrap(), None);
    assert!(ctx.inverse_mod_polynomial(&x, &poly(&[])).is_err());
}
