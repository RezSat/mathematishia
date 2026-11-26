use mathema::{Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    ElementaryIntegrationError, EvaluationContext, EvaluationError, IntegrationDispatchError as E,
    IntegrationError, PolynomialConversionError, ResourceLimitKind, ResourceLimits,
    verify_symbolic_derivative,
};

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn variable() -> Symbol {
    Symbol::new("x").unwrap()
}

fn x() -> Expr {
    Expr::from(variable())
}

fn source(expression: Expr, domain: ScalarDomain) -> DomainExpression {
    DomainExpression::from_expr_in_domain(expression, domain, 4096, 128).unwrap()
}

fn affine(a: Rational, b: Rational) -> Expr {
    Expr::add([Expr::mul([Expr::from(a), x()]), Expr::from(b)])
}

fn checked(
    ctx: &EvaluationContext,
    source: &DomainExpression,
    domain: ScalarDomain,
) -> mathematishia_kernel::DispatchedAntiderivative {
    let result = ctx
        .integrate_expression_dispatch(source, &variable(), domain)
        .unwrap();
    assert_eq!(result.source(), source);
    assert_eq!(result.variable(), &variable());
    assert_eq!(result.domain(), domain);
    assert_eq!(result.conditions(), source.conditions());
    ctx.verify_dispatched_antiderivative(source, &variable(), domain, &result)
        .unwrap();
    result
}

#[test]
fn integrates_x_squared_to_one_third_x_cubed() {
    let ctx = EvaluationContext::default();
    let input = source(Expr::pow(x(), Expr::from(2)), ScalarDomain::Real);
    let result = checked(&ctx, &input, ScalarDomain::Real);
    assert_eq!(
        result.primitive().expression(),
        &Expr::mul([Expr::from(q(1, 3)), Expr::pow(x(), Expr::from(3)),])
    );
    assert_eq!(result.terms().len(), 1);
    let polynomial = result.terms()[0].polynomial_certificate().unwrap();
    assert_eq!(
        polynomial.primitive().differentiate(),
        *polynomial.integrand()
    );
}

#[test]
fn integrates_sine_of_two_x_plus_one() {
    let ctx = EvaluationContext::default();
    let argument = affine(q(2, 1), q(1, 1));
    let input = source(Expr::sin(argument.clone()), ScalarDomain::Real);
    let result = checked(&ctx, &input, ScalarDomain::Real);
    assert_eq!(
        result.primitive().expression(),
        &Expr::mul([Expr::from(q(-1, 2)), Expr::cos(argument)])
    );
    let proof = result.terms()[0].elementary_certificate().unwrap();
    verify_symbolic_derivative(proof.derivative_evidence()).unwrap();
}

#[test]
fn integrates_exp_x_and_scaled_cosine() {
    let ctx = EvaluationContext::default();
    let exp = source(Expr::exp(x()), ScalarDomain::Real);
    assert_eq!(
        checked(&ctx, &exp, ScalarDomain::Real)
            .primitive()
            .expression(),
        &Expr::exp(x())
    );

    let cos = source(
        Expr::mul([Expr::from(q(5, 3)), Expr::cos(affine(q(-2, 1), q(3, 1)))]),
        ScalarDomain::Real,
    );
    assert_eq!(
        checked(&ctx, &cos, ScalarDomain::Real)
            .primitive()
            .expression(),
        &Expr::mul([Expr::from(q(-5, 6)), Expr::sin(affine(q(-2, 1), q(3, 1))),])
    );
}

