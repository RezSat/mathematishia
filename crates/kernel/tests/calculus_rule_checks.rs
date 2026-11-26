use mathema::{Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalCalculusError, RationalCalculusRelation,
    RationalFunction, RationalFunctionError, RationalPolynomial, ResourceLimitKind, ResourceLimits,
    certify_rational_calculus, verify_rational_calculus,
};

use RationalCalculusRelation::{Derivative, Identity};

fn variable() -> Symbol {
    Symbol::new("x").unwrap()
}

fn x() -> Expr {
    Expr::from(variable())
}

fn q(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}

fn domain(expression: Expr) -> DomainExpression {
    DomainExpression::from_expr(expression, 128).unwrap()
}

fn restricted(expression: Expr, restriction: Expr) -> DomainExpression {
    DomainExpression::with_conditions(expression, [Condition::NonZero(restriction)], 128, 16)
        .unwrap()
}

fn power(base: Expr, exponent: i64) -> Expr {
    Expr::pow(base, Expr::from(exponent))
}

fn shift(n: i64) -> Expr {
    Expr::add([x(), Expr::from(n)])
}

#[test]
fn equivalent_polynomial_derivative_without_structural_equality() {
    let context = EvaluationContext::default();
    let source = domain(power(shift(1), 3));
    let candidate = domain(Expr::add([
        3.into(),
        Expr::mul([6.into(), x()]),
        Expr::mul([3.into(), power(x(), 2)]),
    ]));
    let proof =
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative).unwrap();
    assert_eq!(proof.value.numerator().coefficients(), &[q(3), q(6), q(3)]);
    assert_eq!(proof.value.evaluate(&q(2)), Ok(q(27)));
    assert!(proof.value.excluded_zero_sets().is_empty());
    verify_rational_calculus(
        &context,
        &source,
        &candidate,
        &variable(),
        Derivative,
        &proof,
    )
    .unwrap();
    assert_eq!(
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative),
        Ok(proof)
    );
}

#[test]
fn exact_quotient_rule_and_pole() {
    let context = EvaluationContext::default();
    let source = domain(Expr::mul([shift(1), power(shift(-1), -1)]));
    let candidate = domain(Expr::mul([(-2).into(), power(shift(-1), -2)]));
    let proof =
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative).unwrap();
    assert_eq!(proof.value.evaluate(&q(2)), Ok(q(-2)));
    assert_eq!(
        proof.value.evaluate(&q(1)),
        Err(RationalFunctionError::UndefinedAtPoint)
    );
    verify_rational_calculus(
        &context,
        &source,
        &candidate,
        &variable(),
        Derivative,
        &proof,
    )
    .unwrap();
}

#[test]
fn canceled_factor_and_historical_zero_keep_original_holes() {
    let context = EvaluationContext::default();
    let source = domain(Expr::mul([
        Expr::add([power(x(), 2), (-1).into()]),
        power(shift(-1), -1),
    ]));
    let one = domain(1.into());
    let proof =
        certify_rational_calculus(&context, &source, &one, &variable(), Derivative).unwrap();
    assert_eq!(proof.value.evaluate(&q(0)), Ok(q(1)));
    assert_eq!(
        proof.value.evaluate(&q(1)),
        Err(RationalFunctionError::UndefinedAtPoint)
    );
    let zero = DomainExpression::multiply([domain(0.into()), domain(power(x(), -1))], 16).unwrap();
    let candidate = domain(0.into());
    let proof =
        certify_rational_calculus(&context, &zero, &candidate, &variable(), Derivative).unwrap();
    assert_eq!(proof.value.evaluate(&q(2)), Ok(q(0)));
    assert_eq!(
        proof.value.evaluate(&q(0)),
        Err(RationalFunctionError::UndefinedAtPoint)
    );
    verify_rational_calculus(&context, &zero, &candidate, &variable(), Derivative, &proof).unwrap();
}

