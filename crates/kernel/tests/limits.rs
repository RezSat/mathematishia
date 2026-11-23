use mathema::{DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, LimitDirection, LimitError, LimitValue,
    LimitVerificationError, RationalExpressionError, RationalFunctionError, RationalPolynomial,
    ResourceLimitKind, ResourceLimits, verify_rational_limit,
};

fn r(value: i64) -> Rational {
    Rational::from(Integer::from(value))
}

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(values.iter().copied().map(r).collect())
}

fn certified(
    ctx: &EvaluationContext,
    numerator: &[i64],
    denominator: &[i64],
    at: Rational,
    direction: LimitDirection,
    expected: LimitValue,
    excluded: bool,
) {
    let source = ctx.rational_function(&p(numerator), &p(denominator)).unwrap();
    let result = ctx.rational_limit(&source, &at, direction).unwrap();
    assert_eq!(result.value(), &expected);
    assert_eq!(result.direction(), direction);
    assert_eq!(result.at(), &at);
    assert_eq!(result.excluded_at_point(), excluded);
    assert_eq!(result.source(), &source);
    assert_eq!(
        verify_rational_limit(ctx, &source, &at, direction, &result),
        Ok(())
    );
    assert_eq!(ctx.rational_limit(&source, &at, direction), Ok(result));
}

#[test]
fn user_questions_with_exact_finite_answers_and_removable_holes() {
    let ctx = EvaluationContext::default();
    let two = LimitDirection::TwoSided;
    // lim_(x->1) (x^2-1)/(x-1) = 2, but the original function is undefined at 1.
    certified(&ctx, &[-1, 0, 1], &[-1, 1], r(1), two, LimitValue::Finite(r(2)), true);
    // lim_(x->2) (x^2-4)/(x-2) = 4.
    certified(&ctx, &[-4, 0, 1], &[-2, 1], r(2), two, LimitValue::Finite(r(4)), true);
    // lim_(x->1) (x-1)/(x-1) = 1.
    certified(&ctx, &[-1, 1], &[-1, 1], r(1), two, LimitValue::Finite(r(1)), true);
    // lim_(x->0) x^3/x = 0, without filling the original hole.
    certified(&ctx, &[0, 0, 0, 1], &[0, 1], r(0), two, LimitValue::Finite(r(0)), true);
    // lim_(x->2) (3x^2-2x+4) = 12.
    certified(&ctx, &[4, -2, 3], &[1], r(2), two, LimitValue::Finite(r(12)), false);
    // lim_(x->1/2) (2x+1)/(x+2) = 4/5.
    certified(&ctx, &[1, 2], &[2, 1], q(1, 2), two, LimitValue::Finite(q(4, 5)), false);
    // lim_(x->1) 0/(x-1) = 0 despite the source hole.
    certified(&ctx, &[], &[-1, 1], r(1), two, LimitValue::Finite(r(0)), true);
}

#[test]
fn two_sided_poles_and_both_one_sided_signs() {
    use LimitDirection::{Left, Right, TwoSided};
    use LimitValue::{DoesNotExist, NegativeInfinity, PositiveInfinity};
    let ctx = EvaluationContext::default();

    // lim_(x->0) 1/x does not exist two-sided, with signed one-sided poles.
    certified(&ctx, &[1], &[0, 1], r(0), TwoSided, DoesNotExist, true);
    certified(&ctx, &[1], &[0, 1], r(0), Left, NegativeInfinity, true);
    certified(&ctx, &[1], &[0, 1], r(0), Right, PositiveInfinity, true);

    // lim_(x->0) 1/x^2 = +infinity from either side.
    for direction in [Left, Right, TwoSided] {
        certified(&ctx, &[1], &[0, 0, 1], r(0), direction, PositiveInfinity, true);
    }

    // lim_(x->0) -2/x^3 changes sign across the pole.
    certified(&ctx, &[-2], &[0, 0, 0, 1], r(0), Left, PositiveInfinity, true);
    certified(&ctx, &[-2], &[0, 0, 0, 1], r(0), Right, NegativeInfinity, true);
    certified(&ctx, &[-2], &[0, 0, 0, 1], r(0), TwoSided, DoesNotExist, true);

    // lim_(x->1) (x^2+1)/(x-1), numerator is strictly positive nearby.
    certified(&ctx, &[1, 0, 1], &[-1, 1], r(1), Left, NegativeInfinity, true);
    certified(&ctx, &[1, 0, 1], &[-1, 1], r(1), Right, PositiveInfinity, true);
    // lim_(x->0) -1/x^2 is -infinity from both sides.
    certified(&ctx, &[-1], &[0, 0, 1], r(0), TwoSided, NegativeInfinity, true);
}

#[test]
fn original_domain_history_survives_conversion_and_limit() {
    let x = Expr::from(Symbol::new("x").unwrap());
    let numerator = DomainExpression::from_expr(
        Expr::add([Expr::pow(x.clone(), 2.into()), (-1).into()]),
        128,
    )
    .unwrap();
    let denominator = DomainExpression::from_expr(
        Expr::add([x.clone(), (-1).into()]),
        128,
    )
    .unwrap();
    let source = numerator.divide(denominator, 16).unwrap();
    let ctx = EvaluationContext::default();
    let answer = ctx
        .rational_expression_limit(
            &source,
            &Symbol::new("x").unwrap(),
            &r(1),
            LimitDirection::TwoSided,
        )
        .unwrap();
    assert_eq!(answer.value(), &LimitValue::Finite(r(2)));
    assert!(answer.excluded_at_point());
    assert_eq!(
        answer.source().evaluate(&r(1)),
        Err(RationalFunctionError::UndefinedAtPoint)
    );
}

