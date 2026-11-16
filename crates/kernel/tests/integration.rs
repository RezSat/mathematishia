use mathema::{DomainExpression, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, IntegrationError, PolynomialConversionError,
    RationalPolynomial, ResourceLimitKind, ResourceLimits,
};

fn q(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn p(coefficients: &[(i64, i64)]) -> RationalPolynomial {
    RationalPolynomial::new(coefficients.iter().map(|&(n, d)| q(n, d)).collect())
}

fn x() -> (Expr, Symbol) {
    let variable = Symbol::new("x").unwrap();
    (Expr::from(variable.clone()), variable)
}

#[test]
fn canonical_zero_and_constant_primitives() {
    let context = EvaluationContext::default();
    let zero = context
        .integrate_polynomial(&RationalPolynomial::zero())
        .unwrap();
    assert_eq!(zero.primitive(), &RationalPolynomial::zero());
    assert_eq!(zero.primitive().differentiate(), *zero.integrand());
    context.verify_polynomial_primitive(&zero).unwrap();

    let constant = p(&[(2, 3)]);
    let primitive = context.integrate_polynomial(&constant).unwrap();
    assert_eq!(primitive.primitive(), &p(&[(0, 1), (2, 3)]));
    assert_eq!(primitive.primitive().differentiate(), constant);
    context.verify_polynomial_primitive(&primitive).unwrap();
}

#[test]
fn integrates_exact_fractional_sparse_and_signed_coefficients() {
    let context = EvaluationContext::default();
    let integrand = p(&[(3, 2), (-2, 3), (5, 4), (0, 1), (-7, 5)]);
    let primitive = context.integrate_polynomial(&integrand).unwrap();
    assert_eq!(
        primitive.primitive(),
        &p(&[(0, 1), (3, 2), (-1, 3), (5, 12), (0, 1), (-7, 25)])
    );
    assert_eq!(primitive.primitive().differentiate(), integrand);
}

#[test]
fn definite_integral_uses_exact_fundamental_theorem_substitution() {
    let context = EvaluationContext::default();
    let integrand = p(&[(3, 2), (-2, 3), (5, 4)]);
    let result = context
        .integrate_polynomial_between(&integrand, &q(-2, 1), &q(3, 2))
        .unwrap();
    assert_eq!(result.value(), &q(1015, 96));
    assert_eq!(result.lower(), &q(-2, 1));
    assert_eq!(result.upper(), &q(3, 2));
    assert_eq!(result.primitive().integrand(), &integrand);
    context
        .verify_definite_polynomial_integral(&result)
        .unwrap();
}

#[test]
fn reversed_equal_and_negative_bounds_preserve_orientation() {
    let context = EvaluationContext::default();
    let integrand = p(&[(0, 1), (1, 1)]);
    let forward = context
        .integrate_polynomial_between(&integrand, &q(-2, 1), &q(3, 1))
        .unwrap();
    let reverse = context
        .integrate_polynomial_between(&integrand, &q(3, 1), &q(-2, 1))
        .unwrap();
    let same = context
        .integrate_polynomial_between(&integrand, &q(-2, 1), &q(-2, 1))
        .unwrap();
    assert_eq!(forward.value(), &q(5, 2));
    assert_eq!(reverse.value(), &q(-5, 2));
    assert_eq!(same.value(), &q(0, 1));
}

#[test]
fn zero_integrates_to_zero_on_any_rational_interval() {
    let context = EvaluationContext::default();
    let result = context
        .integrate_polynomial_between(&RationalPolynomial::zero(), &q(-71, 13), &q(5, 9))
        .unwrap();
    assert_eq!(result.value(), &q(0, 1));
    context
        .verify_definite_polynomial_integral(&result)
        .unwrap();
}

#[test]
fn integration_is_additive_with_canonical_constants() {
    let context = EvaluationContext::default();
    let a = p(&[(2, 3), (0, 1), (4, 5)]);
    let b = p(&[(-7, 6), (1, 2), (0, 1), (-9, 4)]);
    let sum = a.add(&b);
    let fa = context.integrate_polynomial(&a).unwrap();
    let fb = context.integrate_polynomial(&b).unwrap();
    let fsum = context.integrate_polynomial(&sum).unwrap();
    assert_eq!(fsum.primitive(), &fa.primitive().add(fb.primitive()));
    let lower = q(-3, 2);
    let upper = q(7, 3);
    let ia = context
        .integrate_polynomial_between(&a, &lower, &upper)
        .unwrap();
    let ib = context
        .integrate_polynomial_between(&b, &lower, &upper)
        .unwrap();
    let isum = context
        .integrate_polynomial_between(&sum, &lower, &upper)
        .unwrap();
    assert_eq!(isum.value(), &(ia.value() + ib.value()));
}

#[test]
fn exact_high_degree_within_limit_is_differentiated_back() {
    let context = EvaluationContext::default();
    let mut coefficients = vec![q(0, 1); 32];
    coefficients[31] = q(19, 7);
    let input = RationalPolynomial::new(coefficients);
    let primitive = context.integrate_polynomial(&input).unwrap();
    assert_eq!(primitive.primitive().degree(), Some(32));
    assert_eq!(primitive.primitive().coefficients()[32], q(19, 224));
    context.verify_polynomial_primitive(&primitive).unwrap();
}

#[test]
fn rejects_input_whose_primitive_would_exceed_degree_limit() {
    let context = EvaluationContext::default();
    let mut coefficients = vec![q(0, 1); 33];
    coefficients[32] = q(1, 1);
    let input = RationalPolynomial::new(coefficients);
    assert_eq!(
        context.integrate_polynomial(&input),
        Err(IntegrationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::PolynomialDegree
        )))
    );
}

