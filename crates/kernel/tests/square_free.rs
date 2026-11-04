use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalPolynomial, ResourceLimitKind, ResourceLimits,
    SquareFreeError, verify_square_free_decomposition,
};

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn polynomial(coefficients: &[(i64, i64)]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients
            .iter()
            .map(|&(numerator, denominator)| rational(numerator, denominator))
            .collect(),
    )
}

fn linear(root: i64) -> RationalPolynomial {
    polynomial(&[(-root, 1), (1, 1)])
}

fn power(base: &RationalPolynomial, exponent: usize) -> RationalPolynomial {
    (0..exponent).fold(polynomial(&[(1, 1)]), |value, _| value.multiply(base))
}

#[test]
fn separates_two_multiplicity_classes_and_preserves_nonmonic_unit() {
    let original = power(&linear(1), 2)
        .multiply(&power(&linear(-2), 3))
        .multiply(&polynomial(&[(2, 1)]));
    let decomposition = original.square_free_decomposition().unwrap();
    assert_eq!(decomposition.unit(), &rational(2, 1));
    assert_eq!(decomposition.factors().len(), 2);
    assert_eq!(decomposition.factors()[0].multiplicity(), 2);
    assert_eq!(decomposition.factors()[0].polynomial(), &linear(1));
    assert_eq!(decomposition.factors()[1].multiplicity(), 3);
    assert_eq!(decomposition.factors()[1].polynomial(), &linear(-2));
    assert_eq!(
        verify_square_free_decomposition(&original, &decomposition),
        Ok(())
    );
}

#[test]
fn constants_have_only_a_unit_and_zero_is_rejected() {
    for value in [rational(3, 4), rational(-7, 2), rational(1, 1)] {
        let original = RationalPolynomial::new(vec![value.clone()]);
        let result = original.square_free_decomposition().unwrap();
        assert_eq!(result.unit(), &value);
        assert!(result.factors().is_empty());
        assert_eq!(verify_square_free_decomposition(&original, &result), Ok(()));
    }
    assert_eq!(
        RationalPolynomial::zero().square_free_decomposition(),
        Err(SquareFreeError::ZeroPolynomial)
    );
}

#[test]
fn square_free_does_not_claim_to_factor_irreducible_polynomials() {
    let original = polynomial(&[(1, 1), (0, 1), (1, 1)]);
    let decomposition = original.square_free_decomposition().unwrap();
    assert_eq!(decomposition.factors().len(), 1);
    assert_eq!(decomposition.factors()[0].multiplicity(), 1);
    assert_eq!(decomposition.factors()[0].polynomial(), &original);
    assert_eq!(
        verify_square_free_decomposition(&original, &decomposition),
        Ok(())
    );
}

#[test]
fn published_square_free_reference_polynomial_matches_exact_multiplicities() {
    // SymPy's reference example: 2*(x+1)^2*(x+2)^3
    let input = polynomial(&[
        (16, 1), (56, 1), (76, 1), (50, 1), (16, 1), (2, 1),
    ]);
    let output = input.square_free_decomposition().unwrap();
    assert_eq!(output.unit(), &rational(2, 1));
    assert_eq!(output.factors().len(), 2);
    assert_eq!(output.factors()[0].polynomial(), &linear(-1));
    assert_eq!(output.factors()[0].multiplicity(), 2);
    assert_eq!(output.factors()[1].polynomial(), &linear(-2));
    assert_eq!(output.factors()[1].multiplicity(), 3);
    assert_eq!(verify_square_free_decomposition(&input, &output), Ok(()));
}

#[test]
fn repeated_root_at_zero_and_negative_rational_unit_are_exact() {
    let original = power(&linear(0), 7).multiply(&polynomial(&[(-3, 4)]));
    let decomposition = original.square_free_decomposition().unwrap();
    assert_eq!(decomposition.unit(), &rational(-3, 4));
    assert_eq!(decomposition.factors().len(), 1);
    assert_eq!(decomposition.factors()[0].polynomial(), &linear(0));
    assert_eq!(decomposition.factors()[0].multiplicity(), 7);
    assert_eq!(
        verify_square_free_decomposition(&original, &decomposition),
        Ok(())
    );
}

#[test]
fn deterministic_grid_of_exact_repeated_factor_products_verifies() {
    let ctx = EvaluationContext::default();
    let mut cases = 0usize;
    for root_a in [-2, 0, 3] {
        for root_b in [-1, 2] {
            for exponent_a in 1..=4 {
                for exponent_b in 1..=4 {
                    let original = power(&linear(root_a), exponent_a)
                        .multiply(&power(&linear(root_b), exponent_b));
                    let saved = original.clone();
                    let first = ctx.square_free_decomposition(&original).unwrap();
                    let second = ctx.square_free_decomposition(&original).unwrap();
                    assert_eq!(first, second);
                    assert_eq!(original, saved);
                    assert_eq!(verify_square_free_decomposition(&original, &first), Ok(()));
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 96);
}

#[test]
fn different_roots_with_same_multiplicity_remain_one_square_free_factor() {
    let composite = linear(-2).multiply(&linear(1));
    let input = power(&composite, 2);
    let result = input.square_free_decomposition().unwrap();
    assert_eq!(result.factors().len(), 1);
    assert_eq!(result.factors()[0].polynomial(), &composite);
    assert_eq!(result.factors()[0].multiplicity(), 2);
    assert_eq!(verify_square_free_decomposition(&input, &result), Ok(()));
}

#[test]
fn repeated_multiplicity_with_a_shared_root_is_combined() {
    let common_root = linear(1);
    let input = power(&common_root, 2).multiply(&power(&common_root, 3));
    let answer = input.square_free_decomposition().unwrap();
    assert_eq!(answer.factors().len(), 1);
    assert_eq!(answer.factors()[0].polynomial(), &common_root);
    assert_eq!(answer.factors()[0].multiplicity(), 5);
    assert_eq!(verify_square_free_decomposition(&input, &answer), Ok(()));
}

#[test]
fn a_bound_context_refuses_large_admitted_coefficients() {
    let input = polynomial(&[(0, 1), (8, 1)]);
    let ctx = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.square_free_decomposition(&input),
        Err(EvaluationError::Limit(ResourceLimitKind::ScalarBits))
    );
}
