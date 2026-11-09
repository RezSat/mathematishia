use mathema::{Integer, Rational};
use mathematishia_kernel::{
    AlgebraicExtensionError, CertifiedAlgebraicField, EvaluationContext, RationalPolynomial,
    verify_certified_algebraic_field,
};

fn p(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients.iter().map(|&n| Rational::from(Integer::from(n))).collect()
    )
}

#[test]
fn cubic_extension_supports_exact_arithmetic_and_inverse() {
    let ctx = EvaluationContext::default();
    let field = CertifiedAlgebraicField::new(&ctx, p(&[-2,0,0,1])).unwrap();
    assert!(verify_certified_algebraic_field(&field));
    let alpha = field.generator(&ctx).unwrap();
    let cube = alpha.pow_signed(3, &ctx).unwrap();
    assert_eq!(cube.coefficients(), &p(&[2]));
    let inverse = alpha.inverse(&ctx).unwrap();
    let half = Rational::new(Integer::one(), Integer::from(2)).unwrap();
    assert_eq!(inverse.coefficients(), &RationalPolynomial::new(vec![
        Rational::from(Integer::zero()), Rational::from(Integer::zero()), half
    ]));
    assert_eq!(alpha.multiply(&inverse, &ctx).unwrap().coefficients(), &p(&[1]));
    assert_eq!(alpha.pow_signed(-3, &ctx).unwrap().coefficients(),
        &RationalPolynomial::new(vec![Rational::new(Integer::one(),Integer::from(2)).unwrap()]));
}

#[test]
fn reducible_and_unproven_moduli_are_rejected() {
    let ctx = EvaluationContext::default();
    assert_eq!(
        CertifiedAlgebraicField::new(&ctx, p(&[-1,0,0,1])),
        Err(AlgebraicExtensionError::IrreducibilityNotCertified),
    );
    assert_eq!(
        CertifiedAlgebraicField::new(&ctx, p(&[1,0,0,0,1])),
        Err(AlgebraicExtensionError::UnsupportedDegree),
    );
    assert_eq!(
        CertifiedAlgebraicField::new(&ctx, p(&[-100_003,0,0,1])),
        Err(AlgebraicExtensionError::IrreducibilityNotCertified),
    );
}

#[test]
fn different_presentations_are_not_silently_identified() {
    let ctx = EvaluationContext::default();
    let two = CertifiedAlgebraicField::new(&ctx, p(&[-2,0,0,1])).unwrap();
    let three = CertifiedAlgebraicField::new(&ctx, p(&[-3,0,0,1])).unwrap();
    let first = two.generator(&ctx).unwrap();
    let second = three.generator(&ctx).unwrap();
    assert_eq!(first.add(&second, &ctx), Err(AlgebraicExtensionError::DifferentFields));
    assert_eq!(first.multiply(&second, &ctx), Err(AlgebraicExtensionError::DifferentFields));
    let zero = two.element(&ctx, p(&[0])).unwrap();
    assert_eq!(zero.inverse(&ctx), Err(AlgebraicExtensionError::DivisionByZero));
}

#[test]
fn scalar_multiples_of_field_polynomial_normalize_identically() {
    let ctx = EvaluationContext::default();
    let first = CertifiedAlgebraicField::new(&ctx, p(&[-2,0,0,1])).unwrap();
    let second = CertifiedAlgebraicField::new(&ctx, p(&[-6,0,0,3])).unwrap();
    assert_eq!(first, second);
    assert_eq!(second.generator(&ctx).unwrap().pow_signed(3, &ctx).unwrap().coefficients(), &p(&[2]));
}

#[test]
fn deterministic_field_distributivity() {
    let ctx = EvaluationContext::default();
    let field = CertifiedAlgebraicField::new(&ctx, p(&[-2,0,0,1])).unwrap();
    for a in -3..=3 {
        for b in -3..=3 {
            let x = field.element(&ctx, p(&[a,1,1])).unwrap();
            let y = field.element(&ctx, p(&[b,0,1])).unwrap();
            let z = field.generator(&ctx).unwrap();
            let left = x.multiply(&y.add(&z,&ctx).unwrap(), &ctx).unwrap();
            let right = x.multiply(&y,&ctx).unwrap()
                .add(&x.multiply(&z,&ctx).unwrap(),&ctx).unwrap();
            assert_eq!(left,right);
        }
    }
}
