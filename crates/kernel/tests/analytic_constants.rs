use mathema::{
    Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol,
};
use mathematishia_kernel::{
    AnalyticConstantError, AnalyticConstantLimit, AnalyticConstantLimits,
    AnalyticConstantOutcome, AnalyticConstantRule, AnalyticConstantUnsupported,
    rewrite_analytic_constant_once, verify_analytic_constant_rewrite,
};

fn symbol(name: &str) -> Expr {
    Symbol::new(name).unwrap().into()
}

fn source(expression: Expr, domain: ScalarDomain) -> DomainExpression {
    DomainExpression::from_expr_in_domain(expression, domain, 4_096, 256).unwrap()
}

fn rewrite(
    input: &DomainExpression,
    domain: ScalarDomain,
) -> mathematishia_kernel::AnalyticConstantRewrite {
    let AnalyticConstantOutcome::Exact(step) =
        rewrite_analytic_constant_once(input, domain, AnalyticConstantLimits::default()).unwrap()
    else {
        panic!("expected exact rewrite");
    };
    assert_eq!(
        verify_analytic_constant_rewrite(input, &step, AnalyticConstantLimits::default()),
        Ok(true)
    );
    step
}

#[test]
fn all_five_rules_are_exact_and_stable() {
    let cases = [
        (Expr::sin(0.into()), Expr::from(0), AnalyticConstantRule::SineZero, "analytic.sin_zero"),
        (Expr::cos(0.into()), Expr::from(1), AnalyticConstantRule::CosineZero, "analytic.cos_zero"),
        (Expr::tan(0.into()), Expr::from(0), AnalyticConstantRule::TangentZero, "analytic.tan_zero"),
        (Expr::exp(0.into()), Expr::from(1), AnalyticConstantRule::ExponentialZero, "analytic.exp_zero"),
        (Expr::log(1.into()), Expr::from(0), AnalyticConstantRule::LogarithmOne, "analytic.log_one"),
    ];
    for domain in [ScalarDomain::Real, ScalarDomain::Complex] {
        for (from, expected, rule, stable_id) in &cases {
            let input = source(from.clone(), domain);
            let step = rewrite(&input, domain);
            assert_eq!(step.source, input);
            assert_eq!(step.result.expression(), expected);
            assert_eq!(step.rule, *rule);
            assert_eq!(step.rule.stable_id(), *stable_id);
            assert_eq!(step.path, Vec::<usize>::new());
            // No conditions are removed even when a rational fact is provable.
            assert!(input.conditions().iter().all(|c| step.result.conditions().contains(c)));
            let same = rewrite(&input, domain);
            assert_eq!(same, step);
        }
    }
}

#[test]
fn domain_obligations_are_retained_without_unproved_discharges() {
    let real_log = rewrite(&source(Expr::log(1.into()), ScalarDomain::Real), ScalarDomain::Real);
    assert_eq!(real_log.result.conditions(), &[Condition::Positive(Expr::from(1))]);

    let complex_log = rewrite(
        &source(Expr::log(1.into()), ScalarDomain::Complex), ScalarDomain::Complex
    );
    assert_eq!(complex_log.result.conditions(), &[Condition::NonZero(Expr::from(1))]);

    let tangent = rewrite(&source(Expr::tan(0.into()), ScalarDomain::Real), ScalarDomain::Real);
    assert_eq!(
        tangent.result.conditions(),
        &[Condition::NonZero(Expr::cos(Expr::from(0)))]
    );
}

#[test]
fn nested_rewriting_is_one_step_and_preorder() {
    let initial = source(Expr::sin(Expr::log(1.into())), ScalarDomain::Real);
    let first = rewrite(&initial, ScalarDomain::Real);
    assert_eq!(first.rule, AnalyticConstantRule::LogarithmOne);
    assert_eq!(first.path, vec![0]);
    assert_eq!(first.result.expression(), &Expr::sin(0.into()));
    assert_eq!(first.result.conditions(), &[Condition::Positive(Expr::from(1))]);

    let second = rewrite(&first.result, ScalarDomain::Real);
    assert_eq!(second.rule, AnalyticConstantRule::SineZero);
    assert_eq!(second.path, Vec::<usize>::new());
    assert_eq!(second.result.expression(), &Expr::from(0));
    assert_eq!(second.result.conditions(), first.result.conditions());
    assert_eq!(
        rewrite_analytic_constant_once(
            &second.result, ScalarDomain::Real, AnalyticConstantLimits::default()
        ),
        Ok(AnalyticConstantOutcome::NoRule {
            retained_conditions: vec![Condition::Positive(Expr::from(1))]
        })
    );
}

#[test]
fn deterministic_first_match_uses_existing_canonical_child_order() {
    let expr = Expr::mul([Expr::sin(0.into()), Expr::cos(0.into())]);
    let input = source(expr, ScalarDomain::Complex);
    let step = rewrite(&input, ScalarDomain::Complex);
    assert_eq!(step.rule, AnalyticConstantRule::CosineZero);
    assert_eq!(step.path, vec![0]);
    assert_eq!(step.result.expression(), &Expr::sin(0.into()));
    let next = rewrite(&step.result, ScalarDomain::Complex);
    assert_eq!(next.rule, AnalyticConstantRule::SineZero);
    assert_eq!(next.result.expression(), &Expr::from(0));
}

