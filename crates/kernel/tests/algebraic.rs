use mathema::{Integer, Rational};
use mathematishia_kernel::{
    AlgebraicRootEvidence, AlgebraicRootRule, AlgebraicRootSet, EvaluationContext, EvaluationError,
    ExactQuadraticRoot, QuadraticBranch, QuadraticError, QuadraticRoots, RationalPolynomial,
    ResourceLimitKind, ResourceLimits, verify_algebraic_roots,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&v| Rational::from(Integer::from(v)))
            .collect(),
    )
}

fn branches(
    poly: &RationalPolynomial,
    first: QuadraticBranch,
    second: QuadraticBranch,
) -> AlgebraicRootSet {
    AlgebraicRootSet::Finite(vec![
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial: poly.clone(),
            branch: first,
        },
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial: poly.clone(),
            branch: second,
        },
    ])
}

#[test]
fn exact_rational_real_irrational_and_nonreal_cases() {
    let cases = [
        (
            p(&[]),
            AlgebraicRootSet::AllComplexNumbers,
            AlgebraicRootRule::ZeroPolynomial,
        ),
        (
            p(&[7]),
            AlgebraicRootSet::Finite(vec![]),
            AlgebraicRootRule::NonzeroConstant,
        ),
        (
            p(&[-4, 2]),
            AlgebraicRootSet::Finite(vec![ExactQuadraticRoot::Rational(Rational::from(
                Integer::from(2),
            ))]),
            AlgebraicRootRule::Linear,
        ),
        (
            p(&[-2, 0, 1]),
            branches(
                &p(&[-2, 0, 1]),
                QuadraticBranch::LowerReal,
                QuadraticBranch::UpperReal,
            ),
            AlgebraicRootRule::IrrationalRealPair,
        ),
        (
            p(&[1, 0, 1]),
            branches(
                &p(&[1, 0, 1]),
                QuadraticBranch::NegativeImaginary,
                QuadraticBranch::PositiveImaginary,
            ),
            AlgebraicRootRule::NonrealConjugatePair,
        ),
    ];
    let ctx = EvaluationContext::default();
    for (poly, expected, rule) in cases {
        let actual = ctx.algebraic_roots(&poly).unwrap();
        assert_eq!(actual.roots(), &expected);
        assert_eq!(
            actual.evidence().last(),
            Some(&AlgebraicRootEvidence::Classification(rule))
        );
        assert_eq!(verify_algebraic_roots(&actual), Ok(()));
    }
}

#[test]
fn scalar_multiples_have_identical_canonical_roots() {
    let ctx = EvaluationContext::default();
    for original in [p(&[-2, 0, 1]), p(&[1, 0, 1]), p(&[6, -5, 1])] {
        let reference = ctx.algebraic_roots(&original).unwrap();
        for scale in [-7, -2, 1, 3, 9] {
            let coefficients = original
                .coefficients()
                .iter()
                .map(|coefficient| coefficient * &Rational::from(Integer::from(scale)))
                .collect();
            let result = ctx
                .algebraic_roots(&RationalPolynomial::new(coefficients))
                .unwrap();
            assert_eq!(result.roots(), reference.roots());
            assert_eq!(verify_algebraic_roots(&result), Ok(()));
        }
    }
}

#[test]
fn exact_343_grid_cases_match_existing_rational_solver() {
    let ctx = EvaluationContext::default();
    let mut checked = 0;
    for a in -3..=3 {
        for b in -3..=3 {
            for c in -3..=3 {
                let poly = p(&[a, b, c]);
                let result = ctx.algebraic_roots(&poly).unwrap();
                let rational = ctx.solve_quadratic(&poly).unwrap();
                assert_eq!(verify_algebraic_roots(&result), Ok(()));
                match (result.roots(), rational.roots()) {
                    (AlgebraicRootSet::AllComplexNumbers, QuadraticRoots::AllRationals) => {}
                    (AlgebraicRootSet::Finite(all), QuadraticRoots::Finite(expected)) => {
                        let observed = all
                            .iter()
                            .filter_map(ExactQuadraticRoot::as_rational)
                            .cloned()
                            .collect::<Vec<_>>();
                        assert_eq!(&observed, expected);
                    }
                    _ => panic!("disagreeing solution classifications"),
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 343);
}

#[test]
fn limits_and_degree_are_admitted_before_result() {
    let tiny = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        tiny.algebraic_roots(&p(&[-2, 0, 1])),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    assert_eq!(
        EvaluationContext::default().algebraic_roots(&p(&[1, 0, 0, 1])),
        Err(EvaluationError::Quadratic(QuadraticError::DegreeExceeded))
    );
}

#[test]
fn exact_discriminant_and_stable_rule_identifiers() {
    let result = EvaluationContext::default()
        .algebraic_roots(&p(&[-2, 0, 1]))
        .unwrap();
    assert_eq!(
        result.evidence()[1],
        AlgebraicRootEvidence::Discriminant(Rational::from(Integer::from(8)))
    );
    assert_eq!(QuadraticBranch::LowerReal.stable_id(), "lower_real");
    assert_eq!(
        AlgebraicRootRule::CalculateDiscriminant.stable_id(),
        "roots.quadratic.discriminant"
    );
}
