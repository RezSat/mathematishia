use mathema::{DefinednessError, DomainExpression, Expr, Symbol};
use mathematishia_kernel::{EvaluationContext, PolynomialConversionError, RationalPolynomial, substitute_symbol};

fn symbol(name: &str) -> Symbol { Symbol::new(name).unwrap() }

#[test]
fn analytic_calls_are_not_silently_treated_as_polynomials_or_defined_scalars() {
    let nested = Expr::sin(Expr::log(Expr::from(symbol("x"))));
    assert_eq!(DomainExpression::from_expr(nested.clone(), 64), Err(DefinednessError::UnsupportedFunction));
    assert_eq!(RationalPolynomial::try_from_expr(&nested, &symbol("x"), 32), Err(PolynomialConversionError::UnsupportedFunction));
    assert!(EvaluationContext::default().check_expression(&nested).is_ok());
}

#[test]
fn exact_substitution_recurses_through_unevaluated_calls() {
    let x = symbol("x");
    let y = Expr::from(symbol("y"));
    let original = Expr::log(Expr::sin(Expr::add([Expr::from(x.clone()), y.clone()])));
    let result = substitute_symbol(&original, &x, &Expr::from(2));
    assert_eq!(result, Expr::log(Expr::sin(Expr::add([Expr::from(2), y]))));
    assert_eq!(substitute_symbol(&Expr::cos(Expr::from(x.clone())), &x, &Expr::from(x)), Expr::cos(Expr::from(symbol("x"))));
}

#[test]
fn unknown_unary_function_is_not_implicitly_evaluated() {
    let x = Expr::from(symbol("x"));
    let call = Expr::function(symbol("other"), x.clone());
    assert!(EvaluationContext::default().check_expression(&call).is_ok());
    assert_eq!(DomainExpression::from_expr(call, 64), Err(DefinednessError::UnsupportedFunction));
    assert_ne!(Expr::sin(x.clone()), x);
}