#[test]
fn integrates_mixed_polynomial_and_affine_function_sum() {
    let ctx = EvaluationContext::default();
    let integrand = Expr::add([
        Expr::from(q(7, 4)),
        Expr::pow(x(), Expr::from(2)),
        Expr::mul([Expr::from(q(-3, 2)), Expr::sin(affine(q(3, 1), q(1, 2)))]),
        Expr::cos(x()),
        Expr::exp(x()),
    ]);
    let result = checked(
        &ctx,
        &source(integrand, ScalarDomain::Real),
        ScalarDomain::Real,
    );
    let expected = Expr::add([
        Expr::mul([Expr::from(q(7, 4)), x()]),
        Expr::mul([Expr::from(q(1, 3)), Expr::pow(x(), Expr::from(3))]),
        Expr::mul([Expr::from(q(1, 2)), Expr::cos(affine(q(3, 1), q(1, 2)))]),
        Expr::sin(x()),
        Expr::exp(x()),
    ]);
    assert_eq!(result.primitive().expression(), &expected);
    assert_eq!(result.terms().len(), 5);
    assert_eq!(
        result
            .terms()
            .iter()
            .filter(|term| term.elementary_certificate().is_some())
            .count(),
        3
    );
}

#[test]
fn integrates_zero_constant_and_rational_polynomial() {
    let ctx = EvaluationContext::default();
    let zero = source(Expr::from(0), ScalarDomain::Rational);
    assert_eq!(
        checked(&ctx, &zero, ScalarDomain::Rational)
            .primitive()
            .expression(),
        &Expr::from(0)
    );
    let input = source(
        Expr::add([Expr::from(q(-3, 7)), Expr::mul([Expr::from(q(2, 5)), x()])]),
        ScalarDomain::Rational,
    );
    assert_eq!(
        checked(&ctx, &input, ScalarDomain::Rational)
            .primitive()
            .expression(),
        &Expr::add([
            Expr::mul([Expr::from(q(-3, 7)), x()]),
            Expr::mul([Expr::from(q(1, 5)), Expr::pow(x(), Expr::from(2))]),
        ])
    );
}

#[test]
fn entire_affine_terms_support_complex_scalar_domain() {
    let ctx = EvaluationContext::default();
    let input = source(
        Expr::add([
            Expr::exp(affine(q(-3, 2), q(7, 4))),
            Expr::cos(affine(q(2, 3), q(1, 2))),
        ]),
        ScalarDomain::Complex,
    );
    let result = checked(&ctx, &input, ScalarDomain::Complex);
    assert!(result.conditions().is_empty());
    assert_eq!(
        result.primitive().expression(),
        &Expr::add([
            Expr::mul([Expr::from(q(-2, 3)), Expr::exp(affine(q(-3, 2), q(7, 4))),]),
            Expr::mul([Expr::from(q(3, 2)), Expr::sin(affine(q(2, 3), q(1, 2))),]),
        ])
    );
}

#[test]
fn retains_unproved_historical_domain_conditions() {
    let ctx = EvaluationContext::default();
    let history = vec![
        Condition::NonZero(x()),
        Condition::Positive(Expr::add([x(), Expr::from(5)])),
    ];
    let historical =
        DomainExpression::with_conditions(Expr::from(0), history.clone(), 4096, 128).unwrap();
    let analytic = source(
        Expr::add([Expr::sin(x()), Expr::pow(x(), Expr::from(2))]),
        ScalarDomain::Real,
    );
    let input = DomainExpression::add([historical, analytic], 128).unwrap();
    let result = checked(&ctx, &input, ScalarDomain::Real);
    assert_eq!(result.conditions(), history);
    assert_eq!(result.primitive().conditions(), input.conditions());
    assert_eq!(
        result.primitive().expression(),
        &Expr::add([
            Expr::mul([Expr::from(-1), Expr::cos(x())]),
            Expr::mul([Expr::from(q(1, 3)), Expr::pow(x(), Expr::from(3))]),
        ])
    );
}

