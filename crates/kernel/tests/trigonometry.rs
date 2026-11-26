use mathema::{Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    AnalyticConstantLimits, AnalyticConstantOutcome, AnalyticConstantRule, CheckedBy,
    EvaluationContext, ExactAnswer, ExactQuadraticRoot, PiMultiple, ProblemOperation,
    ProblemOutcome, ProblemRequest, ProblemRestriction, ProblemStatement, QuadraticBranch,
    RationalPolynomial, SpecialAngleCertificate, SpecialAngleOutcome, TrigEquationUnsupported,
    TrigError, TrigFunction, TrigLimits, dispatch_problem, evaluate_special_angle,
    partial_trig_zeros, rewrite_analytic_constant_once, verify_analytic_constant_rewrite,
    verify_special_angle,
};

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn angle(n: i64, d: i64) -> PiMultiple {
    PiMultiple::new(q(n, d))
}

fn value(n: i64, d: i64) -> SpecialAngleOutcome {
    SpecialAngleOutcome::Exact(ExactQuadraticRoot::Rational(q(n, d)))
}

fn surd(n: i64, d: i64, branch: QuadraticBranch) -> SpecialAngleOutcome {
    SpecialAngleOutcome::Exact(ExactQuadraticRoot::Algebraic {
        minimal_polynomial: RationalPolynomial::new(vec![q(-n, d), q(0, 1), q(1, 1)]),
        branch,
    })
}

fn evaluate(function: TrigFunction, n: i64, d: i64) -> SpecialAngleCertificate {
    evaluate_special_angle(
        &angle(n, d),
        function,
        ScalarDomain::Real,
        TrigLimits::default(),
    )
    .unwrap()
}

fn verify(
    function: TrigFunction,
    n: i64,
    d: i64,
    certificate: &SpecialAngleCertificate,
) -> Result<bool, TrigError> {
    verify_special_angle(
        &angle(n, d),
        function,
        ScalarDomain::Real,
        certificate,
        TrigLimits::default(),
    )
}

#[test]
fn familiar_unit_circle_questions_have_exact_answers() {
    use QuadraticBranch::UpperReal;
    use TrigFunction::{Cos, Sin, Tan};

    let cases = [
        (Sin, 0, 1, value(0, 1)),
        (Cos, 0, 1, value(1, 1)),
        (Tan, 0, 1, value(0, 1)),
        (Sin, 1, 6, value(1, 2)),
        (Cos, 1, 4, surd(1, 2, UpperReal)),
        (Sin, 1, 3, surd(3, 4, UpperReal)),
        (Cos, 1, 3, value(1, 2)),
        (Sin, 1, 2, value(1, 1)),
        (Cos, 1, 2, value(0, 1)),
        (Tan, 1, 6, surd(1, 3, UpperReal)),
        (Tan, 1, 4, value(1, 1)),
        (Tan, 1, 3, surd(3, 1, UpperReal)),
    ];
    for (function, n, d, expected) in cases {
        let certificate = evaluate(function, n, d);
        assert_eq!(certificate.outcome, expected);
        assert!(verify(function, n, d, &certificate).unwrap());
        assert_eq!(certificate, evaluate(function, n, d));
        assert_eq!(certificate.domain, ScalarDomain::Real);
    }
}

#[test]
fn quadrant_signs_and_full_turns_are_exact() {
    use QuadraticBranch::{LowerReal, UpperReal};
    use TrigFunction::{Cos, Sin, Tan};

    let cases = [
        (Sin, 5, 6, value(1, 2)),
        (Cos, 5, 6, surd(3, 4, LowerReal)),
        (Sin, 7, 6, value(-1, 2)),
        (Cos, 7, 6, surd(3, 4, LowerReal)),
        (Sin, 11, 6, value(-1, 2)),
        (Cos, 11, 6, surd(3, 4, UpperReal)),
        (Tan, 3, 4, value(-1, 1)),
        (Tan, 5, 4, value(1, 1)),
        (Tan, 7, 4, value(-1, 1)),
        (Sin, -1, 6, value(-1, 2)),
        (Cos, 13, 6, surd(3, 4, UpperReal)),
        (Sin, -9, 2, value(-1, 1)),
    ];
    for (function, n, d, expected) in cases {
        assert_eq!(evaluate(function, n, d).outcome, expected);
    }

    for tick in [0, 2, 3, 4, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20, 21, 22] {
        for function in [Sin, Cos, Tan] {
            let original = evaluate(function, tick, 12);
            for turns in [-100, -1, 1, 100] {
                let translated = evaluate(function, tick + 24 * turns, 12);
                assert_eq!(original.tick, translated.tick);
                assert_eq!(original.outcome, translated.outcome);
                assert_ne!(original.angle, translated.angle);
            }
        }
    }
}

