use mathema::{Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    AbsoluteAffine, AbsoluteEquation, AbsoluteExpression, AbsoluteSign, CertifiedSolutionSet,
    EvaluationContext, EvaluationError, ExactQuadraticRoot, IntegrationError, PiMultiple,
    PolynomialRootMultiplicity, QuadraticBranch, RationalPolynomial, ResourceLimitKind,
    ResourceLimits, SpecialAngleOutcome, TrigError, TrigFunction, TrigLimits, certify_solution_set,
    evaluate_special_angle, solve_absolute_equation, verify_absolute_equation, verify_solution_set,
    verify_special_angle,
};

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn affine(slope: i64, constant: i64) -> AbsoluteAffine {
    AbsoluteAffine {
        slope: q(slope, 1),
        constant: q(constant, 1),
    }
}

#[test]
fn source_bound_root_certificate_rejects_incomplete_and_forged_sets() {
    let ctx = EvaluationContext::default();
    let variable = Symbol::new("x").unwrap();
    let x = Expr::from(variable.clone());
    let right = DomainExpression::from_expr(Expr::from(0), 64).unwrap();
    for a in 3..=12 {
        let left = DomainExpression::from_expr(
            Expr::add([Expr::pow(x.clone(), Expr::from(2)), Expr::from(-a * a)]),
            64,
        )
        .unwrap();
        let domain = ScalarDomain::Real;
        let cert = certify_solution_set(&ctx, &left, &right, &variable, domain, &[]).unwrap();
        let expected = [-a, a].map(|n| {
            PolynomialRootMultiplicity::new(ExactQuadraticRoot::Rational(q(n, 1)), 1)
        });
        assert_eq!(
            cert.solutions,
            CertifiedSolutionSet::Finite(expected.to_vec())
        );
        let verify = |claim| verify_solution_set(&ctx, &left, &right, &variable, domain, claim);
        assert_eq!(verify(&cert), Ok(true));

        let mut missing = cert.clone();
        missing.solutions = CertifiedSolutionSet::Finite(vec![expected[0].clone()]);
        assert_eq!(verify(&missing), Ok(false));
        let mut extra = cert.clone();
        extra.solutions = CertifiedSolutionSet::Finite(vec![
            expected[0].clone(),
            expected[1].clone(),
            PolynomialRootMultiplicity::new(ExactQuadraticRoot::Rational(q(0, 1)), 1),
        ]);
        assert_eq!(verify(&extra), Ok(false));
        let mut forged = cert.clone();
        forged.factors.clear();
        assert_eq!(verify(&forged), Ok(false));

        let restricted = DomainExpression::with_conditions(
            left.expression().clone(),
            [Condition::NonZero(Expr::add([x.clone(), Expr::from(-a)]))],
            64,
            8,
        )
        .unwrap();
        assert_eq!(
            verify_solution_set(&ctx, &restricted, &right, &variable, domain, &cert),
            Ok(false)
        );
    }
}

#[test]
fn absolute_partitions_reject_missing_cells_and_false_signs() {
    let context = EvaluationContext::default();
    let variable = Symbol::new("x").unwrap();
    let equation = AbsoluteEquation {
        domain: ScalarDomain::Real,
        left: AbsoluteExpression {
            variable: variable.clone(),
            base: affine(1, 0),
            layers: vec![affine(1, -5), affine(1, 0)],
        },
        right: AbsoluteExpression {
            variable,
            base: affine(0, 2),
            layers: vec![],
        },
    };
    let result = solve_absolute_equation(&context, &equation).unwrap();
    let expected = [-7, -3, 3, 7].map(|value| (Some(q(value, 1)), Some(q(value, 1))));
    let bounds: Vec<_> = result
        .solutions
        .iter()
        .map(|interval| (interval.lower.clone(), interval.upper.clone()))
        .collect();
    assert_eq!(bounds, expected);
    let verify = |claim| verify_absolute_equation(&context, &equation, claim);
    assert_eq!(verify(&result), Ok(true));
    let mut missing = result.clone();
    missing.cases.remove(1);
    assert_eq!(verify(&missing), Ok(false));
    let mut wrong_sign = result.clone();
    wrong_sign.cases[0].signs[0] = AbsoluteSign::Nonnegative;
    assert_eq!(verify(&wrong_sign), Ok(false));
    let mut incomplete = result;
    incomplete.solutions.pop();
    assert_eq!(verify(&incomplete), Ok(false));
}

#[test]
fn periodic_value_evidence_is_bound_to_angle_and_signed_branch() {
    let domain = ScalarDomain::Real;
    let limits = TrigLimits::default();
    for k in -5..=5 {
        let angle = PiMultiple::new(q(8 * k - 1, 4));
        let cert = evaluate_special_angle(&angle, TrigFunction::Sin, domain, limits).unwrap();
        let minimal_polynomial = RationalPolynomial::new(vec![q(-1, 2), q(0, 1), q(1, 1)]);
        assert_eq!(
            cert.outcome,
            SpecialAngleOutcome::Exact(ExactQuadraticRoot::Algebraic {
                minimal_polynomial: minimal_polynomial.clone(),
                branch: QuadraticBranch::LowerReal,
            })
        );
        let verify = |claim| verify_special_angle(&angle, TrigFunction::Sin, domain, claim, limits);
        assert_eq!(verify(&cert), Ok(true));
        let mut branch = cert.clone();
        branch.outcome = SpecialAngleOutcome::Exact(ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch: QuadraticBranch::UpperReal,
        });
        assert_eq!(verify(&branch), Ok(false));
        let mut tick = cert.clone();
        tick.tick = (tick.tick + 12) % 24;
        assert_eq!(verify(&tick), Ok(false));
        let mut period = cert;
        period.angle = PiMultiple::new(q(8 * k + 7, 4));
        assert_eq!(verify(&period), Ok(false));
    }
}

#[test]
fn polynomial_primitives_match_independent_coefficient_oracle() {
    let context = EvaluationContext::default();
    for a in -8..=8 {
        let source = RationalPolynomial::new(vec![q(a, 3), q(5, 7), q(-3, 2)]);
        let result = context.integrate_polynomial(&source).unwrap();
        let expected = RationalPolynomial::new(vec![q(0, 1), q(a, 3), q(5, 14), q(-1, 2)]);
        assert_eq!(result.primitive(), &expected);
        assert_eq!(context.verify_polynomial_primitive(&result), Ok(()));
        assert_eq!(context.integrate_polynomial(&source).unwrap(), result);
    }
}

#[test]
fn exact_resource_boundary_admits_at_limit_and_rejects_beyond() {
    let source = RationalPolynomial::new(vec![q(1, 1), q(2, 1), q(3, 1)]);
    let at_limit = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 3,
        ..ResourceLimits::default()
    });
    assert!(at_limit.integrate_polynomial(&source).is_ok());
    let too_small = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        too_small.integrate_polynomial(&source),
        Err(IntegrationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::PolynomialDegree
        )))
    );

    let angle = PiMultiple::new(q(8, 1));
    assert!(evaluate_special_angle(
        &angle,
        TrigFunction::Cos,
        ScalarDomain::Real,
        TrigLimits { max_input_bits: 4 },
    )
    .is_ok());
    assert_eq!(
        evaluate_special_angle(
            &angle,
            TrigFunction::Cos,
            ScalarDomain::Real,
            TrigLimits { max_input_bits: 3 },
        ),
        Err(TrigError::ScalarLimit)
    );
}