#[test]
fn degree_zero_budget_still_admits_zero_but_not_nonzero_constants() {
    let mut limits = ResourceLimits::default();
    limits.max_polynomial_degree = 0;
    let context = EvaluationContext::new(limits);
    assert_eq!(
        context
            .integrate_polynomial(&RationalPolynomial::zero())
            .unwrap()
            .primitive(),
        &RationalPolynomial::zero()
    );
    assert_eq!(
        context.integrate_polynomial(&p(&[(1, 1)])),
        Err(IntegrationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::PolynomialDegree
        )))
    );
}

#[test]
fn rejects_work_scalar_and_intermediate_bit_limits() {
    let input = p(&[(1, 1), (1, 1)]);
    let mut limits = ResourceLimits::default();
    limits.max_work_units = 1;
    assert_eq!(
        EvaluationContext::new(limits).integrate_polynomial(&input),
        Err(IntegrationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        )))
    );

    let mut limits = ResourceLimits::default();
    limits.max_scalar_bits = 16;
    let too_large = RationalPolynomial::new(vec![Rational::from(Integer::from(u128::MAX))]);
    assert_eq!(
        EvaluationContext::new(limits).integrate_polynomial(&too_large),
        Err(IntegrationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::ScalarBits
        )))
    );

    let mut limits = ResourceLimits::default();
    limits.max_intermediate_scalar_bits = 8;
    let result = EvaluationContext::new(limits).integrate_polynomial_between(
        &p(&[(0, 1), (0, 1), (0, 1), (0, 1), (0, 1), (1, 1)]),
        &q(0, 1),
        &q(3, 2),
    );
    assert_eq!(
        result,
        Err(IntegrationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::IntermediateScalarBits
        )))
    );
}

#[test]
fn rejects_oversized_bounds_before_evaluation() {
    let mut limits = ResourceLimits::default();
    limits.max_scalar_bits = 4;
    let context = EvaluationContext::new(limits);
    let polynomial = p(&[(1, 1)]);
    assert_eq!(
        context.integrate_polynomial_between(&polynomial, &q(0, 1), &q(200, 1)),
        Err(IntegrationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::ScalarBits
        )))
    );
}