#[test]
fn product_exclusions_and_repeated_factors_are_proved() {
    let context = EvaluationContext::default();
    let product = Expr::mul([shift(-1), shift(1)]);
    let source = restricted(x(), product);
    let candidate = restricted(x(), power(shift(-1), 3));
    let proof =
        certify_rational_calculus(&context, &source, &candidate, &variable(), Identity).unwrap();
    for point in [-1, 1] {
        assert_eq!(
            proof.value.evaluate(&q(point)),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
    }
    verify_rational_calculus(&context, &source, &candidate, &variable(), Identity, &proof).unwrap();

    let source = DomainExpression::with_conditions(
        x(),
        [Condition::NonZero(shift(-1)), Condition::NonZero(shift(1))],
        128,
        16,
    )
    .unwrap();
    let candidate = restricted(x(), Expr::add([power(x(), 2), (-1).into()]));
    assert!(
        certify_rational_calculus(&context, &source, &candidate, &variable(), Identity).is_ok()
    );
}

#[test]
fn equal_field_values_do_not_authorize_new_exclusions() {
    let context = EvaluationContext::default();
    let source = domain(x());
    let candidate = restricted(x(), shift(-1));
    assert_eq!(
        certify_rational_calculus(&context, &source, &candidate, &variable(), Identity),
        Err(RationalCalculusError::NewExclusion)
    );
    let candidate = restricted(1.into(), x());
    assert_eq!(
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative),
        Err(RationalCalculusError::NewExclusion)
    );
    let candidate = restricted(x(), Expr::add([power(x(), 2), 1.into()]));
    assert_eq!(
        certify_rational_calculus(&context, &source, &candidate, &variable(), Identity),
        Err(RationalCalculusError::NewExclusion)
    );
}

#[test]
fn nested_rational_conditions_preserve_their_own_undefined_points() {
    let context = EvaluationContext::default();
    let source = restricted(x(), Expr::mul([shift(1), power(shift(-1), -1)]));
    let candidate = domain(1.into());
    let proof =
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative).unwrap();
    for point in [-1, 1] {
        assert_eq!(
            proof.value.evaluate(&q(point)),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
    }
    assert_eq!(proof.value.evaluate(&q(0)), Ok(q(1)));
}

#[test]
fn wrong_derivative_and_forged_evidence_are_rejected() {
    let context = EvaluationContext::default();
    let source = restricted(power(x(), 2), shift(-1));
    let candidate = domain(Expr::mul([2.into(), x()]));
    let wrong = domain(x());
    assert_eq!(
        certify_rational_calculus(&context, &source, &wrong, &variable(), Derivative),
        Err(RationalCalculusError::IncorrectIdentity)
    );
    let proof =
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative).unwrap();
    let mut forged = proof.clone();
    forged.value = RationalFunction::from_polynomial(proof.value.numerator().clone());
    assert_eq!(
        verify_rational_calculus(
            &context,
            &source,
            &candidate,
            &variable(),
            Derivative,
            &forged
        ),
        Err(RationalCalculusError::IncorrectCertificate)
    );
    let mut forged = proof.clone();
    forged.value = RationalFunction::from_polynomial(RationalPolynomial::new(vec![q(17)]))
        .restrict_nonzero(proof.value.excluded_zero_sets()[0].clone())
        .unwrap();
    assert_eq!(
        verify_rational_calculus(
            &context,
            &source,
            &candidate,
            &variable(),
            Derivative,
            &forged
        ),
        Err(RationalCalculusError::IncorrectIdentity)
    );
    let mut forged = proof.clone();
    forged.source = domain(power(x(), 2));
    assert_eq!(
        verify_rational_calculus(
            &context,
            &source,
            &candidate,
            &variable(),
            Derivative,
            &forged
        ),
        Err(RationalCalculusError::IncorrectCertificate)
    );
    let mut forged = proof;
    forged.relation = Identity;
    assert_eq!(
        verify_rational_calculus(
            &context,
            &source,
            &candidate,
            &variable(),
            Derivative,
            &forged
        ),
        Err(RationalCalculusError::IncorrectCertificate)
    );
}

#[test]
fn zero_power_respects_the_nonzero_base_convention() {
    let context = EvaluationContext::default();
    let source = domain(power(x(), 0));
    let candidate = domain(0.into());
    let proof =
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative).unwrap();
    assert_eq!(proof.value.evaluate(&q(-2)), Ok(q(0)));
    assert_eq!(
        proof.value.evaluate(&q(0)),
        Err(RationalFunctionError::UndefinedAtPoint)
    );
    for exponent in [-1, 0] {
        let invalid = domain(power(0.into(), exponent));
        assert!(matches!(
            certify_rational_calculus(&context, &invalid, &candidate, &variable(), Derivative),
            Err(RationalCalculusError::Evaluation(
                EvaluationError::RationalFunction(RationalFunctionError::ZeroDivisor)
            ))
        ));
    }
    let negative = domain(power((-2).into(), -3));
    assert!(
        certify_rational_calculus(&context, &negative, &candidate, &variable(), Derivative).is_ok()
    );
    let impossible = restricted(x(), 0.into());
    assert_eq!(
        certify_rational_calculus(&context, &impossible, &candidate, &variable(), Derivative),
        Err(RationalCalculusError::Evaluation(
            EvaluationError::RationalFunction(RationalFunctionError::EmptyDomain)
        ))
    );
}

