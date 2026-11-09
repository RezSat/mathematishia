use mathema::{Integer, Rational};
use mathematishia_kernel::{EvaluationContext, RationalPolynomial, verify_rational_factorization};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&v| Rational::from(Integer::from(v)))
            .collect(),
    )
}

#[test]
fn cubic_irreducibility_and_linear_extraction_are_certified() {
    let ctx = EvaluationContext::default();
    let irreducible = ctx.factor_rational_polynomial(&p(&[-2, 0, 0, 1])).unwrap();
    assert!(irreducible.is_complete());
    assert_eq!(irreducible.factors().len(), 1);
    assert_eq!(irreducible.factors()[0].polynomial(), &p(&[-2, 0, 0, 1]));
    let split = p(&[-1, 1]).multiply(&p(&[1, 0, 1]));
    let result = ctx.factor_rational_polynomial(&split).unwrap();
    assert!(result.is_complete());
    assert_eq!(result.factors().len(), 2);
    assert_eq!(verify_rational_factorization(&split, &result), Ok(()));
}

#[test]
fn repeated_quadratic_factors_are_not_left_unresolved() {
    let ctx = EvaluationContext::default();
    let irreducible = p(&[2, 0, 1]);
    let original = irreducible.multiply(&irreducible).multiply(&p(&[-3, 1]));
    let result = ctx.factor_rational_polynomial(&original).unwrap();
    assert!(result.is_complete());
    assert!(
        result
            .factors()
            .iter()
            .any(|f| f.polynomial() == &irreducible && f.multiplicity() == 2)
    );
    assert_eq!(verify_rational_factorization(&original, &result), Ok(()));
}

#[test]
fn bounded_quartic_search_does_not_guess_irreducibility() {
    let ctx = EvaluationContext::default();
    let polynomial = p(&[2_000_000, 0, 3_000, 0, 1]);
    let result = ctx.factor_rational_polynomial(&polynomial).unwrap();
    assert!(!result.is_complete());
    assert_eq!(result.unresolved()[0].polynomial(), &polynomial);
    assert_eq!(verify_rational_factorization(&polynomial, &result), Ok(()));
}

#[test]
fn modular_certificate_handles_large_cubic_coefficients() {
    let ctx = EvaluationContext::default();
    let large = p(&[-100_003, 0, 0, 1]);
    let result = ctx.factor_rational_polynomial(&large).unwrap();
    assert!(result.is_complete());
    assert_eq!(result.factors()[0].polynomial(), &large);
}

#[test]
fn scalar_units_and_zero_are_distinguished() {
    let ctx = EvaluationContext::default();
    let constant = ctx.factor_rational_polynomial(&p(&[-7])).unwrap();
    assert!(constant.is_complete());
    assert_eq!(constant.unit(), &Rational::from(Integer::from(-7)));
    assert!(constant.factors().is_empty());
    assert!(ctx.factor_rational_polynomial(&p(&[])).is_err());
}

#[test]
fn exact_rational_roots_use_primitive_integer_scaling() {
    let ctx = EvaluationContext::default();
    let half = Rational::new(Integer::from(1), Integer::from(2)).unwrap();
    let polynomial = RationalPolynomial::new(vec![-&half, Rational::from(Integer::one())]);
    let result = ctx.factor_rational_polynomial(&polynomial).unwrap();
    assert!(result.is_complete());
    assert_eq!(result.factors()[0].polynomial(), &polynomial);
}

#[test]
fn deterministic_bivariate_linear_factor_grid() {
    let ctx = EvaluationContext::default();
    for a in -3..=3 {
        for b in -3..=3 {
            let first = p(&[-a, 1]);
            let second = p(&[-b, 1]);
            let product = first.multiply(&second);
            let result = ctx.factor_rational_polynomial(&product).unwrap();
            assert!(result.is_complete());
            assert_eq!(verify_rational_factorization(&product, &result), Ok(()));
            assert_eq!(
                result
                    .factors()
                    .iter()
                    .map(|f| f.multiplicity())
                    .sum::<usize>(),
                2
            );
        }
    }
}