#[test]
fn output_retains_both_history_and_new_obligations() {
    let x = symbol("x");
    let y = symbol("y");
    let historical = DomainExpression::with_conditions(
        0.into(),
        [Condition::NonZero(x.clone()), Condition::Positive(y.clone())],
        128, 16
    ).unwrap();
    let call = source(Expr::sin(0.into()), ScalarDomain::Real);
    let input = DomainExpression::add([historical, call], 16).unwrap();
    assert_eq!(input.expression(), &Expr::sin(0.into()));
    let step = rewrite(&input, ScalarDomain::Real);
    assert_eq!(step.result.expression(), &Expr::from(0));
    assert_eq!(
        step.result.conditions(),
        &[Condition::NonZero(x), Condition::Positive(y)]
    );

    let nested = source(Expr::tan(Expr::exp(0.into())), ScalarDomain::Real);
    let changed = rewrite(&nested, ScalarDomain::Real);
    assert_eq!(changed.result.expression(), &Expr::tan(1.into()));
    assert_eq!(
        changed.result.conditions(),
        &[
            Condition::NonZero(Expr::cos(Expr::exp(0.into()))),
            Condition::NonZero(Expr::cos(1.into())),
        ]
    );
}

#[test]
fn no_rule_does_not_invent_analytic_approximations() {
    let x = symbol("x");
    for expr in [
        Expr::sin(x.clone()),
        Expr::log(2.into()),
        Expr::cos(1.into()),
        Expr::exp(1.into()),
        Expr::tan(x.clone()),
        Expr::from(7),
    ] {
        let input = source(expr, ScalarDomain::Complex);
        assert_eq!(
            rewrite_analytic_constant_once(
                &input, ScalarDomain::Complex, AnalyticConstantLimits::default()
            ),
            Ok(AnalyticConstantOutcome::NoRule {
                retained_conditions: input.conditions().to_vec()
            })
        );
    }
    // The complex logarithm is branch-sensitive; only log(1) is simplified.
    let input = source(Expr::log(Expr::exp(x)), ScalarDomain::Complex);
    assert!(matches!(
        rewrite_analytic_constant_once(
            &input, ScalarDomain::Complex, AnalyticConstantLimits::default()
        ),
        Ok(AnalyticConstantOutcome::NoRule { .. })
    ));
}

#[test]
fn excluded_exact_domain_points_are_refused() {
    for (expr, domain, condition) in [
        (Expr::log(0.into()), ScalarDomain::Real, Condition::Positive(0.into())),
        (Expr::log((-1).into()), ScalarDomain::Real, Condition::Positive((-1).into())),
        (Expr::log(0.into()), ScalarDomain::Complex, Condition::NonZero(0.into())),
        (Expr::pow(0.into(), 0.into()), ScalarDomain::Real, Condition::NonZero(0.into())),
    ] {
        let input = source(expr, domain);
        assert_eq!(
            rewrite_analytic_constant_once(&input, domain, AnalyticConstantLimits::default()),
            Err(AnalyticConstantError::ContradictedCondition(condition))
        );
    }
    // Inner exact identity makes the enclosing real logarithm undefined.
    let invalid_after = source(Expr::log(Expr::sin(0.into())), ScalarDomain::Real);
    assert_eq!(
        rewrite_analytic_constant_once(
            &invalid_after, ScalarDomain::Real, AnalyticConstantLimits::default()
        ),
        Err(AnalyticConstantError::ContradictedCondition(Condition::Positive(Expr::from(0))))
    );
    let complex_negative = source(Expr::log((-1).into()), ScalarDomain::Complex);
    assert!(matches!(
        rewrite_analytic_constant_once(
            &complex_negative, ScalarDomain::Complex, AnalyticConstantLimits::default()
        ),
        Ok(AnalyticConstantOutcome::NoRule { .. })
    ));
}

#[test]
fn unsupported_domains_functions_and_powers_fail_closed() {
    let valid = source(Expr::sin(0.into()), ScalarDomain::Real);
    for domain in [ScalarDomain::Integer, ScalarDomain::Rational] {
        assert_eq!(
            rewrite_analytic_constant_once(&valid, domain, AnalyticConstantLimits::default()),
            Ok(AnalyticConstantOutcome::Unsupported(
                AnalyticConstantUnsupported::Domain(domain)
            ))
        );
    }
    let custom = Expr::function(Symbol::new("f").unwrap(), symbol("x"));
    let with_custom_history = DomainExpression::with_conditions(
        0.into(), [Condition::NonZero(custom)], 100, 4
    ).unwrap();
    assert_eq!(
        rewrite_analytic_constant_once(
            &with_custom_history, ScalarDomain::Real, AnalyticConstantLimits::default()
        ),
        Ok(AnalyticConstantOutcome::Unsupported(
            AnalyticConstantUnsupported::Function(Symbol::new("f").unwrap())
        ))
    );
    let symbolic_power = Expr::pow(symbol("x"), symbol("n"));
    let input = DomainExpression::with_conditions(
        Expr::sin(0.into()), [Condition::NonZero(symbolic_power)], 100, 4
    ).unwrap_err();
    assert_eq!(input, mathema::DefinednessError::UnsupportedFunction);
}

