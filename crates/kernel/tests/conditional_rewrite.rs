use mathema::{Condition, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    Assumptions, ConditionStatus, ConditionalRewriteOutcome, DefinednessError,
    ProofGraph, ProofGraphVerificationError, ProofNodeId, cancel_composite_inverse_once,
    verify_composite_inverse_step, verify_proof_graph,
};

fn symbol(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

#[test]
fn composite_cancellation_requires_an_explicit_nonzero_fact() {
    let x = symbol("x");
    let base = Expr::add([x.clone(), 1.into()]);
    let expression = Expr::mul([base.clone(), Expr::pow(base.clone(), (-1).into())]);
    let empty = Assumptions::new();
    assert_eq!(
        cancel_composite_inverse_once(&expression, &empty),
        Ok(ConditionalRewriteOutcome::Blocked(vec![Condition::NonZero(
            base.clone()
        )]))
    );
    let assumptions = Assumptions::from_conditions([Condition::NonZero(base.clone())]).unwrap();
    let step = match cancel_composite_inverse_once(&expression, &assumptions).unwrap() {
        ConditionalRewriteOutcome::Applied(step) => step,
        other => panic!("unexpected result: {other:?}"),
    };
    assert_eq!(step.before(), &expression);
    assert_eq!(step.after(), &Expr::from(1));
    assert_eq!(step.rule().stable_id(), "algebra.cancel_composite_inverse");
    assert_eq!(step.conditions().len(), 1);
    assert_eq!(step.conditions()[0].status(), ConditionStatus::Assumed);
    assert_eq!(verify_composite_inverse_step(&step, &assumptions), Ok(()));
    assert!(verify_composite_inverse_step(&step, &empty).is_err());

    let mut graph = ProofGraph::new(expression.clone());
    let condition_id = graph
        .add_condition(Condition::NonZero(base), ConditionStatus::Assumed)
        .unwrap();
    let conclusion = graph
        .add_composite_cancellation(ProofNodeId::new(0), vec![condition_id], step)
        .unwrap();
    graph.select_conclusion(conclusion);
    assert_eq!(verify_proof_graph(&expression, &assumptions, &graph), Ok(()));
    assert_eq!(
        graph.nodes()[conclusion.index()].kind().rule_id(),
        Some("algebra.cancel_composite_inverse")
    );

    let mut missing = graph.clone();
    let invalid = missing
        .add_composite_cancellation(ProofNodeId::new(0), vec![], match
            cancel_composite_inverse_once(&expression, &assumptions).unwrap()
        {
            ConditionalRewriteOutcome::Applied(step) => step,
            other => panic!("unexpected result: {other:?}"),
        })
        .unwrap();
    missing.select_conclusion(invalid);
    assert_eq!(
        verify_proof_graph(&expression, &assumptions, &missing),
        Err(ProofGraphVerificationError::InvalidDependency)
    );
}

#[test]
fn nested_reciprocals_propagate_both_conditions() {
    let x = symbol("x");
    let inner = Expr::pow(x.clone(), (-1).into());
    let expr = Expr::mul([
        inner.clone(),
        Expr::pow(inner.clone(), (-1).into()),
    ]);
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x.clone())]).unwrap();

    let step = match cancel_composite_inverse_once(&expr, &assumptions).unwrap() {
        ConditionalRewriteOutcome::Applied(step) => step,
        other => panic!("unexpected result: {other:?}"),
    };
    assert_eq!(step.after(), &Expr::from(1));
    assert_eq!(step.conditions().len(), 2);
    assert_eq!(
        step.conditions()[0].condition(),
        &Condition::NonZero(x.clone())
    );
    assert_eq!(
        step.conditions()[1].condition(),
        &Condition::NonZero(inner.clone())
    );
    assert_eq!(verify_composite_inverse_step(&step, &assumptions), Ok(()));

    let mut graph = ProofGraph::new(expr.clone());
    let first = graph
        .add_condition(Condition::NonZero(x), ConditionStatus::Assumed)
        .unwrap();
    let second = graph
        .add_condition(Condition::NonZero(inner), ConditionStatus::Assumed)
        .unwrap();
    let end = graph
        .add_composite_cancellation(ProofNodeId::new(0), vec![first, second], step)
        .unwrap();
    graph.select_conclusion(end);
    assert_eq!(verify_proof_graph(&expr, &assumptions, &graph), Ok(()));
}

#[test]
fn rejected_domain_and_unsupported_exponents_are_distinct() {
    let undefined = Expr::pow(Expr::from(0), (-1).into());
    let base = Expr::add([undefined, Expr::from(1)]);
    let expr = Expr::mul([base.clone(), Expr::pow(base, (-1).into())]);
    assert_eq!(
        cancel_composite_inverse_once(&expr, &Assumptions::new()),
        Ok(ConditionalRewriteOutcome::Refuted(Condition::NonZero(
            Expr::from(0)
        )))
    );

    let x = symbol("x");
    let half = Rational::new(Integer::one(), Integer::from(2)).unwrap();
    let base = Expr::pow(x, Expr::from(half));
    let expression = Expr::mul([base.clone(), Expr::pow(base, (-1).into())]);
    assert_eq!(
        cancel_composite_inverse_once(&expression, &Assumptions::new()),
        Err(DefinednessError::UnsupportedExponent)
    );
}

#[test]
fn refuses_to_guess_a_sum_is_nonzero() {
    let x = symbol("x");
    let base = Expr::add([x.clone(), 1.into()]);
    let expression = Expr::mul([base.clone(), Expr::pow(base.clone(), (-1).into())]);
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x)]).unwrap();
    assert_eq!(
        cancel_composite_inverse_once(&expression, &assumptions),
        Ok(ConditionalRewriteOutcome::Blocked(vec![Condition::NonZero(
            base
        )]))
    );
}
