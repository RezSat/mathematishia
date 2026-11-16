use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol,
};
use mathematishia_kernel::{
    ElementaryIntegrationError as E, ElementaryPrimitiveRule as Rule, EvaluationContext,
    EvaluationError, PolynomialConversionError, ResourceLimitKind, ResourceLimits,
    verify_symbolic_derivative,
};

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}

fn affine(a: Rational, b: Rational, variable: &Symbol) -> Expr {
    Expr::add([
        Expr::mul([Expr::from(a), Expr::from(variable.clone())]),
        Expr::from(b),
    ])
}

fn source(expr: Expr, domain: ScalarDomain) -> DomainExpression {
    DomainExpression::from_expr_in_domain(expr, domain, 256, 32).unwrap()
}

fn verify(
    context: &EvaluationContext,
    input: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
) -> mathematishia_kernel::ElementaryAntiderivative {
    let primitive = context.integrate_elementary_affine(input, variable, domain).unwrap();
    assert_eq!(primitive.source(), input);
    assert_eq!(primitive.domain(), domain);
    assert_eq!(primitive.variable(), variable);
    assert_eq!(primitive.conditions(), input.conditions());
    assert_eq!(primitive.derivative_evidence().derivative(), input.expression());
    assert_eq!(primitive.derivative_evidence().conditions(), input.conditions());
    assert_eq!(primitive.derivative_evidence().domain(), Some(domain));
    verify_symbolic_derivative(primitive.derivative_evidence()).unwrap();
    context
        .verify_elementary_antiderivative(input, variable, domain, &primitive)
        .unwrap();
    primitive
}

#[test]
fn sine_with_fractional_scale_positive_slope_and_offset() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let argument = affine(q(2, 1), q(3, 1), &variable);
    let input = source(
        Expr::mul([Expr::from(q(5, 7)), Expr::sin(argument.clone())]),
        ScalarDomain::Real,
    );
    let result = verify(&ctx, &input, &variable, ScalarDomain::Real);
    assert_eq!(result.rule(), Rule::Sine);
    assert_eq!(result.rule().stable_id(), "calculus.integrate_affine_sine");
    assert_eq!(result.input_scale(), &q(5, 7));
    assert_eq!(result.slope(), &q(2, 1));
    assert_eq!(result.intercept(), &q(3, 1));
    assert_eq!(
        result.primitive().expression(),
        &Expr::mul([Expr::from(q(-5, 14)), Expr::cos(argument)])
    );
}

#[test]
fn cosine_with_negative_fractional_slope_and_negative_scale() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let argument = affine(q(-3, 2), q(-7, 3), &variable);
    let input = source(
        Expr::mul([Expr::from(q(-4, 5)), Expr::cos(argument.clone())]),
        ScalarDomain::Real,
    );
    let result = verify(&ctx, &input, &variable, ScalarDomain::Real);
    assert_eq!(result.rule(), Rule::Cosine);
    assert_eq!(result.rule().stable_id(), "calculus.integrate_affine_cosine");
    assert_eq!(result.slope(), &q(-3, 2));
    assert_eq!(result.intercept(), &q(-7, 3));
    assert_eq!(
        result.primitive().expression(),
        &Expr::mul([Expr::from(q(8, 15)), Expr::sin(argument)])
    );
}

#[test]
fn exponential_with_fractional_slope_and_negative_scale() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let argument = affine(q(1, 4), q(2, 1), &variable);
    let input = source(
        Expr::mul([Expr::from(q(-3, 8)), Expr::exp(argument.clone())]),
        ScalarDomain::Real,
    );
    let result = verify(&ctx, &input, &variable, ScalarDomain::Real);
    assert_eq!(result.rule(), Rule::Exponential);
    assert_eq!(result.rule().stable_id(), "calculus.integrate_affine_exponential");
    assert_eq!(result.slope(), &q(1, 4));
    assert_eq!(
        result.primitive().expression(),
        &Expr::mul([Expr::from(q(-3, 2)), Expr::exp(argument)])
    );
}

