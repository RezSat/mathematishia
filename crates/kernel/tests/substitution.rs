use mathema::{Expr, Integer, Rational, Symbol};
use mathematishia_kernel::substitute_symbol;

fn sym(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

fn expr_sym(name: &str) -> Expr {
    Expr::from(sym(name))
}

fn rational_expr(n: i64, d: i64) -> Expr {
    Expr::from(Rational::new(Integer::from(n), Integer::from(d)).unwrap())
}

#[test]
fn replaces_target_symbol() {
    let x = sym("x");
    assert_eq!(
        substitute_symbol(&expr_sym("x"), &x, &Expr::from(2)),
        Expr::from(2)
    );
}

#[test]
fn target_absent_leaves_expression_unchanged() {
    let x = sym("x");
    let expr = Expr::add([expr_sym("y"), 2.into()]);
    assert_eq!(substitute_symbol(&expr, &x, &Expr::from(2)), expr);
}

#[test]
fn non_target_symbol_is_unchanged() {
    let x = sym("x");
    let y = expr_sym("y");
    assert_eq!(substitute_symbol(&y, &x, &Expr::from(2)), y);
}

#[test]
fn scalars_are_unchanged() {
    let x = sym("x");
    let replacement = expr_sym("y");
    assert_eq!(
        substitute_symbol(&Expr::from(3), &x, &replacement),
        Expr::from(3)
    );
    assert_eq!(
        substitute_symbol(&rational_expr(1, 2), &x, &replacement),
        rational_expr(1, 2)
    );
}

#[test]
fn substitutes_inside_add() {
    let x = sym("x");
    let y = expr_sym("y");
    let expr = Expr::add([expr_sym("x"), y.clone()]);
    assert_eq!(
        substitute_symbol(&expr, &x, &Expr::from(2)),
        Expr::add([Expr::from(2), y])
    );
}

#[test]
fn substitutes_inside_mul() {
    let x = sym("x");
    let y = expr_sym("y");
    let expr = Expr::mul([expr_sym("x"), y.clone()]);
    assert_eq!(
        substitute_symbol(&expr, &x, &Expr::from(2)),
        Expr::mul([Expr::from(2), y])
    );
}

#[test]
fn substitutes_pow_base_and_exponent() {
    let x = sym("x");
    let y = sym("y");
    let base_case = Expr::pow(expr_sym("x"), expr_sym("y"));
    assert_eq!(
        substitute_symbol(&base_case, &x, &Expr::from(2)),
        Expr::pow(Expr::from(2), expr_sym("y"))
    );
    assert_eq!(
        substitute_symbol(&base_case, &y, &Expr::from(2)),
        Expr::pow(expr_sym("x"), Expr::from(2))
    );
    let both = Expr::pow(expr_sym("x"), expr_sym("x"));
    assert_eq!(
        substitute_symbol(&both, &x, &Expr::from(2)),
        Expr::pow(Expr::from(2), Expr::from(2))
    );
}

#[test]
fn replaces_repeated_occurrences() {
    let x = sym("x");
    let expr = Expr::mul([expr_sym("x"), expr_sym("x")]);
    let expected = Expr::mul([expr_sym("y"), expr_sym("y")]);
    assert_eq!(substitute_symbol(&expr, &x, &expr_sym("y")), expected);
}

#[test]
fn constructor_folding_applies_to_results() {
    let x = sym("x");
    let sum = Expr::add([expr_sym("x"), 2.into()]);
    assert_eq!(substitute_symbol(&sum, &x, &Expr::from(3)), Expr::from(5));
    let product = Expr::mul([expr_sym("x"), expr_sym("x")]);
    assert_eq!(
        substitute_symbol(&product, &x, &Expr::from(2)),
        Expr::from(4)
    );
}

#[test]
fn replacement_is_not_recursively_substituted() {
    let x = sym("x");
    let replacement = Expr::add([expr_sym("x"), 1.into()]);
    assert_eq!(
        substitute_symbol(&expr_sym("x"), &x, &replacement),
        replacement
    );
    let expr = Expr::mul([expr_sym("x"), expr_sym("y")]);
    assert_eq!(
        substitute_symbol(&expr, &x, &replacement),
        Expr::mul([replacement, expr_sym("y")])
    );
}

#[test]
fn self_substitution_is_structurally_unchanged() {
    let x = sym("x");
    let expr = Expr::add([expr_sym("x"), expr_sym("y")]);
    assert_eq!(substitute_symbol(&expr, &x, &expr_sym("x")), expr);
}

#[test]
fn existing_zero_one_and_exponent_one_rules_apply() {
    let x = sym("x");
    let y = expr_sym("y");
    assert_eq!(
        substitute_symbol(&Expr::add([expr_sym("x"), y.clone()]), &x, &Expr::from(0)),
        y
    );
    assert_eq!(
        substitute_symbol(&Expr::mul([expr_sym("x"), y.clone()]), &x, &Expr::from(1)),
        y
    );
    assert_eq!(
        substitute_symbol(&Expr::mul([expr_sym("x"), y]), &x, &Expr::from(0)),
        Expr::from(0)
    );
    assert_eq!(
        substitute_symbol(
            &Expr::pow(expr_sym("x"), expr_sym("y")),
            &sym("y"),
            &Expr::from(1)
        ),
        expr_sym("x")
    );
}