#[test]
fn analytic_and_general_power_dependencies_are_explicit() {
    let context = EvaluationContext::default();
    let candidate = domain(1.into());
    let nested = DomainExpression::from_expr_in_domain(
        Expr::exp(Expr::sin(x())),
        ScalarDomain::Real,
        128,
        16,
    )
    .unwrap();
    assert_eq!(
        certify_rational_calculus(&context, &nested, &candidate, &variable(), Derivative),
        Err(RationalCalculusError::UnsupportedFunction)
    );
    for base in [x(), (-2).into(), 0.into()] {
        assert!(DomainExpression::from_expr(Expr::pow(base, x()), 128).is_err());
    }
    let source =
        DomainExpression::with_conditions(x(), [Condition::Positive(x())], 128, 16).unwrap();
    assert_eq!(
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative),
        Err(RationalCalculusError::UnsupportedCondition)
    );
    let other = domain(Expr::from(Symbol::new("y").unwrap()));
    assert_eq!(
        certify_rational_calculus(&context, &other, &candidate, &variable(), Derivative),
        Err(RationalCalculusError::UnsupportedSymbol)
    );
}

#[test]
fn resource_refusals_apply_to_generation_and_verification() {
    let context = EvaluationContext::default();
    let source = domain(power(shift(1), 3));
    let candidate = domain(Expr::mul([3.into(), power(shift(1), 2)]));
    let proof =
        certify_rational_calculus(&context, &source, &candidate, &variable(), Derivative).unwrap();
    let tight = EvaluationContext::new(ResourceLimits {
        max_work_units: 1,
        ..ResourceLimits::default()
    });
    let expected =
        RationalCalculusError::Evaluation(EvaluationError::Limit(ResourceLimitKind::WorkUnits));
    assert_eq!(
        certify_rational_calculus(&tight, &source, &candidate, &variable(), Derivative),
        Err(expected)
    );
    assert_eq!(
        verify_rational_calculus(&tight, &source, &candidate, &variable(), Derivative, &proof),
        Err(expected)
    );
    let tight = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 3,
        ..ResourceLimits::default()
    });
    assert_eq!(
        certify_rational_calculus(&tight, &source, &candidate, &variable(), Derivative),
        Err(RationalCalculusError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::ExpressionNodes
        )))
    );
    let tight = EvaluationContext::new(ResourceLimits {
        max_symbolic_power: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        certify_rational_calculus(&tight, &source, &candidate, &variable(), Derivative),
        Err(RationalCalculusError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::SymbolicPower
        )))
    );
}

#[test]
fn admission_checks_historical_conditions_and_intermediate_growth() {
    let source = restricted(x(), power(shift(-1), 4));
    let candidate = domain(1.into());
    let limits = ResourceLimits {
        max_polynomial_degree: 3,
        ..ResourceLimits::default()
    };
    assert_eq!(
        certify_rational_calculus(
            &EvaluationContext::new(limits),
            &source,
            &candidate,
            &variable(),
            Derivative,
        ),
        Err(RationalCalculusError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::PolynomialDegree
        )))
    );
    let limits = ResourceLimits {
        max_rational_conditions: 0,
        ..ResourceLimits::default()
    };
    assert_eq!(
        certify_rational_calculus(
            &EvaluationContext::new(limits),
            &source,
            &candidate,
            &variable(),
            Derivative,
        ),
        Err(RationalCalculusError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::RationalConditions
        )))
    );
    let source = domain(power(Expr::from(257), 2));
    let limits = ResourceLimits {
        max_intermediate_scalar_bits: 12,
        ..ResourceLimits::default()
    };
    assert_eq!(
        certify_rational_calculus(
            &EvaluationContext::new(limits),
            &source,
            &domain(0.into()),
            &variable(),
            Derivative,
        ),
        Err(RationalCalculusError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::IntermediateScalarBits
        )))
    );
}
