use mathema::{Integer, Rational};
use mathematishia_kernel::{EvaluationContext, RationalPolynomial, verify_rational_factorization};

fn p(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients
            .iter()
            .map(|&number| Rational::from(Integer::from(number)))
            .collect(),
    )
}

#[test]
fn modular_hensel_reconstruction_splits_degree_six_without_linear_roots() {
    let ctx = EvaluationContext::default();
    let first = p(&[1, 1, 0, 1]);
    let second = p(&[1, 0, 1, 1]);
    let original = first.multiply(&second);
    let result = ctx.factor_rational_polynomial(&original).unwrap();
    assert!(result.is_complete());
    assert_eq!(result.factors().len(), 2);
    assert_eq!(verify_rational_factorization(&original, &result), Ok(()));
    assert!(
        result
            .factors()
            .iter()
            .any(|factor| factor.polynomial() == &first)
    );
    assert!(
        result
            .factors()
            .iter()
            .any(|factor| factor.polynomial() == &second)
    );
}

#[test]
fn composite_factors_reconstruct_exact_product_with_multiplicity() {
    let ctx = EvaluationContext::default();
    let cubic = p(&[1, 1, 0, 1]);
    let quartic = p(&[1, 0, 0, 0, 1]);
    let original = cubic.multiply(&quartic);
    let result = ctx.factor_rational_polynomial(&original).unwrap();
    assert!(result.is_complete());
    assert_eq!(verify_rational_factorization(&original, &result), Ok(()));
}

#[test]
fn unrelated_large_degree_factor_is_not_guessed() {
    let ctx = EvaluationContext::default();
    let f = p(&[10_000, 0, 0, 0, 0, 1]);
    let result = ctx.factor_rational_polynomial(&f).unwrap();
    assert_eq!(verify_rational_factorization(&f, &result), Ok(()));
}