#[test]
fn accepts_polynomial_expression_with_explicit_full_domain() {
    let context = EvaluationContext::default();
    let (symbol, variable) = x();
    let expression = Expr::add([
        Expr::pow(symbol.clone(), Expr::from(3)),
        Expr::mul([Expr::from(2), symbol]),
        Expr::from(-5),
    ]);
    let domain = DomainExpression::from_expr(expression, 32).unwrap();
    let result = context
        .integrate_domain_expression(&domain, &variable)
        .unwrap();
    assert_eq!(
        result.primitive(),
        &p(&[(0, 1), (-5, 1), (1, 1), (0, 1), (1, 4)])
    );
    let definite = context
        .integrate_domain_expression_between(&domain, &variable, &q(-1, 1), &q(2, 1))
        .unwrap();
    assert_eq!(definite.value(), &q(-33, 4));
    context
        .verify_definite_polynomial_integral(&definite)
        .unwrap();
}

#[test]
fn unsupported_other_variable_is_explicit() {
    let context = EvaluationContext::default();
    let (_, variable) = x();
    let y = DomainExpression::from_expr(Expr::from(Symbol::new("y").unwrap()), 10).unwrap();
    assert_eq!(
        context.integrate_domain_expression(&y, &variable),
        Err(IntegrationError::UnsupportedIntegrand(
            PolynomialConversionError::OtherSymbol
        ))
    );
}

#[test]
fn rejects_negative_zero_and_noninteger_powers_without_guessing() {
    let context = EvaluationContext::default();
    let (symbol, variable) = x();
    for power in [-1, 0] {
        let expr = Expr::pow(symbol.clone(), Expr::from(power));
        let domain = DomainExpression::from_expr(expr, 20).unwrap();
        assert_eq!(
            context.integrate_domain_expression(&domain, &variable),
            Err(IntegrationError::RestrictedDomain)
        );
    }
    let symbolic_power = Expr::pow(symbol, Expr::from(Symbol::new("n").unwrap()));
    assert!(DomainExpression::from_expr(symbolic_power, 20).is_err());
}

#[test]
fn preserves_source_exclusions_even_after_zero_product_normalization() {
    let context = EvaluationContext::default();
    let (symbol, variable) = x();
    let inverse = DomainExpression::from_expr(Expr::pow(symbol, Expr::from(-1)), 20).unwrap();
    let zero = DomainExpression::from_expr(Expr::from(0), 20).unwrap();
    let product = DomainExpression::multiply([zero, inverse], 20).unwrap();
    assert_eq!(product.expression(), &Expr::from(0));
    assert!(!product.conditions().is_empty());
    assert_eq!(
        context.integrate_domain_expression(&product, &variable),
        Err(IntegrationError::RestrictedDomain)
    );
    assert_eq!(
        context.integrate_domain_expression_between(&product, &variable, &q(1, 1), &q(2, 1)),
        Err(IntegrationError::RestrictedDomain)
    );
}

#[test]
fn result_and_verification_are_deterministic_and_operands_unchanged() {
    let context = EvaluationContext::default();
    let input = p(&[(9, 8), (-11, 12), (13, 7)]);
    let copy = input.clone();
    let a = context.integrate_polynomial(&input).unwrap();
    let b = context.integrate_polynomial(&input).unwrap();
    let definite_a = context
        .integrate_polynomial_between(&input, &q(-9, 4), &q(17, 6))
        .unwrap();
    let definite_b = context
        .integrate_polynomial_between(&input, &q(-9, 4), &q(17, 6))
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(definite_a, definite_b);
    assert_eq!(input, copy);
    context.verify_polynomial_primitive(&a).unwrap();
    context
        .verify_definite_polynomial_integral(&definite_a)
        .unwrap();
}

#[test]
fn large_exact_integer_coefficients_are_not_narrowed() {
    let context = EvaluationContext::default();
    let huge = Rational::from(Integer::from(u128::MAX));
    let input = RationalPolynomial::new(vec![huge.clone(), huge]);
    let primitive = context.integrate_polynomial(&input).unwrap();
    assert_eq!(
        primitive.primitive().coefficients()[2],
        q(1, 2) * Rational::from(Integer::from(u128::MAX))
    );
    assert_eq!(primitive.primitive().differentiate(), input);
}
