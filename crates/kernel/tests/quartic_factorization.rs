use mathema::{Integer, Rational};
use mathematishia_kernel::{EvaluationContext, RationalPolynomial, verify_rational_factorization};

fn p(v: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        v.iter()
            .map(|&x| Rational::from(Integer::from(x)))
            .collect(),
    )
}

#[test]
fn irreducible_quartics_have_exact_certificates() {
    let ctx = EvaluationContext::default();
    for poly in [p(&[1, 0, 0, 0, 1]), p(&[1, 1, 0, 0, 1])] {
        let result = ctx.factor_rational_polynomial(&poly).unwrap();
        assert!(result.is_complete());
        assert_eq!(result.factors().len(), 1);
        assert_eq!(result.factors()[0].polynomial(), &poly);
        assert_eq!(verify_rational_factorization(&poly, &result), Ok(()));
    }
}

#[test]
fn quartics_split_without_rational_roots() {
    let ctx = EvaluationContext::default();
    let original = p(&[4, 0, 0, 0, 1]);
    let result = ctx.factor_rational_polynomial(&original).unwrap();
    assert!(result.is_complete());
    assert_eq!(result.factors().len(), 2);
    assert!(
        result
            .factors()
            .iter()
            .all(|factor| factor.polynomial().degree() == Some(2))
    );
    assert_eq!(verify_rational_factorization(&original, &result), Ok(()));
}

#[test]
fn larger_degree_modular_irreducibility_is_certified() {
    let ctx = EvaluationContext::default();
    let poly = p(&[1, 1, 0, 0, 1]);
    let product = poly.multiply(&p(&[-2, 1]));
    let result = ctx.factor_rational_polynomial(&product).unwrap();
    assert!(result.is_complete());
    assert_eq!(result.factors().len(), 2);
    assert_eq!(verify_rational_factorization(&product, &result), Ok(()));
}

#[test]
fn unsupported_cofactors_remain_explicit() {
    let ctx = EvaluationContext::default();
    let original = p(&[1, 0, 0, 0, 2]);
    let result = ctx.factor_rational_polynomial(&original).unwrap();
    assert_eq!(verify_rational_factorization(&original, &result), Ok(()));
}