#[test]
fn definite_polynomial_path_checks_directed_exact_values() {
    let ctx = EvaluationContext::default();
    let input = source(
        Expr::add([Expr::pow(x(), Expr::from(2)), Expr::from(1)]),
        ScalarDomain::Real,
    );
    let low = q(0, 1);
    let high = q(2, 1);
    let result = ctx
        .integrate_expression_dispatch_between(&input, &variable(), ScalarDomain::Real, &low, &high)
        .unwrap();
    assert_eq!(result.certificate().value(), &q(14, 3));
    ctx.verify_dispatched_definite_integral(
        &input,
        &variable(),
        ScalarDomain::Real,
        &low,
        &high,
        &result,
    )
    .unwrap();

    let reverse = ctx
        .integrate_expression_dispatch_between(&input, &variable(), ScalarDomain::Real, &high, &low)
        .unwrap();
    assert_eq!(reverse.certificate().value(), &q(-14, 3));
    let same = ctx
        .integrate_expression_dispatch_between(&input, &variable(), ScalarDomain::Real, &low, &low)
        .unwrap();
    assert_eq!(same.certificate().value(), &q(0, 1));
}

#[test]
fn rejects_mixed_function_products_and_unexpanded_products_of_sums() {
    let ctx = EvaluationContext::default();
    for expression in [
        Expr::mul([Expr::exp(x()), Expr::cos(x())]),
        Expr::mul([x(), Expr::sin(x())]),
        Expr::mul([Expr::from(2), Expr::add([Expr::sin(x()), Expr::cos(x())])]),
    ] {
        let input = source(expression, ScalarDomain::Real);
        assert_eq!(
            ctx.integrate_expression_dispatch(&input, &variable(), ScalarDomain::Real),
            Err(E::Elementary(ElementaryIntegrationError::UnsupportedForm))
        );
    }
}

#[test]
fn rejects_unsupported_analytic_rules_arguments_and_other_symbols() {
    let ctx = EvaluationContext::default();
    for expression in [Expr::log(x()), Expr::tan(x())] {
        let input = source(expression, ScalarDomain::Real);
        assert_eq!(
            ctx.integrate_expression_dispatch(&input, &variable(), ScalarDomain::Real),
            Err(E::Elementary(
                ElementaryIntegrationError::UnsupportedFunction
            ))
        );
    }
    let nonlinear = source(Expr::sin(Expr::pow(x(), Expr::from(2))), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_expression_dispatch(&nonlinear, &variable(), ScalarDomain::Real),
        Err(E::Elementary(ElementaryIntegrationError::NonAffineArgument))
    );
    let y = source(Expr::from(Symbol::new("y").unwrap()), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_expression_dispatch(&y, &variable(), ScalarDomain::Real),
        Err(E::Polynomial(IntegrationError::UnsupportedIntegrand(
            PolynomialConversionError::OtherSymbol,
        )))
    );
}

#[test]
fn rejects_nonpolynomial_powers_and_other_scalar_domains() {
    let ctx = EvaluationContext::default();
    for exponent in [-1, 0] {
        let input = source(Expr::pow(x(), Expr::from(exponent)), ScalarDomain::Real);
        assert_eq!(
            ctx.integrate_expression_dispatch(&input, &variable(), ScalarDomain::Real),
            Err(E::Polynomial(IntegrationError::UnsupportedIntegrand(
                PolynomialConversionError::UnsupportedPower,
            )))
        );
    }
    let integral = source(x(), ScalarDomain::Integer);
    assert_eq!(
        ctx.integrate_expression_dispatch(&integral, &variable(), ScalarDomain::Integer),
        Err(E::UnsupportedScalarDomain)
    );
}

#[test]
fn definite_integrals_refuse_analytic_endpoints_and_historical_holes() {
    let ctx = EvaluationContext::default();
    let analytic = source(Expr::exp(x()), ScalarDomain::Real);
    assert_eq!(
        ctx.integrate_expression_dispatch_between(
            &analytic,
            &variable(),
            ScalarDomain::Real,
            &q(0, 1),
            &q(1, 1),
        ),
        Err(E::UnsupportedAnalyticDefiniteIntegral)
    );

    let inherited = DomainExpression::with_conditions(
        Expr::pow(x(), Expr::from(2)),
        [Condition::NonZero(x())],
        256,
        32,
    )
    .unwrap();
    assert_eq!(
        ctx.integrate_expression_dispatch_between(
            &inherited,
            &variable(),
            ScalarDomain::Real,
            &q(1, 1),
            &q(3, 1),
        ),
        Err(E::RestrictedDefiniteDomain)
    );
}

