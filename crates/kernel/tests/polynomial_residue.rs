use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, PolynomialResidue, RationalPolynomial, ResidueError, verify_residue_product,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&value| Rational::from(Integer::from(value)))
            .collect(),
    )
}

fn r(value: RationalPolynomial, modulus: RationalPolynomial) -> PolynomialResidue {
    PolynomialResidue::new(&EvaluationContext::default(), value, modulus).unwrap()
}

#[test]
fn canonical_representatives_ignore_scalar_multiples_of_modulus() {
    let a = r(p(&[0, 0, 1]), p(&[1, 0, 1]));
    let b = r(p(&[-1]), p(&[2, 0, 2]));
    assert_eq!(a, b);
    assert_eq!(a.representative(), &p(&[-1]));
    assert_eq!(a.modulus(), &p(&[1, 0, 1]));
}

#[test]
fn exact_complex_quadratic_ring_arithmetic_and_unit_inverse() {
    let ctx = EvaluationContext::default();
    let i = r(p(&[0, 1]), p(&[1, 0, 1]));
    let one = r(p(&[1]), p(&[1, 0, 1]));
    assert_eq!(i.multiply(&i, &ctx).unwrap().representative(), &p(&[-1]));
    assert_eq!(i.pow_signed(4, &ctx).unwrap().unwrap(), one);
    assert_eq!(
        i.pow_signed(-1, &ctx).unwrap().unwrap().representative(),
        &p(&[0, -1])
    );
    let sum = i.add(&one, &ctx).unwrap();
    let product = sum.multiply(&sum, &ctx).unwrap();
    assert_eq!(product.representative(), &p(&[0, 2]));
    assert!(verify_residue_product(&ctx, &sum, &sum, &product).unwrap());
    assert!(!verify_residue_product(&ctx, &sum, &sum, &one).unwrap());
}

#[test]
fn reducible_quotients_are_rings_not_automatically_fields() {
    let ctx = EvaluationContext::default();
    let x_minus_one = r(p(&[-1, 1]), p(&[-1, 0, 1]));
    let x_plus_one = r(p(&[1, 1]), p(&[-1, 0, 1]));
    assert!(x_minus_one.multiply(&x_plus_one, &ctx).unwrap().is_zero());
    assert!(x_minus_one.inverse(&ctx).unwrap().is_none());
    assert!(x_minus_one.pow_signed(-1, &ctx).unwrap().is_none());
    assert!(x_minus_one.pow_signed(0, &ctx).unwrap().is_some());
}

#[test]
fn mismatched_moduli_and_constant_moduli_are_rejected() {
    let ctx = EvaluationContext::default();
    assert_eq!(
        PolynomialResidue::new(&ctx, p(&[1]), p(&[2])),
        Err(ResidueError::InvalidModulus)
    );
    assert_eq!(
        PolynomialResidue::new(&ctx, p(&[1]), p(&[])),
        Err(ResidueError::InvalidModulus)
    );
    let a = r(p(&[1]), p(&[1, 0, 1]));
    let b = r(p(&[1]), p(&[2, 0, 1]));
    assert_eq!(a.add(&b, &ctx), Err(ResidueError::MismatchedModulus));
    assert_eq!(
        verify_residue_product(&ctx, &a, &b, &a),
        Err(ResidueError::MismatchedModulus)
    );
}

#[test]
fn distributive_identity_holds_across_deterministic_inputs() {
    let ctx = EvaluationContext::default();
    let modulus = p(&[2, 0, 1]);
    for a in -3..=3 {
        for b in -3..=3 {
            let x = r(p(&[a, 1]), modulus.clone());
            let y = r(p(&[b, 1]), modulus.clone());
            let z = r(p(&[1, -1]), modulus.clone());
            let left = x.multiply(&y.add(&z, &ctx).unwrap(), &ctx).unwrap();
            let right = x
                .multiply(&y, &ctx)
                .unwrap()
                .add(&x.multiply(&z, &ctx).unwrap(), &ctx)
                .unwrap();
            assert_eq!(left, right);
            assert!(verify_residue_product(&ctx, &x, &y.add(&z, &ctx).unwrap(), &left).unwrap());
        }
    }
}