#[test]
fn forged_rule_target_path_source_and_conditions_are_rejected() {
    let input = source(Expr::tan(0.into()), ScalarDomain::Real);
    let genuine = rewrite(&input, ScalarDomain::Real);
    let mut forged = genuine.clone();

    forged.rule = AnalyticConstantRule::LogarithmOne;
    assert_eq!(
        verify_analytic_constant_rewrite(&input, &forged, AnalyticConstantLimits::default()),
        Ok(false)
    );
    forged = genuine.clone();
    forged.path = vec![0];
    assert_eq!(
        verify_analytic_constant_rewrite(&input, &forged, AnalyticConstantLimits::default()),
        Ok(false)
    );
    forged = genuine.clone();
    forged.result = source(Expr::from(1), ScalarDomain::Real);
    assert_eq!(
        verify_analytic_constant_rewrite(&input, &forged, AnalyticConstantLimits::default()),
        Ok(false)
    );
    forged = genuine.clone();
    forged.result = source(Expr::from(0), ScalarDomain::Real); // dropped cos(0) exclusion
    assert_eq!(
        verify_analytic_constant_rewrite(&input, &forged, AnalyticConstantLimits::default()),
        Ok(false)
    );
    forged = genuine.clone();
    forged.result = DomainExpression::with_conditions(
        Expr::from(0), [
            Condition::NonZero(Expr::cos(0.into())),
            Condition::Positive(symbol("x"))
        ], 128, 4
    ).unwrap(); // extraneous narrowing condition
    assert_eq!(
        verify_analytic_constant_rewrite(&input, &forged, AnalyticConstantLimits::default()),
        Ok(false)
    );
    forged = genuine.clone();
    forged.source = source(Expr::cos(0.into()), ScalarDomain::Real);
    assert_eq!(
        verify_analytic_constant_rewrite(&input, &forged, AnalyticConstantLimits::default()),
        Ok(false)
    );
    forged = genuine.clone();
    forged.domain = ScalarDomain::Complex;
    assert_eq!(
        verify_analytic_constant_rewrite(&input, &forged, AnalyticConstantLimits::default()),
        Ok(false)
    );
}

#[test]
fn deterministic_resource_limits_cover_source_and_historical_conditions() {
    let x = symbol("x");
    let input = source(Expr::sin(0.into()), ScalarDomain::Real);
    let limits = AnalyticConstantLimits { max_nodes: 1, ..Default::default() };
    assert_eq!(
        rewrite_analytic_constant_once(&input, ScalarDomain::Real, limits),
        Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Nodes))
    );

    let deep = source(Expr::sin(Expr::sin(Expr::sin(x.clone()))), ScalarDomain::Real);
    let limits = AnalyticConstantLimits { max_depth: 1, ..Default::default() };
    assert_eq!(
        rewrite_analytic_constant_once(&deep, ScalarDomain::Real, limits),
        Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Depth))
    );
    let huge = "1".to_owned() + &"0".repeat(100);
    let huge: Integer = huge.parse().unwrap();
    let input = source(Expr::sin(Expr::from(huge)), ScalarDomain::Real);
    let limits = AnalyticConstantLimits { max_scalar_bits: 8, ..Default::default() };
    assert_eq!(
        rewrite_analytic_constant_once(&input, ScalarDomain::Real, limits),
        Err(AnalyticConstantError::Limit(AnalyticConstantLimit::ScalarBits))
    );

    let historical = DomainExpression::with_conditions(
        0.into(), [
            Condition::NonZero(x.clone()),
            Condition::Positive(symbol("y")),
        ], 100, 8
    ).unwrap();
    let input = DomainExpression::add([historical, source(Expr::sin(0.into()), ScalarDomain::Real)], 8).unwrap();
    let limits = AnalyticConstantLimits { max_conditions: 1, ..Default::default() };
    assert_eq!(
        rewrite_analytic_constant_once(&input, ScalarDomain::Real, limits),
        Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Conditions))
    );
    let limits = AnalyticConstantLimits { max_work: 1, ..Default::default() };
    assert_eq!(
        rewrite_analytic_constant_once(&input, ScalarDomain::Real, limits),
        Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Work))
    );
    let limits = AnalyticConstantLimits { max_nodes: 4_097, ..Default::default() };
    assert_eq!(
        rewrite_analytic_constant_once(&input, ScalarDomain::Real, limits),
        Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Configuration))
    );
}

#[test]
fn bounded_rational_inputs_do_not_trigger_decimal_approximation() {
    let r = Rational::new(Integer::from(3), Integer::from(2)).unwrap();
    let input = source(Expr::sin(Expr::from(r)), ScalarDomain::Real);
    assert!(matches!(
        rewrite_analytic_constant_once(
            &input, ScalarDomain::Real, AnalyticConstantLimits::default()
        ),
        Ok(AnalyticConstantOutcome::NoRule { .. })
    ));
}
