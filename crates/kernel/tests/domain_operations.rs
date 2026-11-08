use mathema::{Condition, DomainExpression, Expr, Symbol};
use mathematishia_kernel::{
    Assumptions, ConditionStatus, DomainRewriteOutcome, rewrite_domain_once, substitute_domain,
    verify_proof_graph,
};

fn sym(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

fn domain(expression: Expr) -> DomainExpression {
    DomainExpression::from_expr(expression, 128).unwrap()
}

#[test]
fn checked_rewrite_retains_original_domain() {
    let x = sym("x");
    let expr = Expr::mul([x.clone(), Expr::pow(x.clone(), (-1).into())]);
    let original = domain(expr.clone());
    let empty = Assumptions::new();
    assert_eq!(
        rewrite_domain_once(&original, &empty).unwrap(),
        DomainRewriteOutcome::Blocked(vec![Condition::NonZero(x.clone())])
    );
    let premises = Assumptions::from_conditions([Condition::NonZero(x.clone())]).unwrap();
    let DomainRewriteOutcome::Applied(rewrite) =
        rewrite_domain_once(&original, &premises).unwrap()
    else {
        panic!("expected a checked cancellation");
    };
    assert_eq!(rewrite.result().expression(), &Expr::from(1));
    assert_eq!(rewrite.result().conditions(), &[Condition::NonZero(x)]);
    assert_eq!(rewrite.rule_id(), Some("algebra.cancel_symbol_inverse"));
    assert_eq!(verify_proof_graph(&expr, &premises, rewrite.proof()), Ok(()));
}

#[test]
fn scoped_composite_rule_uses_existing_independent_verifier() {
    let x = sym("x");
    let base = Expr::add([x.clone(), 1.into()]);
    let original = domain(Expr::mul([
        base.clone(),
        Expr::pow(base.clone(), (-1).into()),
    ]));
    let premises = Assumptions::from_conditions([Condition::NonZero(base.clone())]).unwrap();
    let DomainRewriteOutcome::Applied(rewrite) =
        rewrite_domain_once(&original, &premises).unwrap()
    else {
        panic!("expected composite cancellation");
    };
    assert_eq!(rewrite.result().conditions(), &[Condition::NonZero(base)]);
    assert_eq!(rewrite.rule_id(), Some("algebra.cancel_composite_inverse"));
    assert_eq!(
        verify_proof_graph(original.expression(), &premises, rewrite.proof()),
        Ok(())
    );
}

#[test]
fn rewriting_does_not_discard_preexisting_holes() {
    let x = sym("x");
    let y = sym("y");
    let original = DomainExpression::multiply(
        [
            domain(Expr::pow(x.clone(), (-1).into())),
            domain(Expr::mul([y.clone(), Expr::pow(y.clone(), (-1).into())])),
        ],
        32,
    )
    .unwrap();
    let assumptions = Assumptions::from_conditions([
        Condition::NonZero(x.clone()),
        Condition::NonZero(y.clone()),
    ])
    .unwrap();
    let DomainRewriteOutcome::Applied(rewrite) =
        rewrite_domain_once(&original, &assumptions).unwrap()
    else {
        panic!("expected cancellation");
    };
    assert_eq!(
        rewrite.result().conditions(),
        &[Condition::NonZero(x), Condition::NonZero(y)]
    );
}

#[test]
fn substitution_transforms_guards_even_when_value_is_zero() {
    let x = Symbol::new("x").unwrap();
    let y = sym("y");
    let original = DomainExpression::multiply(
        [
            domain(Expr::from(0)),
            domain(Expr::pow(Expr::from(x.clone()), (-1).into())),
        ],
        32,
    )
    .unwrap();
    assert_eq!(original.expression(), &Expr::from(0));
    let replaced = substitute_domain(&original, &x, &domain(Expr::from(0)), 128, 32).unwrap();
    assert_eq!(replaced.conditions(), &[Condition::NonZero(Expr::from(0))]);
    assert_eq!(
        Assumptions::new().assess(&replaced.conditions()[0]),
        ConditionStatus::Refuted
    );

    let replacement = domain(Expr::pow(y.clone(), (-1).into()));
    let replacement_result =
        substitute_domain(&original, &x, &replacement, 128, 32).unwrap();
    assert_eq!(
        replacement_result.conditions(),
        &[
            Condition::NonZero(Expr::pow(y.clone(), (-1).into())),
            Condition::NonZero(y)
        ]
    );
}

#[test]
fn unused_replacement_does_not_shrink_domain() {
    let x = Symbol::new("x").unwrap();
    let y = sym("y");
    let original = domain(Expr::from(2));
    let replacement = domain(Expr::pow(y, (-1).into()));
    let updated = substitute_domain(&original, &x, &replacement, 128, 32).unwrap();
    assert_eq!(updated.expression(), &Expr::from(2));
    assert!(updated.conditions().is_empty());
}