#[test]
fn tangent_poles_are_undefined_at_every_supported_translation() {
    for n in [-7, -3, -1, 1, 3, 5, 101] {
        let certificate = evaluate(TrigFunction::Tan, n, 2);
        assert_eq!(
            certificate.outcome,
            SpecialAngleOutcome::UndefinedTangentPole
        );
        assert!(verify(TrigFunction::Tan, n, 2, &certificate).unwrap());
        assert_eq!(evaluate(TrigFunction::Cos, n, 2).outcome, value(0, 1));
    }
}

#[test]
fn unsupported_angles_and_domains_are_explicit() {
    for (n, d) in [(1, 12), (5, 12), (1, 5), (1, 7), (-1, 10)] {
        assert_eq!(
            evaluate_special_angle(
                &angle(n, d),
                TrigFunction::Sin,
                ScalarDomain::Real,
                TrigLimits::default(),
            ),
            Err(TrigError::UnsupportedAngle)
        );
    }
    for domain in [ScalarDomain::Rational, ScalarDomain::Complex] {
        assert_eq!(
            evaluate_special_angle(
                &angle(0, 1),
                TrigFunction::Sin,
                domain,
                TrigLimits::default()
            ),
            Err(TrigError::UnsupportedDomain(domain))
        );
    }
}

#[test]
fn forged_source_rule_tick_value_and_branch_are_rejected() {
    use QuadraticBranch::LowerReal;
    use TrigFunction::{Cos, Sin, Tan};

    let original = evaluate(Sin, 1, 4);
    assert!(!verify(Sin, 9, 4, &original).unwrap());
    assert!(!verify(Cos, 1, 4, &original).unwrap());
    let mut forged = original.clone();
    forged.tick = 9;
    assert!(!verify(Sin, 1, 4, &forged).unwrap());

    forged = original.clone();
    forged.domain = ScalarDomain::Complex;
    assert!(!verify(Sin, 1, 4, &forged).unwrap());

    forged = original.clone();
    forged.outcome = surd(1, 2, LowerReal);
    assert!(!verify(Sin, 1, 4, &forged).unwrap());

    forged.outcome = surd(3, 4, LowerReal);
    assert!(!verify(Sin, 1, 4, &forged).unwrap());

    forged.outcome = value(7, 10);
    assert!(!verify(Sin, 1, 4, &forged).unwrap());

    forged = evaluate(Tan, 1, 2);
    forged.outcome = value(0, 1);
    assert!(!verify(Tan, 1, 2, &forged).unwrap());

    forged = evaluate(Tan, 1, 4);
    forged.outcome = SpecialAngleOutcome::UndefinedTangentPole;
    assert!(!verify(Tan, 1, 4, &forged).unwrap());

    // A reducible polynomial must not masquerade as an irrational root.
    forged = evaluate(Sin, 1, 6);
    forged.outcome = surd(1, 4, QuadraticBranch::UpperReal);
    assert!(!verify(Sin, 1, 6, &forged).unwrap());
}

#[test]
fn resource_bounds_apply_to_original_inputs_and_forged_evidence() {
    let small_limits = TrigLimits { max_input_bits: 8 };
    assert_eq!(
        evaluate_special_angle(
            &angle(257, 1),
            TrigFunction::Sin,
            ScalarDomain::Real,
            small_limits,
        ),
        Err(TrigError::ScalarLimit)
    );
    for max_input_bits in [0, 4097] {
        assert_eq!(
            evaluate_special_angle(
                &angle(0, 1),
                TrigFunction::Sin,
                ScalarDomain::Real,
                TrigLimits { max_input_bits },
            ),
            Err(TrigError::InvalidLimits)
        );
    }
    let large = "9".repeat(1000).parse::<Integer>().unwrap();
    let large_angle = PiMultiple::new(Rational::from(large));
    let certificate = evaluate_special_angle(
        &large_angle,
        TrigFunction::Cos,
        ScalarDomain::Real,
        TrigLimits::default(),
    )
    .unwrap();
    assert_eq!(certificate.angle, large_angle);
    assert_eq!(certificate.outcome, value(-1, 1));

    let enormous = "9".repeat(1300).parse::<Integer>().unwrap();
    let huge_angle = PiMultiple::new(Rational::from(enormous));
    assert_eq!(
        evaluate_special_angle(
            &huge_angle,
            TrigFunction::Sin,
            ScalarDomain::Real,
            TrigLimits::default(),
        ),
        Err(TrigError::ScalarLimit)
    );
    let mut forged = evaluate(TrigFunction::Sin, 0, 1);
    forged.outcome = value(1_000_000, 1);
    assert_eq!(
        verify(TrigFunction::Sin, 0, 1, &forged),
        Err(TrigError::ScalarLimit)
    );
}

