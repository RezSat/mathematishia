use mathema::{Expr, ExprView, Integer, Rational, Symbol};

fn sym(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

fn expr_sym(name: &str) -> Expr {
    Expr::from(sym(name))
}

fn rational(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

#[test]
fn integer_view_borrows_value() {
    let expr = Expr::from(Integer::from(7));
    assert!(matches!(expr.view(), ExprView::Integer(value) if value == &Integer::from(7)));
}

#[test]
fn rational_view_borrows_value() {
    let expr = Expr::from(rational(1, 2));
    assert!(matches!(expr.view(), ExprView::Rational(value) if value == &rational(1, 2)));
}

#[test]
fn symbol_view_borrows_value() {
    let expr = expr_sym("x");
    assert!(matches!(expr.view(), ExprView::Symbol(value) if value == &sym("x")));
}

#[test]
fn add_view_borrows_children() {
    let x = expr_sym("x");
    let expr = Expr::add([x.clone(), 2.into()]);
    let ExprView::Add(children) = expr.view() else {
        panic!("expected Add view");
    };
    assert_eq!(children, &[Expr::from(2), x]);
}

#[test]
fn mul_view_borrows_children() {
    let x = expr_sym("x");
    let expr = Expr::mul([x.clone(), 2.into()]);
    let ExprView::Mul(children) = expr.view() else {
        panic!("expected Mul view");
    };
    assert_eq!(children, &[Expr::from(2), x]);
}

#[test]
fn pow_view_borrows_base_and_exponent() {
    let base = expr_sym("x");
    let exponent = expr_sym("y");
    let expr = Expr::pow(base.clone(), exponent.clone());
    let ExprView::Pow {
        base: seen_base,
        exponent: seen_exponent,
    } = expr.view()
    else {
        panic!("expected Pow view");
    };
    assert_eq!(seen_base, &base);
    assert_eq!(seen_exponent, &exponent);
}

#[test]
fn whole_rational_expression_views_as_integer() {
    let expr = Expr::from(rational(6, 3));
    assert_eq!(expr, Expr::from(2));
    assert!(matches!(expr.view(), ExprView::Integer(value) if value == &Integer::from(2)));
}
