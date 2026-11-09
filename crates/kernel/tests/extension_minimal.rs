use mathema::{Integer, Rational};
use mathematishia_kernel::{
    CertifiedAlgebraicField, EvaluationContext, RationalPolynomial,
    verify_extension_minimal_polynomial,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&v| Rational::from(Integer::from(v)))
            .collect(),
    )
}

#[test]
fn cubic_generator_and_shifted_powers_have_exact_minimal_polynomials() {
    let ctx = EvaluationContext::default();
    let field = CertifiedAlgebraicField::new(&ctx, p(&[-2, 0, 0, 1])).unwrap();
    let a = field.generator(&ctx).unwrap();
    let a2 = a.multiply(&a, &ctx).unwrap();
    let shifted = a.add(&field.element(&ctx, p(&[1])).unwrap(), &ctx).unwrap();
    assert_eq!(a.minimal_polynomial(&ctx).unwrap(), p(&[-2, 0, 0, 1]));
    assert_eq!(a2.minimal_polynomial(&ctx).unwrap(), p(&[-4, 0, 0, 1]));
    assert_eq!(
        shifted.minimal_polynomial(&ctx).unwrap(),
        p(&[-3, 3, -3, 1])
    );
    for value in [&a, &a2, &shifted] {
        let candidate = value.minimal_polynomial(&ctx).unwrap();
        assert!(verify_extension_minimal_polynomial(&ctx, value, &candidate).unwrap());
    }
}

#[test]
fn rational_elements_get_degree_one_polynomials() {
    let ctx = EvaluationContext::default();
    let field = CertifiedAlgebraicField::new(&ctx, p(&[-2, 0, 0, 1])).unwrap();
    let half = Rational::new(Integer::one(), Integer::from(2)).unwrap();
    let value = field
        .element(&ctx, RationalPolynomial::new(vec![half.clone()]))
        .unwrap();
    assert_eq!(
        value.minimal_polynomial(&ctx).unwrap(),
        RationalPolynomial::new(vec![-&half, Rational::from(Integer::one())])
    );
}

#[test]
fn verifier_rejects_false_annihilators_and_reducible_candidates() {
    let ctx = EvaluationContext::default();
    let field = CertifiedAlgebraicField::new(&ctx, p(&[-2, 0, 0, 1])).unwrap();
    let a = field.generator(&ctx).unwrap();
    for forged in [
        p(&[-1, 0, 0, 1]),
        p(&[-2, 0, 0, 2]),
        p(&[0, 0, 1]),
        p(&[-2, 0, 0, 0, 1]),
    ] {
        assert!(!verify_extension_minimal_polynomial(&ctx, &a, &forged).unwrap());
    }
}

#[test]
fn deterministic_general_cubic_values_are_annhilated() {
    let ctx = EvaluationContext::default();
    let field = CertifiedAlgebraicField::new(&ctx, p(&[-2, 0, 0, 1])).unwrap();
    for a in -2..=2 {
        for b in -2..=2 {
            let value = field.element(&ctx, p(&[a, b, 1])).unwrap();
            let minimal = value.minimal_polynomial(&ctx).unwrap();
            assert!(verify_extension_minimal_polynomial(&ctx, &value, &minimal).unwrap());
            assert_eq!(minimal.degree(), Some(3));
        }
    }
}
