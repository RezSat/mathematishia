use mathema::{Condition, Expr, Symbol};
use mathematishia_kernel::{
    Assumptions, ConditionStatus, GuardedRewriteOutcome, ProofGraph, ProofGraphBuildError,
    ProofGraphVerificationError, ProofNodeId, ProofNodeKind, cancel_symbol_inverse_once,
    distribute_product_once, verify_proof_graph,
};

fn symbol(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

fn guarded_step(
    input: &Expr,
    assumptions: &Assumptions,
) -> mathematishia_kernel::GuardedRewriteStep {
    match cancel_symbol_inverse_once(input, assumptions) {
        GuardedRewriteOutcome::Applied(step) => step,
        other => panic!("expected a guarded rewrite: {other:?}"),
    }
}

#[test]
fn verifies_dependency_graph_with_a_condition_and_two_rules() {
    let x = symbol("x");
    let y = symbol("y");
    let z = symbol("z");
    let input = Expr::mul([
        x.clone(),
        Expr::pow(x.clone(), (-1).into()),
        z.clone(),
        Expr::add([y.clone(), 1.into()]),
    ]);
    let assumptions = Assumptions::from_conditions([Condition::Positive(x.clone())]).unwrap();
    let mut graph = ProofGraph::new(input.clone());
    let premise = graph
        .add_condition(Condition::NonZero(x.clone()), ConditionStatus::Assumed)
        .unwrap();
    let cancel = graph
        .add_guarded_cancellation(
            ProofNodeId::new(0),
            premise,
            guarded_step(&input, &assumptions),
        )
        .unwrap();
    let after_cancel = match graph.nodes()[cancel.index()].kind().expression() {
        Some(expression) => expression.clone(),
        None => panic!("expected expression node"),
    };
    let distribute = graph
        .add_distribution(cancel, distribute_product_once(&after_cancel).unwrap())
        .unwrap();
    graph.select_conclusion(distribute);
    assert_eq!(verify_proof_graph(&input, &assumptions, &graph), Ok(()));
    let expected = Expr::add([z.clone(), Expr::mul([z, y])]);
    assert_eq!(
        graph.nodes()[distribute.index()].kind().expression(),
        Some(&expected)
    );
}

#[test]
fn verifies_unconditional_distribution() {
    let x = symbol("x");
    let input = Expr::mul([x, Expr::add([symbol("y"), 1.into()])]);
    let step = distribute_product_once(&input).unwrap();
    let mut graph = ProofGraph::new(input.clone());
    let conclusion = graph.add_distribution(ProofNodeId::new(0), step).unwrap();
    graph.select_conclusion(conclusion);
    assert_eq!(
        verify_proof_graph(&input, &Assumptions::new(), &graph),
        Ok(())
    );
    assert_eq!(
        graph.nodes()[conclusion.index()].kind().rule_id(),
        Some("algebra.distribute_product_over_sum")
    );
}

#[test]
fn verifies_guarded_cancellation_with_explicit_condition_dependency() {
    let x = symbol("x");
    let y = symbol("y");
    let input = Expr::mul([x.clone(), Expr::pow(x.clone(), (-1).into()), y.clone()]);
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x.clone())]).unwrap();
    let mut graph = ProofGraph::new(input.clone());
    let premise = graph
        .add_condition(Condition::NonZero(x), ConditionStatus::Assumed)
        .unwrap();
    let finish = graph
        .add_guarded_cancellation(
            ProofNodeId::new(0),
            premise,
            guarded_step(&input, &assumptions),
        )
        .unwrap();
    graph.select_conclusion(finish);

    assert_eq!(verify_proof_graph(&input, &assumptions, &graph), Ok(()));
    assert_eq!(graph.nodes()[finish.index()].kind().expression(), Some(&y));
    assert_eq!(
        graph.nodes()[finish.index()].kind().dependencies(),
        vec![ProofNodeId::new(0), premise]
    );
    assert!(matches!(
        graph.nodes()[premise.index()].kind(),
        ProofNodeKind::Condition { .. }
    ));
    assert_eq!(
        verify_proof_graph(&input, &Assumptions::new(), &graph),
        Err(ProofGraphVerificationError::UnjustifiedCondition)
    );
}

