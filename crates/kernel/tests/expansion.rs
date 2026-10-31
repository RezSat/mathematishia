use mathema::{Expr, ExprView, Integer, Rational, Symbol};
use mathematishia_kernel::{
    Relation, RuleId, distribute_first_once, distribute_product_once, expand_distributively,
};

fn symbol(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

fn rational(n: i64, d: i64) -> Expr {
    Expr::from(Rational::new(Integer::from(n), Integer::from(d)).unwrap())
}

fn two_sums() -> Expr {
    Expr::mul([
        Expr::add([symbol("a"), symbol("b")]),
        Expr::add([symbol("c"), symbol("d")]),
    ])
}

#[test]
fn root_distribution_precedes_any_nested_work() {
    let nested = Expr::mul([symbol("x"), Expr::add([symbol("y"), symbol("z")])]);
    let before = Expr::mul([
        Expr::add([symbol("a"), symbol("b")]),
        Expr::pow(nested, 2.into()),
    ]);

    assert_eq!(
        distribute_first_once(&before),
        distribute_product_once(&before)
    );
}

#[test]
fn distributes_inside_add_and_records_the_whole_expression() {
    let product = Expr::mul([symbol("x"), Expr::add([symbol("y"), symbol("z")])]);
    let before = Expr::add([product, symbol("w")]);
    assert_eq!(distribute_product_once(&before), None);

    let step = distribute_first_once(&before).expect("nested product is distributable");
    let expected = Expr::add([
        symbol("w"),
        Expr::mul([symbol("x"), symbol("y")]),
        Expr::mul([symbol("x"), symbol("z")]),
    ]);

    assert_eq!(step.before(), &before);
    assert_eq!(step.after(), &expected);
    assert_eq!(step.rule(), RuleId::DistributeProductOverSum);
    assert_eq!(step.relation(), Relation::Equal);
}

#[test]
fn selects_first_nested_child_in_canonical_order() {
    let left = Expr::mul([symbol("a"), Expr::add([symbol("b"), symbol("c")])]);
    let right = Expr::mul([symbol("x"), Expr::add([symbol("y"), symbol("z")])]);
    let before = Expr::add([right, left]);
    let ExprView::Add(children) = before.view() else {
        panic!("expected a sum of products");
    };

    let first_step = distribute_product_once(&children[0]).unwrap();
    let expected = Expr::add([first_step.after().clone(), children[1].clone()]);
    let step = distribute_first_once(&before).unwrap();
    assert_eq!(step.after(), &expected);
}

#[test]
fn expands_two_sums_with_contiguous_steps() {
    let before = two_sums();
    let expansion = expand_distributively(&before, 3);
    let expected = Expr::add([
        Expr::mul([symbol("a"), symbol("c")]),
        Expr::mul([symbol("a"), symbol("d")]),
        Expr::mul([symbol("b"), symbol("c")]),
        Expr::mul([symbol("b"), symbol("d")]),
    ]);

    assert!(expansion.is_complete());
    assert_eq!(expansion.steps().len(), 3);
    assert_eq!(expansion.result(), &expected);
    assert_eq!(expansion.steps()[0].before(), &before);
    for window in expansion.steps().windows(2) {
        assert_eq!(window[0].after(), window[1].before());
    }
    assert_eq!(
        expansion.steps().last().unwrap().after(),
        expansion.result()
    );
    for step in expansion.steps() {
        assert_eq!(
            step.rule().stable_id(),
            "algebra.distribute_product_over_sum"
        );
        assert_eq!(step.relation(), Relation::Equal);
    }
}

#[test]
fn distinguishes_zero_one_and_sufficient_budgets() {
    let before = two_sums();

    let none = expand_distributively(&before, 0);
    assert_eq!(none.result(), &before);
    assert!(none.steps().is_empty());
    assert!(!none.is_complete());

    let one = expand_distributively(&before, 1);
    assert_eq!(one.steps().len(), 1);
    assert!(!one.is_complete());
    assert_eq!(one.steps()[0].before(), &before);
    assert_eq!(one.steps()[0].after(), one.result());

    let enough = expand_distributively(&before, 3);
    assert_eq!(enough.steps().len(), 3);
    assert!(enough.is_complete());
}

#[test]
fn exact_scalar_folding_is_preserved() {
    let before = Expr::mul([2.into(), Expr::add([3.into(), symbol("x")])]);
    let expansion = expand_distributively(&before, 10);
    assert_eq!(
        expansion.result(),
        &Expr::add([6.into(), Expr::mul([2.into(), symbol("x")])])
    );
    assert_eq!(expansion.steps().len(), 1);
    assert!(expansion.is_complete());

    let half = rational(1, 2);
    let third = rational(1, 3);
    let before = Expr::mul([half.clone(), Expr::add([third, symbol("x")])]);
    let expansion = expand_distributively(&before, 3);
    assert_eq!(
        expansion.result(),
        &Expr::add([rational(1, 6), Expr::mul([half, symbol("x")])])
    );
    assert!(expansion.is_complete());
}

#[test]
fn completed_expression_has_zero_steps_even_with_zero_budget() {
    let before = Expr::add([symbol("x"), Expr::pow(symbol("y"), 2.into())]);
    for budget in [0, 1, 4] {
        let expansion = expand_distributively(&before, budget);
        assert_eq!(expansion.result(), &before);
        assert!(expansion.steps().is_empty());
        assert!(expansion.is_complete());
        assert_eq!(distribute_first_once(&before), None);
    }
}

#[test]
fn never_enters_powers() {
    let hidden = Expr::mul([symbol("x"), Expr::add([symbol("y"), symbol("z")])]);
    let before = Expr::pow(hidden, 2.into());
    assert_eq!(distribute_first_once(&before), None);
    let expansion = expand_distributively(&before, 10);
    assert_eq!(expansion.result(), &before);
    assert!(expansion.steps().is_empty());
    assert!(expansion.is_complete());
}

#[test]
fn original_nodes_remain_unchanged() {
    let sum = Expr::add([symbol("a"), symbol("b")]);
    let before = Expr::mul([sum.clone(), symbol("x")]);
    let held_sum = sum.clone();
    let held_before = before.clone();

    let expansion = expand_distributively(&before, 10);
    assert_eq!(sum, held_sum);
    assert_eq!(before, held_before);
    assert_eq!(expansion.steps()[0].before(), &held_before);
}

#[test]
fn permutation_invariance_includes_the_step_chain() {
    let a = symbol("a");
    let b = symbol("b");
    let c = symbol("c");
    let d = symbol("d");
    let first = Expr::add([a.clone(), b.clone()]);
    let second = Expr::add([c.clone(), d.clone()]);
    let candidates = [
        Expr::mul([first.clone(), second.clone()]),
        Expr::mul([second.clone(), first.clone()]),
        Expr::mul([Expr::add([b, a]), Expr::add([d, c])]),
    ];
    let expected = expand_distributively(&candidates[0], 3);
    for expr in &candidates[1..] {
        assert_eq!(expand_distributively(expr, 3), expected);
    }
}
