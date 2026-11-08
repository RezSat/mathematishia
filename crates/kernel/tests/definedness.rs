use mathema::{Condition, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    Assumptions, ConditionStatus, DefinednessError, DomainExpression, definedness_obligations,
};

fn symbol(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

#[test]
fn detects_negative_and_zero_power_obligations() {
    let x = symbol("x");
    let inverse = Expr::pow(x.clone(), (-1).into());
    assert_eq!(
        definedness_obligations(&inverse, 32),
        Ok(vec![Condition::NonZero(x.clone())])
    );
    assert_eq!(
        definedness_obligations(&Expr::pow(x.clone(), 0.into()), 32),
        Ok(vec![Condition::NonZero(x.clone())])
    );
    assert_eq!(
        definedness_obligations(&Expr::pow(x.clone(), 3.into()), 32),
        Ok(vec![])
    );
    assert_eq!(
        definedness_obligations(&Expr::pow(Expr::from(0), 0.into()), 32),
        Ok(vec![Condition::NonZero(Expr::from(0))])
    );
    let nested = Expr::pow(inverse.clone(), (-1).into());
    assert_eq!(
        definedness_obligations(&nested, 32),
        Ok(vec![Condition::NonZero(inverse), Condition::NonZero(x)])
    );
}

#[test]
fn rejects_unknown_power_semantics_and_limits_work() {
    let x = symbol("x");
    let square_root = Expr::pow(
        x.clone(),
        Rational::new(Integer::one(), Integer::from(2))
            .unwrap()
            .into(),
    );
    assert_eq!(
        definedness_obligations(&square_root, 100),
        Err(DefinednessError::UnsupportedExponent)
    );
    assert_eq!(
        definedness_obligations(&Expr::pow(x.clone(), symbol("n")), 100),
        Err(DefinednessError::UnsupportedExponent)
    );
    assert_eq!(
        definedness_obligations(&Expr::pow(x, (-1).into()), 1),
        Err(DefinednessError::NodeLimitExceeded)
    );
}

#[test]
fn powers_and_products_inherit_nonzero_premises() {
    let x = symbol("x");
    let y = symbol("y");
    let assumptions = Assumptions::from_conditions([
        Condition::NonZero(x.clone()),
        Condition::Positive(y.clone()),
    ])
    .unwrap();
    assert_eq!(
        assumptions.assess(&Condition::NonZero(Expr::pow(x.clone(), (-2).into()))),
        ConditionStatus::Assumed
    );
    assert_eq!(
        assumptions.assess(&Condition::NonZero(Expr::mul([x.clone(), y.clone()]))),
        ConditionStatus::Assumed
    );
    assert_eq!(
        assumptions.assess(&Condition::NonZero(Expr::add([x, y]))),
        ConditionStatus::Unknown
    );
}

#[test]
fn retains_domain_holes_through_zero_multiplication() {
    let x = symbol("x");
    let zero = DomainExpression::from_expr(Expr::from(0), 32).unwrap();
    let reciprocal = DomainExpression::from_expr(Expr::pow(x.clone(), (-1).into()), 32)
        .unwrap();
    let product = DomainExpression::multiply([zero, reciprocal], 4).unwrap();

    assert_eq!(product.expression(), &Expr::from(0));
    assert_eq!(product.conditions(), &[Condition::NonZero(x)]);
    assert_eq!(
        DomainExpression::multiply(
            [
                product.clone(),
                DomainExpression::from_expr(Expr::from(1), 32).unwrap()
            ],
            0
        ),
        Err(DefinednessError::ConditionLimitExceeded)
    );
}

#[test]
fn composition_keeps_restrictions_after_folding() {
    let x = symbol("x");
    let input = DomainExpression::from_expr(Expr::pow(x.clone(), (-1).into()), 32).unwrap();
    let sum = DomainExpression::add(
        [
            input,
            DomainExpression::from_expr(Expr::from(2), 32).unwrap(),
        ],
        4,
    )
    .unwrap();
    assert_eq!(sum.conditions(), &[Condition::NonZero(x.clone())]);
    let inverse = sum.pow_integer(Integer::from(-1), 4).unwrap();
    assert_eq!(inverse.conditions().len(), 2);
    assert_eq!(
        inverse.conditions().first(),
        Some(&Condition::NonZero(x))
    );
}