#[test]
fn rejects_false_conditions_and_disconnected_edges() {
    let x = symbol("x");
    let original = Expr::mul([x.clone(), Expr::add([symbol("z"), 2.into()])]);
    let step = distribute_product_once(&original).unwrap();
    let assumptions = Assumptions::new();
    let mut graph = ProofGraph::new(original.clone());

    let unused = graph
        .add_condition(Condition::NonZero(x.clone()), ConditionStatus::Assumed)
        .unwrap();
    let conclusion = graph.add_distribution(ProofNodeId::new(0), step).unwrap();
    graph.select_conclusion(conclusion);
    assert_eq!(
        verify_proof_graph(&original, &assumptions, &graph),
        Err(ProofGraphVerificationError::UnjustifiedCondition)
    );

    let known = Assumptions::from_conditions([Condition::NonZero(x)]).unwrap();
    assert_eq!(
        verify_proof_graph(&original, &known, &graph),
        Err(ProofGraphVerificationError::DisconnectedNode)
    );
    graph.select_conclusion(unused);
    assert_eq!(
        verify_proof_graph(&original, &known, &graph),
        Err(ProofGraphVerificationError::InvalidConclusion)
    );
}

#[test]
fn rejects_forged_parents_forward_references_and_wrong_sources() {
    let x = symbol("x");
    let original = Expr::mul([x, Expr::add([symbol("y"), 1.into()])]);
    let step = distribute_product_once(&original).unwrap();
    let assumptions = Assumptions::new();

    let mut wrong_source = ProofGraph::new(original.clone());
    let conclusion = wrong_source
        .add_distribution(ProofNodeId::new(0), step.clone())
        .unwrap();
    wrong_source.select_conclusion(conclusion);
    assert_eq!(
        verify_proof_graph(&Expr::from(5), &assumptions, &wrong_source),
        Err(ProofGraphVerificationError::IncorrectInput)
    );

    let mut cycle = ProofGraph::new(original.clone());
    let cyclic = cycle
        .add_distribution(ProofNodeId::new(1), step.clone())
        .unwrap();
    cycle.select_conclusion(cyclic);
    assert_eq!(
        verify_proof_graph(&original, &assumptions, &cycle),
        Err(ProofGraphVerificationError::InvalidDependency)
    );

    let mut nonexistent = ProofGraph::new(original.clone());
    let missing = nonexistent
        .add_distribution(ProofNodeId::new(99), step)
        .unwrap();
    nonexistent.select_conclusion(missing);
    assert_eq!(
        verify_proof_graph(&original, &assumptions, &nonexistent),
        Err(ProofGraphVerificationError::InvalidDependency)
    );
}

#[test]
fn enforces_node_limit_and_deterministic_ids() {
    let source = symbol("x");
    assert_eq!(
        ProofGraph::with_node_limit(source.clone(), 0),
        Err(ProofGraphBuildError::InvalidNodeLimit)
    );
    let mut graph = ProofGraph::with_node_limit(source.clone(), 2).unwrap();
    assert_eq!(
        graph.add_condition(Condition::NonZero(source.clone()), ConditionStatus::Assumed),
        Ok(ProofNodeId::new(1))
    );
    assert_eq!(
        graph.add_condition(Condition::NonZero(source.clone()), ConditionStatus::Assumed),
        Err(ProofGraphBuildError::NodeLimitExceeded)
    );
    assert_eq!(graph.nodes().len(), 2);
    assert_eq!(
        verify_proof_graph(&source, &Assumptions::new(), &ProofGraph::new(source.clone())),
        Ok(())
    );
}