#[test]
fn unsupported_functions_and_other_variables_fail_explicitly() {
    let ctx = EvaluationContext::default();
    let x = Expr::from(Symbol::new("x").unwrap());
    let y = Expr::from(Symbol::new("y").unwrap());
    let variable = Symbol::new("x").unwrap();
    let sine = DomainExpression::from_expr_in_domain(
        Expr::sin(x.clone()),
        ScalarDomain::Real,
        128,
        16,
    )
    .unwrap();
    assert_eq!(
        ctx.rational_expression_limit(&sine, &variable, &r(0), LimitDirection::TwoSided),
        Err(LimitError::Conversion(RationalExpressionError::UnsupportedFunction))
    );
    let multiple = DomainExpression::from_expr(Expr::add([x, y]), 128).unwrap();
    assert_eq!(
        ctx.rational_expression_limit(&multiple, &variable, &r(0), LimitDirection::Left),
        Err(LimitError::Conversion(RationalExpressionError::OtherSymbol))
    );
}

#[test]
fn invalid_denominator_and_contradictory_exclusions_are_rejected() {
    let ctx = EvaluationContext::default();
    assert_eq!(
        ctx.rational_function(&p(&[1]), &p(&[])),
        Err(EvaluationError::RationalFunction(
            RationalFunctionError::ZeroDenominator
        ))
    );
    let source = ctx.rational_function(&p(&[1]), &p(&[1])).unwrap();
    assert_eq!(
        source.restrict_nonzero(p(&[])),
        Err(RationalFunctionError::EmptyDomain)
    );
}

#[test]
fn verify_checks_the_supplied_source_point_and_direction() {
    let ctx = EvaluationContext::default();
    let source = ctx.rational_function(&p(&[1]), &p(&[0, 1])).unwrap();
    let claimed = ctx
        .rational_limit(&source, &r(0), LimitDirection::Right)
        .unwrap();
    assert_eq!(
        verify_rational_limit(&ctx, &source, &r(1), LimitDirection::Right, &claimed),
        Err(LimitVerificationError::InvalidCertificate)
    );
    assert_eq!(
        verify_rational_limit(&ctx, &source, &r(0), LimitDirection::Left, &claimed),
        Err(LimitVerificationError::InvalidCertificate)
    );
    let altered = ctx.rational_function(&p(&[2]), &p(&[0, 1])).unwrap();
    assert_eq!(
        verify_rational_limit(&ctx, &altered, &r(0), LimitDirection::Right, &claimed),
        Err(LimitVerificationError::InvalidCertificate)
    );
}

#[test]
fn resource_limits_fail_closed_without_approximating() {
    let standard = EvaluationContext::default();
    let source = standard.rational_function(&p(&[1, 0, 1]), &p(&[0, 1])).unwrap();
    for (limits, expected) in [
        (
            ResourceLimits { max_work_units: 0, ..ResourceLimits::default() },
            ResourceLimitKind::WorkUnits,
        ),
        (
            ResourceLimits { max_polynomial_degree: 1, ..ResourceLimits::default() },
            ResourceLimitKind::PolynomialDegree,
        ),
        (
            ResourceLimits { max_scalar_bits: 1, ..ResourceLimits::default() },
            ResourceLimitKind::ScalarBits,
        ),
    ] {
        let ctx = EvaluationContext::new(limits);
        let target = if expected == ResourceLimitKind::ScalarBits {
            r(2)
        } else {
            r(0)
        };
        assert_eq!(
            ctx.rational_limit(&source, &target, LimitDirection::TwoSided),
            Err(LimitError::Evaluation(EvaluationError::Limit(expected)))
        );
    }
    let tiny_intermediate = EvaluationContext::new(ResourceLimits {
        max_intermediate_scalar_bits: 1,
        ..ResourceLimits::default()
    });
    let huge_target = q(100, 101);
    assert_eq!(
        tiny_intermediate.rational_limit(&source, &huge_target, LimitDirection::Left),
        Err(LimitError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::IntermediateScalarBits
        )))
    );
    let restricted_conditions = EvaluationContext::new(ResourceLimits {
        max_rational_conditions: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        restricted_conditions.rational_limit(&source, &r(0), LimitDirection::Left),
        Err(LimitError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::RationalConditions
        )))
    );
}

#[test]
fn deterministic_dense_bounded_polynomial_corpus() {
    let ctx = EvaluationContext::default();
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                let polynomial = p(&[a, b, c]);
                let target = r(1);
                let expected = LimitValue::Finite(r(a + b + c));
                let source = mathematishia_kernel::RationalFunction::from_polynomial(polynomial);
                let result = ctx
                    .rational_limit(&source, &target, LimitDirection::TwoSided)
                    .unwrap();
                assert_eq!(result.value(), &expected);
                assert_eq!(
                    verify_rational_limit(
                        &ctx,
                        &source,
                        &target,
                        LimitDirection::TwoSided,
                        &result
                    ),
                    Ok(())
                );
            }
        }
    }
}