#[test]
fn rejects_exhausted_work_node_degree_and_term_budgets() {
    let input = source(Expr::sin(x()), ScalarDomain::Real);
    let work = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        work.integrate_expression_dispatch(&input, &variable(), ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        )))
    );

    let nodes = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        nodes.integrate_expression_dispatch(&input, &variable(), ScalarDomain::Real),
        Err(E::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::ExpressionNodes
        )))
    );

    let degree = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 2,
        ..ResourceLimits::default()
    });
    let square = source(Expr::pow(x(), Expr::from(2)), ScalarDomain::Real);
    assert_eq!(
        degree.integrate_expression_dispatch(&square, &variable(), ScalarDomain::Real),
        Err(E::Polynomial(IntegrationError::Evaluation(
            EvaluationError::Limit(ResourceLimitKind::PolynomialDegree,)
        )))
    );

    let many_terms = source(
        Expr::add((0..65).map(|i| Expr::sin(Expr::add([x(), Expr::from(i)])))),
        ScalarDomain::Real,
    );
    let generous = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 4096,
        max_rational_conditions: 128,
        ..ResourceLimits::default()
    });
    assert_eq!(
        generous.integrate_expression_dispatch(&many_terms, &variable(), ScalarDomain::Real),
        Err(E::TooManyTerms)
    );
}

#[test]
fn exact_source_and_endpoint_certificate_inputs_are_bound() {
    let ctx = EvaluationContext::default();
    let input = source(Expr::pow(x(), Expr::from(2)), ScalarDomain::Real);
    let result = ctx
        .integrate_expression_dispatch_between(
            &input,
            &variable(),
            ScalarDomain::Real,
            &q(0, 1),
            &q(1, 1),
        )
        .unwrap();
    assert_eq!(
        ctx.verify_dispatched_definite_integral(
            &input,
            &variable(),
            ScalarDomain::Real,
            &q(0, 1),
            &q(2, 1),
            &result,
        ),
        Err(E::IncorrectEvidence)
    );
    let different = source(x(), ScalarDomain::Real);
    assert_eq!(
        ctx.verify_dispatched_definite_integral(
            &different,
            &variable(),
            ScalarDomain::Real,
            &q(0, 1),
            &q(1, 1),
            &result,
        ),
        Err(E::SourceMismatch)
    );
}

#[test]
fn deterministic_repeated_invocations_preserve_original_expression() {
    let ctx = EvaluationContext::default();
    let input = source(
        Expr::add([
            Expr::pow(x(), Expr::from(3)),
            Expr::sin(affine(q(4, 3), q(-2, 1))),
            Expr::mul([Expr::from(q(5, 9)), Expr::exp(x())]),
        ]),
        ScalarDomain::Complex,
    );
    let original = input.clone();
    let a = checked(&ctx, &input, ScalarDomain::Complex);
    let b = checked(&ctx, &input, ScalarDomain::Complex);
    assert_eq!(a, b);
    assert_eq!(input, original);
}

#[test]
fn dispatches_checked_polynomial_exponential_terms() {
    let ctx = EvaluationContext::default();
    let input = source(Expr::mul([x(), Expr::exp(x())]), ScalarDomain::Real);
    let result = checked(&ctx, &input, ScalarDomain::Real);
    let proof = result.terms()[0]
        .polynomial_exponential_certificate()
        .unwrap();
    assert_eq!(
        proof.primitive_polynomial().coefficients(),
        &[q(-1, 1), q(1, 1)]
    );
    assert_eq!(
        result.primitive().expression(),
        &Expr::mul([Expr::add([x(), Expr::from(-1)]), Expr::exp(x())])
    );
}
