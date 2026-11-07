use mathema::{Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    DerivativeRuleId, EvaluationContext, EvaluationError, Relation, ResourceLimitKind,
    ResourceLimits, RationalPolynomial, SymbolicDifferentiationError,
    verify_symbolic_derivative,
};

fn symbol(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}
fn x() -> Expr {
    Expr::from(symbol("x"))
}
fn y() -> Expr {
    Expr::from(symbol("y"))
}
fn derivative(expr: &Expr, variable: &str) -> Expr {
    EvaluationContext::default()
        .differentiate_expression(expr, &symbol(variable))
        .unwrap()
        .derivative()
        .clone()
}

#[test]
fn constants_and_other_variables_are_exact_constants() {
    assert_eq!(derivative(&Expr::from(17), "x"), Expr::from(0));
    let fraction = Rational::new(Integer::from(3), Integer::from(7)).unwrap();
    assert_eq!(derivative(&Expr::from(fraction), "x"), Expr::from(0));
    assert_eq!(derivative(&x(), "x"), Expr::from(1));
    assert_eq!(derivative(&y(), "x"), Expr::from(0));
    assert_eq!(derivative(&x(), "y"), Expr::from(0));
}

#[test]
fn sum_and_product_rules_respect_other_symbols_as_parameters() {
    let expr = Expr::add([
        Expr::pow(x(), 2.into()),
        Expr::mul([x(), y()]),
        Expr::from(5),
    ]);
    let expected = Expr::add([Expr::mul([2.into(), x()]), y()]);
    assert_eq!(derivative(&expr, "x"), expected);
    assert_eq!(derivative(&expr, "y"), x());
    let repeated = Expr::mul([x(), x()]);
    assert_eq!(derivative(&repeated, "x"), Expr::add([x(), x()]));
}

#[test]
fn positive_integer_power_and_nested_chain_rule_are_exact() {
    let base = Expr::add([x(), y()]);
    let expr = Expr::pow(base.clone(), 3.into());
    let expected = Expr::mul([3.into(), Expr::pow(base, 2.into())]);
    assert_eq!(derivative(&expr, "x"), expected);
    assert_eq!(derivative(&expr, "y"), expected);
    let cubic = Expr::pow(x(), 3.into());
    assert_eq!(
        derivative(&cubic, "x"),
        Expr::mul([3.into(), Expr::pow(x(), 2.into())])
    );
}

#[test]
fn differentiation_preserves_input_and_is_deterministic() {
    let original = Expr::mul([
        Expr::add([x(), y()]),
        Expr::pow(Expr::add([x(), 1.into()]), 3.into()),
    ]);
    let saved = original.clone();
    let ctx = EvaluationContext::default();
    let a = ctx
        .differentiate_expression(&original, &symbol("x"))
        .unwrap();
    let b = ctx
        .differentiate_expression(&original, &symbol("x"))
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(original, saved);
    assert_eq!(verify_symbolic_derivative(&a), Ok(()));
    assert_eq!(a.steps().last().unwrap().derivative(), a.derivative());
    for step in a.steps() {
        assert_eq!(step.relation(), Relation::DerivativeOf);
        assert!(step.rule().stable_id().starts_with("calculus."));
    }
}

#[test]
fn rejects_zero_negative_fractional_and_symbolic_powers() {
    let ctx = EvaluationContext::default();
    for power in [
        Expr::from(0),
        Expr::from(-1),
        Expr::from(Rational::new(Integer::from(1), Integer::from(2)).unwrap()),
        y(),
    ] {
        let expr = Expr::pow(x(), power);
        assert_eq!(
            ctx.differentiate_expression(&expr, &symbol("x")),
            Err(EvaluationError::SymbolicDifferentiation(
                SymbolicDifferentiationError::UnsupportedPower
            ))
        );
    }
}

#[test]
fn positive_power_limit_is_distinct_from_unsupported_domain() {
    let ctx = EvaluationContext::new(ResourceLimits {
        max_symbolic_power: 2,
        ..ResourceLimits::default()
    });
    let cubic = Expr::pow(x(), 3.into());
    assert_eq!(
        ctx.differentiate_expression(&cubic, &symbol("x")),
        Err(EvaluationError::Limit(ResourceLimitKind::SymbolicPower))
    );
    assert!(
        ctx.differentiate_expression(&Expr::pow(x(), 2.into()), &symbol("x"))
            .is_ok()
    );
}

#[test]
fn work_and_node_limits_are_enforced_without_partial_answers() {
    let expr = Expr::mul([x(), y()]);
    let no_work = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        no_work.differentiate_expression(&expr, &symbol("x")),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    let tiny = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        tiny.differentiate_expression(&expr, &symbol("x")),
        Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes))
    );
}

#[test]
fn power_rule_and_product_rule_have_stable_rule_ids() {
    assert_eq!(
        DerivativeRuleId::PositiveIntegerPower.stable_id(),
        "calculus.positive_integer_power"
    );
    assert_eq!(DerivativeRuleId::Product.stable_id(), "calculus.product");
}

#[test]
fn symbolic_derivative_agrees_with_independent_polynomial_engine_for_625_cases() {
    let variable = symbol("x");
    let ctx = EvaluationContext::default();
    let mut checked = 0;
    for cubic in -2..=2 {
        for quadratic in -2..=2 {
            for linear in -2..=2 {
                for constant in -2..=2 {
                    let polynomial = RationalPolynomial::new(
                        [constant, linear, quadratic, cubic]
                            .into_iter()
                            .map(|value| Rational::from(Integer::from(value)))
                            .collect(),
                    );
                    let expression = polynomial.to_expr(&variable);
                    let result = ctx
                        .differentiate_expression(&expression, &variable)
                        .unwrap();
                    let interpreted = RationalPolynomial::try_from_expr(
                        result.derivative(),
                        &variable,
                        32,
                    )
                    .unwrap();
                    assert_eq!(
                        interpreted,
                        polynomial.differentiate(),
                        "coefficients: {constant}, {linear}, {quadratic}, {cubic}"
                    );
                    assert_eq!(verify_symbolic_derivative(&result), Ok(()));
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 625);
}
