use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalPolynomial, ResourceLimitKind, ResourceLimits,
};

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn p(terms: &[(i64, i64)]) -> RationalPolynomial {
    RationalPolynomial::new(terms.iter().map(|&(n, d)| q(n, d)).collect())
}

#[test]
fn exact_formal_derivative_of_rational_polynomials() {
    let polynomial = p(&[(3, 4), (-5, 2), (7, 3), (1, 2)]);
    let expected = p(&[(-5, 2), (14, 3), (3, 2)]);
    assert_eq!(polynomial.differentiate(), expected);
    assert_eq!(polynomial.degree(), Some(3));
    assert_eq!(expected.degree(), Some(2));
}

#[test]
fn zero_and_constant_derivatives_are_canonical_zero() {
    for value in [p(&[]), p(&[(0, 1)]), p(&[(8, 3)]), p(&[(-7, 2)])] {
        assert_eq!(value.differentiate(), RationalPolynomial::zero());
    }
    assert_eq!(p(&[(0, 1), (1, 2)]).differentiate(), p(&[(1, 2)]));
}

#[test]
fn exact_linearity_and_product_rule_hold_on_a_grid() {
    let samples = [
        p(&[]),
        p(&[(2, 1)]),
        p(&[(1, 3), (-2, 1)]),
        p(&[(0, 1), (0, 1), (5, 2)]),
        p(&[(1, 2), (-3, 4), (2, 3), (1, 1)]),
    ];
    for a in &samples {
        for b in &samples {
            assert_eq!(
                a.add(b).differentiate(),
                a.differentiate().add(&b.differentiate())
            );
            assert_eq!(
                a.subtract(b).differentiate(),
                a.differentiate().subtract(&b.differentiate())
            );
            assert_eq!(
                a.multiply(b).differentiate(),
                a.differentiate()
                    .multiply(b)
                    .add(&a.multiply(&b.differentiate()))
            );
        }
    }
}

#[test]
fn repeated_derivatives_terminate_at_zero() {
    let polynomial = p(&[(1, 3), (2, 1), (3, 1), (4, 1)]);
    let first = polynomial.differentiate();
    let second = first.differentiate();
    let third = second.differentiate();
    let fourth = third.differentiate();
    assert_eq!(third, p(&[(24, 1)]));
    assert!(fourth.is_zero());
    assert!(fourth.differentiate().is_zero());
}

#[test]
fn large_exact_coefficients_are_never_approximated() {
    let large = Rational::from(Integer::from(u128::MAX));
    let polynomial = RationalPolynomial::new(vec![
        Rational::from(Integer::zero()),
        Rational::from(Integer::zero()),
        large.clone(),
    ]);
    let result = polynomial.differentiate();
    assert_eq!(
        result.coefficients(),
        &[Rational::from(Integer::zero()), &large + &large]
    );
}

#[test]
fn operations_preserve_caller_owned_inputs() {
    let polynomial = p(&[(1, 2), (-2, 3), (5, 7)]);
    let saved = polynomial.clone();
    let _ = polynomial.differentiate();
    assert_eq!(polynomial, saved);
}

#[test]
fn context_checks_exact_input_and_output() {
    let poly = p(&[(0, 1), (1, 1), (3, 1)]);
    let normal = EvaluationContext::default();
    assert_eq!(normal.differentiate(&poly).unwrap(), p(&[(1, 1), (6, 1)]));
    let limited = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        limited.differentiate(&poly),
        Err(EvaluationError::Limit(ResourceLimitKind::ScalarBits))
    );
}
