use mathema::{Equation, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{EvaluationContext, EvaluationError, ResourceLimitKind, ResourceLimits};

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}
fn symbol() -> Expr {
    Expr::from(x())
}
fn limited() -> EvaluationContext {
    EvaluationContext::default()
}

#[test]
fn default_context_is_explicit_and_deterministic() {
    let a = EvaluationContext::default();
    let b = EvaluationContext::new(ResourceLimits::default());
    assert_eq!(a, b);
    assert_eq!(a.limits().max_polynomial_degree, 32);
    assert_eq!(a.limits().max_distribution_steps, 12);
}

#[test]
fn node_limit_applies_to_each_semantic_occurrence() {
    let expr = Expr::add([symbol(), Expr::pow(symbol(), 2.into())]);
    let ctx = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 3,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.check_expression(&expr),
        Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes))
    );
    assert_eq!(limited().check_expression(&expr), Ok(()));
}

#[test]
fn depth_limit_traverses_powers_without_recursion() {
    let expr = Expr::pow(Expr::pow(symbol(), 2.into()), 3.into());
    let ctx = EvaluationContext::new(ResourceLimits {
        max_expression_depth: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.check_expression(&expr),
        Err(EvaluationError::Limit(ResourceLimitKind::ExpressionDepth))
    );
}

#[test]
fn scalar_limits_reject_large_inputs_and_generated_outputs() {
    let ctx = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 8,
        ..ResourceLimits::default()
    });
    let large = Expr::from(Integer::from(65_536));
    assert_eq!(
        ctx.check_expression(&large),
        Err(EvaluationError::Limit(ResourceLimitKind::ScalarBits))
    );
    let pow = Expr::pow(Expr::from(2), Expr::from(12));
    assert_eq!(
        ctx.polynomial(&pow, &x()),
        Err(EvaluationError::Limit(ResourceLimitKind::ScalarBits))
    );
}

#[test]
fn polynomial_degree_limit_is_typed_and_distinct_from_unsupported_input() {
    let ctx = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 2,
        ..ResourceLimits::default()
    });
    let cubic = Expr::pow(symbol(), 3.into());
    assert!(matches!(
        ctx.polynomial(&cubic, &x()),
        Err(EvaluationError::Polynomial(_))
    ));
    let quadratic = Expr::pow(symbol(), 2.into());
    assert!(ctx.polynomial(&quadratic, &x()).is_ok());
}

#[test]
fn checked_distribution_reports_partial_completion_and_validates_steps() {
    let expr = Expr::mul([symbol(), Expr::add([symbol(), 2.into()])]);
    let zero = limited().with_distribution_steps(0).expand(&expr).unwrap();
    assert!(zero.steps().is_empty());
    assert!(!zero.is_complete());
    let one = limited().with_distribution_steps(1).expand(&expr).unwrap();
    assert_eq!(one.steps().len(), 1);
    assert!(one.is_complete());
    assert_eq!(limited().expand(&Expr::from(3)).unwrap().steps().len(), 0);
}

#[test]
fn checked_expansion_rejects_growth_at_step_boundary() {
    let input = Expr::mul([
        Expr::add([symbol(), 1.into()]),
        Expr::add([symbol(), 2.into()]),
    ]);
    let ctx = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 8,
        ..ResourceLimits::default()
    });
    assert!(matches!(
        ctx.expand(&input),
        Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes))
    ));
}

#[test]
fn verified_linear_solver_and_arithmetic_use_same_context() {
    let eq = Equation::new(
        Expr::add([Expr::mul([2.into(), symbol()]), 4.into()]),
        Expr::from(10),
    );
    let ctx = limited();
    let solved = ctx.solve_linear(&eq, &x()).unwrap();
    assert_eq!(
        solved.solution(),
        &mathematishia_kernel::LinearSolution::Unique(Rational::from(Integer::from(3)))
    );
    let poly = ctx
        .polynomial(&Expr::add([Expr::pow(symbol(), 2.into()), 1.into()]), &x())
        .unwrap();
    assert_eq!(
        ctx.evaluate_polynomial(&poly, &Rational::from(Integer::one()))
            .unwrap(),
        Rational::from(Integer::from(2))
    );
    let (quotient, remainder) = ctx.divide(&poly, &poly).unwrap();
    assert!(remainder.is_zero());
    assert_eq!(quotient.degree(), Some(0));
    assert_eq!(ctx.gcd(&poly, &poly).unwrap(), poly);
}

#[test]
fn checked_polynomial_division_rejects_zero_divisor() {
    let ctx = limited();
    let poly = ctx.polynomial(&symbol(), &x()).unwrap();
    assert_eq!(
        ctx.divide(&poly, &mathematishia_kernel::RationalPolynomial::zero()),
        Err(EvaluationError::DivisionByZero)
    );
}

#[test]
fn linear_solver_respects_context_degree_even_if_high_terms_cancel() {
    let cubic = Expr::pow(symbol(), 3.into());
    let equation = Equation::new(Expr::add([cubic.clone(), symbol()]), cubic);
    let ctx = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 1,
        ..ResourceLimits::default()
    });
    assert!(matches!(
        ctx.solve_linear(&equation, &x()),
        Err(EvaluationError::Polynomial(_))
    ));
}
