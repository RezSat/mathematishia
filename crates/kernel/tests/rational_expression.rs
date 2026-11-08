use mathema::{Condition, DomainExpression, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{EvaluationContext, RationalExpressionError, RationalFunctionError};

fn symbol(value: &str) -> Symbol {
    Symbol::new(value).unwrap()
}

fn domain(expression: &str) -> DomainExpression {
    // Tests use the same raw semantic graph as the frontend, plus explicit
    // source-domain conditions when construction would discard factors.
    let x = Expr::from(symbol("x"));
    match expression {
        "canceled" => DomainExpression::from_expr(
            Expr::mul([
                Expr::add([Expr::pow(x.clone(), 2.into()), (-1).into()]),
                Expr::pow(Expr::add([x, (-1).into()]), (-1).into()),
            ]),
            128,
        )
        .unwrap(),
        "zero_with_hole" => DomainExpression::multiply(
            [
                DomainExpression::from_expr(Expr::from(0), 128).unwrap(),
                DomainExpression::from_expr(
                    Expr::pow(Expr::add([x, (-1).into()]), (-1).into()),
                    128,
                )
                .unwrap(),
            ],
            16,
        )
        .unwrap(),
        _ => panic!("unknown example"),
    }
}

#[test]
fn cancels_rational_polynomial_factors_but_keeps_exclusions() {
    let ctx = EvaluationContext::default();
    let value = ctx
        .rational_expression(&domain("canceled"), &symbol("x"))
        .unwrap();
    assert_eq!(
        value.numerator().to_expr(&symbol("x")),
        Expr::add([1.into(), Expr::from(symbol("x"))])
    );
    assert_eq!(value.denominator().to_expr(&symbol("x")), 1.into());
    assert_eq!(value.excluded_zero_sets().len(), 1);
    assert_eq!(
        value.evaluate(&Rational::from(Integer::from(1))),
        Err(RationalFunctionError::UndefinedAtPoint)
    );
    assert_eq!(
        value.evaluate(&Rational::from(Integer::from(2))),
        Ok(Rational::from(Integer::from(3)))
    );
}

#[test]
fn constant_zero_preserves_a_removed_domain_hole() {
    let ctx = EvaluationContext::default();
    let value = ctx
        .rational_expression(&domain("zero_with_hole"), &symbol("x"))
        .unwrap();
    assert!(value.is_zero());
    assert_eq!(value.excluded_zero_sets().len(), 1);
    assert_eq!(
        value.evaluate(&Rational::from(Integer::from(1))),
        Err(RationalFunctionError::UndefinedAtPoint)
    );
}

#[test]
fn impossible_domain_is_rejected() {
    let x = Expr::from(symbol("x"));
    let value = DomainExpression::with_conditions(
        Expr::from(0),
        [Condition::NonZero(Expr::from(0)), Condition::NonZero(x)],
        128,
        8,
    )
    .unwrap();
    assert_eq!(
        EvaluationContext::default().rational_expression(&value, &symbol("x")),
        Err(RationalExpressionError::RationalFunction(
            RationalFunctionError::EmptyDomain
        ))
    );
}

#[test]
fn rejects_multivariate_and_nonpolynomial_domains() {
    let x = Expr::from(symbol("x"));
    let y = Expr::from(symbol("y"));
    let value = DomainExpression::from_expr(Expr::add([x.clone(), y]), 128).unwrap();
    assert_eq!(
        EvaluationContext::default().rational_expression(&value, &symbol("x")),
        Err(RationalExpressionError::OtherSymbol)
    );
    let symbolic = DomainExpression::from_expr(Expr::pow(x, Expr::from(symbol("n"))), 128);
    assert!(symbolic.is_err());
}
