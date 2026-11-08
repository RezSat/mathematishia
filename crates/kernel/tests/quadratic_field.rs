use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, ExactQuadraticRoot, QuadraticBranch, QuadraticElement,
    QuadraticField, QuadraticFieldError, RationalPolynomial,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values.iter().map(|&value| Rational::from(Integer::from(value))).collect(),
    )
}

fn q(a: i64, b: i64) -> Rational {
    Rational::new(Integer::from(a), Integer::from(b)).unwrap()
}

#[test]
fn sqrt_two_field_exact_arithmetic() {
    let ctx = EvaluationContext::default();
    let field = QuadraticField::new(&ctx, p(&[-2, 0, 1]), QuadraticBranch::UpperReal).unwrap();
    let alpha = QuadraticElement::new(&ctx, field.clone(), q(0, 1), q(1, 1)).unwrap();
    let two = alpha.multiply(&alpha, &ctx).unwrap();
    assert_eq!(two.constant(), &q(2, 1));
    assert_eq!(two.coefficient(), &q(0, 1));
    assert_eq!(alpha.norm(&ctx).unwrap(), q(-2, 1));
    assert_eq!(alpha.trace(&ctx).unwrap(), q(0, 1));

    let inverse = alpha.inverse(&ctx).unwrap();
    assert_eq!(inverse.coefficient(), &q(1, 2));
    assert_eq!(alpha.multiply(&inverse, &ctx).unwrap().constant(), &q(1, 1));

    let shifted = QuadraticElement::new(&ctx, field, q(1, 1), q(2, 1)).unwrap();
    assert_eq!(shifted.minimal_polynomial(&ctx).unwrap(), p(&[-7, -2, 1]));
    let ExactQuadraticRoot::Algebraic { minimal_polynomial, branch } =
        shifted.as_exact_root(&ctx).unwrap() else { panic!("must be irrational") };
    assert_eq!(minimal_polynomial, p(&[-7, -2, 1]));
    assert_eq!(branch, QuadraticBranch::UpperReal);
}

#[test]
fn negative_coefficients_flip_real_and_complex_branches() {
    let ctx = EvaluationContext::default();
    for (poly, selected, flipped) in [
        (p(&[-2, 0, 1]), QuadraticBranch::LowerReal, QuadraticBranch::UpperReal),
        (p(&[1, 0, 1]), QuadraticBranch::PositiveImaginary, QuadraticBranch::NegativeImaginary),
    ] {
        let f = QuadraticField::new(&ctx, poly, selected).unwrap();
        let negative = QuadraticElement::new(&ctx, f, q(0, 1), q(-1, 1)).unwrap();
        assert_eq!(negative.as_exact_root(&ctx).unwrap().branch(), Some(flipped));
        assert_eq!(negative.inverse(&ctx).unwrap().multiply(&negative, &ctx).unwrap().constant(), &q(1,1));
    }
}

#[test]
fn quadratic_fields_reject_reducible_polynomials_and_invalid_branches() {
    let ctx = EvaluationContext::default();
    assert_eq!(
        QuadraticField::new(&ctx, p(&[-4, 0, 1]), QuadraticBranch::UpperReal),
        Err(QuadraticFieldError::ReduciblePolynomial),
    );
    assert_eq!(
        QuadraticField::new(&ctx, p(&[2, 1]), QuadraticBranch::UpperReal),
        Err(QuadraticFieldError::NotQuadratic),
    );
    assert_eq!(
        QuadraticField::new(&ctx, p(&[1, 0, 1]), QuadraticBranch::UpperReal),
        Err(QuadraticFieldError::InvalidBranch),
    );
}

#[test]
fn different_embeddings_are_not_implicitly_mixed() {
    let ctx = EvaluationContext::default();
    let upper = QuadraticField::new(&ctx, p(&[-2, 0, 1]), QuadraticBranch::UpperReal).unwrap();
    let lower = QuadraticField::new(&ctx, p(&[-2, 0, 1]), QuadraticBranch::LowerReal).unwrap();
    let a = QuadraticElement::new(&ctx, upper, q(0,1), q(1,1)).unwrap();
    let b = QuadraticElement::new(&ctx, lower, q(0,1), q(1,1)).unwrap();
    assert_eq!(a.add(&b, &ctx), Err(QuadraticFieldError::MismatchedFields));
}

#[test]
fn irrational_roots_construct_quadratic_elements() {
    let ctx = EvaluationContext::default();
    let root = ExactQuadraticRoot::Algebraic {
        minimal_polynomial: p(&[1, 0, 1]),
        branch: QuadraticBranch::PositiveImaginary,
    };
    let alpha = QuadraticElement::from_root(&ctx, &root).unwrap();
    assert_eq!(alpha.as_exact_root(&ctx).unwrap(), root);
    assert_eq!(alpha.multiply(&alpha, &ctx).unwrap().constant(), &q(-1, 1));
    assert_eq!(
        QuadraticElement::from_root(&ctx, &ExactQuadraticRoot::Rational(q(2,1))),
        Err(QuadraticFieldError::RationalRoot),
    );
}