#[test]
fn unit_coefficient_and_zero_offset_are_supported() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let input = source(Expr::cos(Expr::from(variable.clone())), ScalarDomain::Real);
    let result = verify(&ctx, &input, &variable, ScalarDomain::Real);
    assert_eq!(result.input_scale(), &q(1, 1));
    assert_eq!(result.slope(), &q(1, 1));
    assert_eq!(result.intercept(), &q(0, 1));
    assert_eq!(
        result.primitive().expression(),
        &Expr::sin(Expr::from(variable))
    );
}

#[test]
fn nested_affine_sums_and_scalar_distribution_verify_exactly() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let symbol = Expr::from(variable.clone());
    // 2(x+3)-x+5 = x+11; input shape is deliberately not expanded.
    let argument = Expr::add([
        Expr::mul([Expr::from(2), Expr::add([symbol.clone(), Expr::from(3)])]),
        Expr::mul([Expr::from(-1), symbol]),
        Expr::from(5),
    ]);
    let input = source(Expr::exp(argument.clone()), ScalarDomain::Real);
    let result = verify(&ctx, &input, &variable, ScalarDomain::Real);
    assert_eq!(result.slope(), &q(1, 1));
    assert_eq!(result.intercept(), &q(11, 1));
    assert_eq!(result.primitive().expression(), &Expr::exp(argument));
}

#[test]
fn complex_domain_entire_functions_are_supported() {
    let ctx = EvaluationContext::default();
    let variable = x();
    for analytic in [Expr::sin, Expr::cos, Expr::exp] {
        let arg = affine(q(-3, 2), q(5, 4), &variable);
        let input = source(analytic(arg), ScalarDomain::Complex);
        let result = verify(&ctx, &input, &variable, ScalarDomain::Complex);
        assert!(result.conditions().is_empty());
    }
    let input = source(Expr::sin(Expr::from(variable.clone())), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_elementary_affine(&input, &variable, ScalarDomain::Rational),
        Err(E::UnsupportedScalarDomain)
    );
    assert_eq!(
        ctx.integrate_elementary_affine(&input, &variable, ScalarDomain::Integer),
        Err(E::UnsupportedScalarDomain)
    );
}

#[test]
fn preserves_history_conditions_and_their_order_without_proving_them() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let symbol = Expr::from(variable.clone());
    let inherited = vec![
        Condition::NonZero(symbol.clone()),
        Condition::Positive(Expr::add([symbol.clone(), Expr::from(7)])),
    ];
    let history = DomainExpression::with_conditions(
        Expr::from(q(1, 1)),
        inherited.clone(),
        256, 32,
    )
    .unwrap();
    let analytic = source(
        Expr::sin(affine(q(2, 3), q(9, 8), &variable)),
        ScalarDomain::Real,
    );
    let input = DomainExpression::multiply([history, analytic], 32).unwrap();
    assert_eq!(input.conditions(), inherited);
    let result = verify(&ctx, &input, &variable, ScalarDomain::Real);
    assert_eq!(result.primitive().conditions(), inherited);
    assert_eq!(
        result.derivative_evidence().conditions(),
        input.conditions()
    );
}

#[test]
fn rejects_log_tan_and_unknown_nested_functions() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let symbol = Expr::from(variable.clone());
    let log = source(Expr::log(symbol.clone()), ScalarDomain::Real);
    let tan = source(Expr::tan(symbol.clone()), ScalarDomain::Complex);
    assert_eq!(
        ctx.integrate_elementary_affine(&log, &variable, ScalarDomain::Real),
        Err(E::UnsupportedFunction)
    );
    assert_eq!(
        ctx.integrate_elementary_affine(&tan, &variable, ScalarDomain::Complex),
        Err(E::UnsupportedFunction)
    );
    assert_eq!(log.conditions(), &[Condition::Positive(symbol.clone())]);
    assert_eq!(
        tan.conditions(),
        &[Condition::NonZero(Expr::cos(symbol.clone()))]
    );
    let nested = source(Expr::sin(Expr::exp(symbol)), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_elementary_affine(&nested, &variable, ScalarDomain::Real),
        Err(E::UnsupportedArgument(PolynomialConversionError::UnsupportedFunction))
    );
}

