use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ScalarDomain, Symbol,
    definedness_obligations_in_domain,
};

fn x() -> Expr {
    Expr::from(Symbol::new("x").unwrap())
}

#[test]
fn nested_real_function_conditions_include_reciprocal_and_log() {
    let expr = Expr::tan(Expr::log(Expr::pow(x(), (-1).into())));
    assert_eq!(
        definedness_obligations_in_domain(&expr, 32, ScalarDomain::Real).unwrap(),
        vec![
            Condition::NonZero(Expr::cos(Expr::log(Expr::pow(x(), (-1).into())))),
            Condition::Positive(Expr::pow(x(), (-1).into())),
            Condition::NonZero(x()),
        ]
    );
    let complex = definedness_obligations_in_domain(&expr, 32, ScalarDomain::Complex).unwrap();
    assert_eq!(complex.len(), 3);
    assert_eq!(complex[1], Condition::NonZero(Expr::pow(x(), (-1).into())));
}

#[test]
fn unknown_functions_and_resource_limits_are_rejected() {
    let unknown = Expr::function(Symbol::new("other").unwrap(), x());
    assert_eq!(
        definedness_obligations_in_domain(&unknown, 32, ScalarDomain::Real),
        Err(DefinednessError::UnsupportedFunction)
    );
    assert_eq!(
        definedness_obligations_in_domain(&Expr::sin(x()), 1, ScalarDomain::Real),
        Err(DefinednessError::NodeLimitExceeded)
    );
    assert_eq!(
        DomainExpression::from_expr_in_domain(Expr::tan(Expr::log(x())), ScalarDomain::Real, 32, 1),
        Err(DefinednessError::ConditionLimitExceeded)
    );
}

#[test]
fn function_construction_preserves_inherited_domain_history() {
    let source = DomainExpression::from_expr(Expr::pow(x(), (-1).into()), 32).unwrap();
    let result =
        DomainExpression::function(Symbol::new("log").unwrap(), source, ScalarDomain::Real, 8)
            .unwrap();
    assert_eq!(
        result.conditions(),
        &[
            Condition::NonZero(x()),
            Condition::Positive(Expr::pow(x(), (-1).into())),
        ]
    );
}
