use mathema::{Expr, ExprView, Integer, Rational, Symbol};
use mathematishia_kernel::{Relation, RuleId, distribute_product_once};

fn expr_sym(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

fn rational_expr(n: i64, d: i64) -> Expr {
    Expr::from(Rational::new(Integer::from(n), Integer::from(d)).unwrap())
}

fn has_mul_with_add_factor(expr: &Expr) -> bool {
    match expr.view() {
        ExprView::Add(children) => children.iter().any(has_mul_with_add_factor),
        ExprView::Mul(factors) => {
            factors
                .iter()
                .any(|factor| matches!(factor.view(), ExprView::Add(_)))
                || factors.iter().any(has_mul_with_add_factor)
        }
        ExprView::Pow { base, exponent } => {
            has_mul_with_add_factor(base) || has_mul_with_add_factor(exponent)
        }
        ExprView::Integer(_) | ExprView::Rational(_) | ExprView::Symbol(_) => false,
    }
}

#[test]
fn distributes_a_symbolic_product_over_a_sum() {
    let x = expr_sym("x");
    let y = expr_sym("y");
    let before = Expr::mul([x.clone(), Expr::add([y.clone(), 2.into()])]);

    let step = distribute_product_once(&before).expect("the product contains a sum");

    assert_eq!(step.rule(), RuleId::DistributeProductOverSum);
    assert_eq!(
        step.rule().stable_id(),
        "algebra.distribute_product_over_sum"
    );
    assert_eq!(step.relation(), Relation::Equal);
    assert_eq!(step.before(), &before);
    assert_eq!(
        step.after(),
        &Expr::add([Expr::mul([x.clone(), y]), Expr::mul([x, 2.into()])])
    );
}

#[test]
fn folds_scalar_products_exactly() {
    let x = expr_sym("x");
    let before = Expr::mul([2.into(), Expr::add([x.clone(), 3.into()])]);

    let step = distribute_product_once(&before).expect("the product contains a sum");

    assert_eq!(
        step.after(),
        &Expr::add([6.into(), Expr::mul([2.into(), x])])
    );
}

#[test]
fn keeps_exact_rational_coefficients() {
    let x = expr_sym("x");
    let half = rational_expr(1, 2);
    let third = rational_expr(1, 3);
    let before = Expr::mul([half.clone(), Expr::add([third, x.clone()])]);

    let step = distribute_product_once(&before).expect("the product contains a sum");

    assert_eq!(
        step.after(),
        &Expr::add([rational_expr(1, 6), Expr::mul([half, x])])
    );
}

#[test]
fn selects_the_first_add_factor_in_canonical_order() {
    let a = expr_sym("a");
    let x = expr_sym("x");
    let y = expr_sym("y");
    let z = expr_sym("z");
    let first_sum = Expr::add([x, y]);
    let second_sum = Expr::add([z.clone(), 1.into()]);
    let before = Expr::mul([a.clone(), first_sum.clone(), second_sum]);

    let step = distribute_product_once(&before).expect("the product contains sums");

    // Canonical order places (1 + z) before (x + y), so the former is selected.
    let remaining = Expr::mul([a, first_sum.clone()]);
    assert_eq!(
        step.after(),
        &Expr::add([Expr::mul([remaining.clone(), z]), remaining])
    );

    assert!(has_mul_with_add_factor(step.after()));
}

#[test]
fn returns_none_without_a_top_level_product_over_a_sum() {
    let x = expr_sym("x");
    let y = expr_sym("y");
    let nested_sum = Expr::pow(Expr::add([y.clone(), 1.into()]), 2.into());

    for expr in [
        Expr::from(3),
        rational_expr(1, 2),
        x.clone(),
        Expr::add([x.clone(), y.clone()]),
        Expr::pow(x.clone(), 2.into()),
        Expr::mul([x.clone(), y]),
        Expr::mul([x, nested_sum]),
    ] {
        assert_eq!(distribute_product_once(&expr), None);
    }
}

#[test]
fn applies_exactly_one_step() {
    let a = expr_sym("a");
    let x = expr_sym("x");
    let y = expr_sym("y");
    let z = expr_sym("z");
    let first_sum = Expr::add([x, y]);
    let second_sum = Expr::add([z.clone(), 1.into()]);
    let before = Expr::mul([a.clone(), first_sum.clone(), second_sum]);

    let step = distribute_product_once(&before).expect("the product contains sums");

    let remaining = Expr::mul([a, first_sum]);
    assert_eq!(
        step.after(),
        &Expr::add([Expr::mul([remaining.clone(), z]), remaining])
    );
    assert!(
        has_mul_with_add_factor(step.after()),
        "one step leaves the other sum unexpanded"
    );
}

#[test]
fn canonical_operand_order_produces_identical_steps() {
    let a = expr_sym("a");
    let first_sum = Expr::add([expr_sym("x"), expr_sym("y")]);
    let second_sum = Expr::add([expr_sym("z"), 1.into()]);
    let operands = [a, first_sum, second_sum];

    let mut steps = Vec::new();
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let before = Expr::mul(order.map(|index| operands[index].clone()));
        steps.push(distribute_product_once(&before).expect("the product contains sums"));
    }

    for step in &steps[1..] {
        assert_eq!(step, &steps[0]);
    }
}

#[test]
fn leaves_the_input_and_caller_held_values_unchanged() {
    let x = expr_sym("x");
    let sum = Expr::add([expr_sym("y"), 2.into()]);
    let x_snapshot = x.clone();
    let sum_snapshot = sum.clone();
    let before = Expr::mul([x.clone(), sum.clone()]);
    let before_snapshot = before.clone();

    let step = distribute_product_once(&before).expect("the product contains a sum");

    assert_eq!(x, x_snapshot);
    assert_eq!(sum, sum_snapshot);
    assert_eq!(before, before_snapshot);
    assert_eq!(step.before(), &before_snapshot);
    assert_ne!(step.after(), step.before());
}