#[test]
fn sine_and_cosine_zero_equations_return_only_checked_partial_facts() {
    for (function, expected) in [
        (TrigFunction::Sin, [angle(0, 1), angle(1, 1)]),
        (TrigFunction::Cos, [angle(1, 2), angle(3, 2)]),
    ] {
        let partial =
            partial_trig_zeros(function, ScalarDomain::Real, TrigLimits::default()).unwrap();
        assert_eq!(partial.function(), function);
        assert_eq!(
            partial.unsupported_reason(),
            TrigEquationUnsupported::IntegerParameterCompletenessUnavailable
        );
        for (certificate, expected_angle) in partial.known_zeros().iter().zip(expected) {
            assert_eq!(certificate.angle, expected_angle);
            assert_eq!(certificate.outcome, value(0, 1));
            assert!(
                verify_special_angle(
                    &expected_angle,
                    function,
                    ScalarDomain::Real,
                    certificate,
                    TrigLimits::default(),
                )
                .unwrap()
            );
        }
    }
    assert_eq!(
        partial_trig_zeros(TrigFunction::Tan, ScalarDomain::Real, TrigLimits::default()),
        Err(TrigError::UnsupportedEquation)
    );
}

fn exact_request(source: DomainExpression) -> ProblemRequest {
    ProblemRequest {
        operation: ProblemOperation::EvaluateExact,
        statement: ProblemStatement::Expression(source),
        variables: vec![],
        domain: ScalarDomain::Real,
    }
}

#[test]
fn existing_constant_dispatch_checks_all_five_identities() {
    let context = EvaluationContext::default();
    for (expr, expected) in [
        (Expr::sin(Expr::from(0)), 0),
        (Expr::cos(Expr::from(0)), 1),
        (Expr::tan(Expr::from(0)), 0),
        (Expr::exp(Expr::from(0)), 1),
        (Expr::log(Expr::from(1)), 0),
    ] {
        let source =
            DomainExpression::from_expr_in_domain(expr, ScalarDomain::Real, 64, 8).unwrap();
        let result = dispatch_problem(&context, &exact_request(source.clone())).unwrap();
        match result.outcome() {
            ProblemOutcome::SolvedExact(ExactAnswer::Rational(answer))
            | ProblemOutcome::SolvedConditional(ExactAnswer::Rational(answer)) => {
                assert_eq!(answer, &q(expected, 1));
            }
            other => panic!("expected checked rational constant, got {other:?}"),
        }
        assert_eq!(result.checked_by(), Some(CheckedBy::AnalyticConstantRules));
        for condition in source.conditions() {
            assert!(
                result
                    .restrictions()
                    .contains(&ProblemRestriction::Condition(condition.clone()))
            );
        }
    }
}

#[test]
fn inherited_restrictions_and_forged_existing_identities_are_checked() {
    let x = Expr::from(Symbol::new("x").unwrap());
    let condition = Condition::NonZero(x.clone());
    let numerator =
        DomainExpression::from_expr_in_domain(Expr::from(0), ScalarDomain::Real, 64, 8).unwrap();
    let denominator = DomainExpression::from_expr_in_domain(x, ScalarDomain::Real, 64, 8).unwrap();
    let argument = numerator.divide(denominator, 8).unwrap();
    let source =
        DomainExpression::function(Symbol::new("sin").unwrap(), argument, ScalarDomain::Real, 8)
            .unwrap();
    let result = dispatch_problem(
        &EvaluationContext::default(),
        &exact_request(source.clone()),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedConditional(ExactAnswer::Rational(q(0, 1)))
    );
    assert!(
        result
            .restrictions()
            .contains(&ProblemRestriction::Condition(condition))
    );

    let limits = AnalyticConstantLimits::default();
    let AnalyticConstantOutcome::Exact(mut proof) =
        rewrite_analytic_constant_once(&source, ScalarDomain::Real, limits).unwrap()
    else {
        panic!("expected a constant identity");
    };
    assert!(verify_analytic_constant_rewrite(&source, ScalarDomain::Real, &proof, limits).unwrap());
    proof.rule = AnalyticConstantRule::CosineZero;
    assert!(
        !verify_analytic_constant_rewrite(&source, ScalarDomain::Real, &proof, limits).unwrap()
    );
    proof.rule = AnalyticConstantRule::SineZero;
    proof.result =
        DomainExpression::from_expr_in_domain(Expr::from(0), ScalarDomain::Real, 64, 8).unwrap();
    assert!(
        !verify_analytic_constant_rewrite(&source, ScalarDomain::Real, &proof, limits).unwrap()
    );
}

#[test]
fn arbitrary_symbols_named_pi_are_not_mathematical_pi() {
    let pi = Expr::from(Symbol::new("pi").unwrap());
    let source =
        DomainExpression::from_expr_in_domain(Expr::sin(pi), ScalarDomain::Real, 64, 8).unwrap();
    let result = dispatch_problem(&EvaluationContext::default(), &exact_request(source)).unwrap();
    assert!(matches!(result.outcome(), ProblemOutcome::Unsupported(_)));
    assert_eq!(result.checked_by(), None);
}