#[test]
fn refuses_sums_products_and_zeroed_function_products() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let symbol = Expr::from(variable.clone());
    for value in [
        Expr::add([Expr::sin(symbol.clone()), Expr::cos(symbol.clone())]),
        Expr::mul([Expr::sin(symbol.clone()), Expr::cos(symbol.clone())]),
        Expr::from(3),
        Expr::mul([Expr::from(0), Expr::sin(symbol.clone())]),
        Expr::mul([Expr::from(2), symbol]),
    ] {
        let input = source(value, ScalarDomain::Real);
        assert_eq!(
            ctx.integrate_elementary_affine(&input, &variable, ScalarDomain::Real),
            Err(E::UnsupportedForm)
        );
    }
}

#[test]
fn refuses_non_affine_and_other_symbol_arguments() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let symbol = Expr::from(variable.clone());
    let nonlinear = source(
        Expr::cos(Expr::pow(symbol.clone(), Expr::from(2))),
        ScalarDomain::Real,
    );
    assert_eq!(
        ctx.integrate_elementary_affine(&nonlinear, &variable, ScalarDomain::Real),
        Err(E::NonAffineArgument)
    );
    let y = Expr::from(Symbol::new("y").unwrap());
    let mixed = source(Expr::exp(Expr::add([symbol, y])), ScalarDomain::Complex);
    assert_eq!(
        ctx.integrate_elementary_affine(&mixed, &variable, ScalarDomain::Complex),
        Err(E::UnsupportedArgument(PolynomialConversionError::OtherSymbol))
    );
}

#[test]
fn zero_slope_and_unsupported_powers_do_not_get_guessed() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let symbol = Expr::from(variable.clone());
    let constant = source(Expr::sin(Expr::from(q(7, 3))), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_elementary_affine(&constant, &variable, ScalarDomain::Real),
        Err(E::ZeroSlope)
    );
    for power in [-1, 0] {
        let source = source(
            Expr::exp(Expr::pow(symbol.clone(), Expr::from(power))),
            ScalarDomain::Real,
        );
        assert!(!source.conditions().is_empty());
        assert_eq!(
            ctx.integrate_elementary_affine(&source, &variable, ScalarDomain::Real),
            Err(E::UnsupportedArgument(PolynomialConversionError::UnsupportedPower))
        );
    }
    assert_eq!(
        DomainExpression::from_expr_in_domain(
            Expr::sin(Expr::pow(symbol, Expr::from(q(1, 2)))),
            ScalarDomain::Real, 64, 32,
        ),
        Err(DefinednessError::UnsupportedExponent)
    );
}

#[test]
fn rejects_structurally_unverifiable_affine_cancellation() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let symbol = Expr::from(variable.clone());
    let square = Expr::mul([symbol.clone(), symbol.clone()]);
    let cancel_square = Expr::mul([Expr::from(-1), symbol.clone(), symbol.clone()]);
    let affine_by_cancellation = Expr::add([square, cancel_square, symbol]);
    let input = source(Expr::sin(affine_by_cancellation), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_elementary_affine(&input, &variable, ScalarDomain::Real),
        Err(E::UnsupportedNormalization)
    );
}

#[test]
fn large_rational_coefficients_are_not_narrowed() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let huge = Rational::from(Integer::from(u128::MAX));
    let source = source(
        Expr::mul([
            Expr::from(huge.clone()),
            Expr::cos(Expr::from(variable.clone())),
        ]),
        ScalarDomain::Real,
    );
    let result = verify(&ctx, &source, &variable, ScalarDomain::Real);
    assert_eq!(result.input_scale(), &huge);
    assert_eq!(
        result.primitive().expression(),
        &Expr::mul([Expr::from(huge), Expr::sin(Expr::from(variable))])
    );
}

