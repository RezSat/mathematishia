use mathema::{Condition, DomainExpression, Expr, ScalarDomain, Symbol};
use mathematishia_kernel::{
    DerivativeRuleId, EvaluationContext, EvaluationError, SymbolicDifferentiationError,
    verify_symbolic_derivative,
};

fn x() -> Expr { Expr::from(Symbol::new("x").unwrap()) }
fn selected() -> Symbol { Symbol::new("x").unwrap() }

#[test]
fn entire_functions_have_exact_checked_chain_rules() {
    let ctx = EvaluationContext::default();
    let argument = Expr::pow(x(), Expr::from(2));
    let input = Expr::sin(argument.clone());
    let result = ctx.differentiate_expression(&input, &selected()).unwrap();
    assert_eq!(
        result.derivative(),
        &Expr::mul([Expr::from(2),x(),Expr::cos(argument)])
    );
    assert_eq!(result.steps().last().unwrap().rule(),DerivativeRuleId::SineChain);
    assert!(result.conditions().is_empty());
    assert_eq!(verify_symbolic_derivative(&result),Ok(()));

    let composed=Expr::exp(Expr::cos(x()));
    let result=ctx.differentiate_expression(&composed,&selected()).unwrap();
    assert_eq!(
        result.derivative(),
        &Expr::mul([Expr::from(-1),Expr::exp(Expr::cos(x())),Expr::sin(x())])
    );
    assert_eq!(verify_symbolic_derivative(&result),Ok(()));
}

#[test]
fn logarithm_and_tangent_require_real_domain_conditions() {
    let ctx=EvaluationContext::default();
    let expr=Expr::tan(Expr::log(x()));
    assert_eq!(
        ctx.differentiate_expression(&expr,&selected()).unwrap_err(),
        EvaluationError::SymbolicDifferentiation(SymbolicDifferentiationError::UnsupportedFunction)
    );
    let source=DomainExpression::from_expr_in_domain(
        expr,ScalarDomain::Real,128,16,
    ).unwrap();
    assert_eq!(source.conditions(),&[
        Condition::NonZero(Expr::cos(Expr::log(x()))),
        Condition::Positive(x()),
    ]);
    let result=ctx.differentiate_domain_expression(
        &source,&selected(),ScalarDomain::Real,
    ).unwrap();
    assert_eq!(result.domain(),Some(ScalarDomain::Real));
    assert_eq!(result.conditions(),source.conditions());
    assert_eq!(
        result.derivative(),
        &Expr::mul([
            Expr::pow(Expr::cos(Expr::log(x())),Expr::from(-2)),
            Expr::pow(x(),Expr::from(-1)),
        ])
    );
    assert_eq!(verify_symbolic_derivative(&result),Ok(()));
}

#[test]
fn complex_log_is_explicitly_excluded_from_derivative_contract() {
    let ctx=EvaluationContext::default();
    let logarithm=DomainExpression::from_expr_in_domain(
        Expr::log(x()),ScalarDomain::Complex,16,8,
    ).unwrap();
    assert_eq!(
        ctx.differentiate_domain_expression(
            &logarithm,&selected(),ScalarDomain::Complex
        ),
        Err(EvaluationError::SymbolicDifferentiation(
            SymbolicDifferentiationError::UnsupportedFunction
        ))
    );
    let tangent=DomainExpression::from_expr_in_domain(
        Expr::tan(x()),ScalarDomain::Complex,16,8,
    ).unwrap();
    let result=ctx.differentiate_domain_expression(
        &tangent,&selected(),ScalarDomain::Complex
    ).unwrap();
    assert_eq!(verify_symbolic_derivative(&result),Ok(()));
}

#[test]
fn condition_mismatch_is_not_silently_accepted() {
    let ctx=EvaluationContext::default();
    let complex=DomainExpression::from_expr_in_domain(
        Expr::log(x()),ScalarDomain::Complex,16,8,
    ).unwrap();
    assert_eq!(
        ctx.differentiate_domain_expression(&complex,&selected(),ScalarDomain::Real),
        Err(EvaluationError::SymbolicDifferentiation(
            SymbolicDifferentiationError::MissingDomainCondition
        ))
    );
}

#[test]
fn unknown_functions_and_fractional_powers_stay_unsupported() {
    let ctx=EvaluationContext::default();
    let unknown=Expr::function(Symbol::new("custom").unwrap(),x());
    assert_eq!(
        ctx.differentiate_expression(&unknown,&selected()),
        Err(EvaluationError::SymbolicDifferentiation(SymbolicDifferentiationError::UnsupportedFunction))
    );
    assert_eq!(
        ctx.differentiate_expression(&Expr::pow(x(),Expr::from(-1)),&selected()),
        Err(EvaluationError::SymbolicDifferentiation(SymbolicDifferentiationError::UnsupportedPower))
    );
}