#[test]
fn bounded_polynomial_degree_rejects_large_exponents() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let input = source(
        Expr::exp(Expr::pow(
            Expr::from(variable.clone()),
            Expr::from(Integer::from(u128::MAX)),
        )),
        ScalarDomain::Real,
    );
    assert_eq!(
        ctx.integrate_elementary_affine(&input, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree)))
    );
    let limits = ResourceLimits {
        max_polynomial_degree: 0,
        ..ResourceLimits::default()
    };
    let ctx = EvaluationContext::new(limits);
    let input = source(Expr::sin(Expr::from(variable.clone())), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_elementary_affine(&input, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree)))
    );
}

#[test]
fn work_node_depth_condition_and_scalar_limits_are_explicit() {
    let variable = x();
    let symbol = Expr::from(variable.clone());
    let input = source(Expr::sin(symbol.clone()), ScalarDomain::Real);
    let work = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        work.integrate_elementary_affine(&input, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::WorkUnits)))
    );
    let nodes = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        nodes.integrate_elementary_affine(&input, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes)))
    );
    let depth = EvaluationContext::new(ResourceLimits {
        max_expression_depth: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        depth.integrate_elementary_affine(&input, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::ExpressionDepth)))
    );
    let conditional = DomainExpression::multiply(
        [
            DomainExpression::with_conditions(
                Expr::from(1), [Condition::NonZero(symbol.clone())], 256, 32,
            ).unwrap(),
            input,
        ],
        32,
    ).unwrap();
    let conditions = EvaluationContext::new(ResourceLimits {
        max_rational_conditions: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        conditions.integrate_elementary_affine(&conditional, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::RationalConditions)))
    );
    let big = source(
        Expr::sin(affine(q(1024, 1), q(0, 1), &variable)),
        ScalarDomain::Real,
    );
    let scalar_bits = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 5,
        ..ResourceLimits::default()
    });
    assert_eq!(
        scalar_bits.integrate_elementary_affine(&big, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::ScalarBits)))
    );
    let intermediate_bits = EvaluationContext::new(ResourceLimits {
        max_intermediate_scalar_bits: 3,
        ..ResourceLimits::default()
    });
    let fractional = source(
        Expr::sin(affine(q(1, 7), q(0, 1), &variable)),
        ScalarDomain::Real,
    );
    assert_eq!(
        intermediate_bits.integrate_elementary_affine(&fractional, &variable, ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(ResourceLimitKind::IntermediateScalarBits)))
    );
}

#[test]
fn verifier_rejects_changed_original_variable_and_domain() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let input = source(
        Expr::sin(affine(q(3, 2), q(1, 4), &variable)),
        ScalarDomain::Real,
    );
    let result = verify(&ctx, &input, &variable, ScalarDomain::Real);
    let other = source(Expr::cos(Expr::from(variable.clone())), ScalarDomain::Real);
    assert_eq!(
        ctx.verify_elementary_antiderivative(&other, &variable, ScalarDomain::Real, &result),
        Err(E::SourceMismatch)
    );
    assert_eq!(
        ctx.verify_elementary_antiderivative(
            &input, &Symbol::new("y").unwrap(), ScalarDomain::Real, &result,
        ),
        Err(E::SourceMismatch)
    );
    assert_eq!(
        ctx.verify_elementary_antiderivative(
            &input, &variable, ScalarDomain::Complex, &result,
        ),
        Err(E::SourceMismatch)
    );
}

#[test]
fn repeated_runs_are_deterministic_and_do_not_mutate_inputs() {
    let ctx = EvaluationContext::default();
    let variable = x();
    let input = source(
        Expr::mul([
            Expr::from(q(7, 9)),
            Expr::exp(affine(q(-8, 13), q(11, 17), &variable)),
        ]),
        ScalarDomain::Complex,
    );
    let before = input.clone();
    let first = verify(&ctx, &input, &variable, ScalarDomain::Complex);
    let second = verify(&ctx, &input, &variable, ScalarDomain::Complex);
    assert_eq!(first, second);
    assert_eq!(input, before);
}
