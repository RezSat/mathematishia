use mathema::{Condition, Equation, Expr, Symbol};
use mathematishia_kernel::{
    Assumptions, ConditionStatus, ConditionalRewriteOutcome, DerivationRenderError,
    EvaluationContext, GuardedRewriteOutcome, LinearSolution, LinearSolveError, ProofGraph,
    ProofGraphVerificationError, ProofNodeId, Relation, RenderedClaim, RenderedConclusion,
    RenderedDerivation, RenderedLine, cancel_composite_inverse_once, cancel_symbol_inverse_once,
    distribute_product_once, render_linear_solution, render_proof_graph,
    render_symbolic_derivative, solve_linear_equation,
};

fn variable(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

fn steps(rendered: &RenderedDerivation) -> Vec<&mathematishia_kernel::RenderedStep> {
    rendered
        .lines
        .iter()
        .filter_map(|line| {
            if let RenderedLine::Step(step) = line {
                Some(step)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn equality_graph_retains_parent_order_premises_and_rule_ids() {
    let x = variable("x");
    let y = variable("y");
    let z = variable("z");
    let original = Expr::mul([
        x.clone(),
        Expr::pow(x.clone(), (-1).into()),
        z,
        Expr::add([y, 1.into()]),
    ]);
    let assumptions = Assumptions::from_conditions([Condition::Positive(x.clone())]).unwrap();
    let mut graph = ProofGraph::new(original.clone());
    let premise = graph
        .add_condition(Condition::NonZero(x), ConditionStatus::Assumed)
        .unwrap();
    let cancel = match cancel_symbol_inverse_once(&original, &assumptions) {
        GuardedRewriteOutcome::Applied(step) => graph
            .add_guarded_cancellation(ProofNodeId::new(0), premise, step)
            .unwrap(),
        other => panic!("expected cancellation: {other:?}"),
    };
    let intermediate = graph.nodes()[cancel.index()]
        .kind()
        .expression()
        .unwrap()
        .clone();
    let distribution = graph
        .add_distribution(cancel, distribute_product_once(&intermediate).unwrap())
        .unwrap();
    graph.select_conclusion(distribution);
    let rendered = render_proof_graph(&original, &assumptions, &graph).unwrap();
    assert_eq!(rendered.lines.len(), 4);
    assert!(matches!(rendered.lines[0], RenderedLine::Input { .. }));
    assert!(matches!(rendered.lines[1], RenderedLine::Condition(_)));
    let recorded = steps(&rendered);
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].proof_node, Some(cancel));
    assert_eq!(recorded[0].source_node, Some(ProofNodeId::new(0)));
    assert_eq!(recorded[0].rule_id, "algebra.cancel_symbol_inverse");
    assert_eq!(recorded[0].claim.relation(), Relation::Equal);
    assert_eq!(recorded[0].conditions.len(), 1);
    assert_eq!(recorded[0].conditions[0].proof_node, Some(premise));
    assert_eq!(recorded[0].conditions[0].status, ConditionStatus::Assumed);
    assert_eq!(recorded[1].source_node, Some(cancel));
    assert_eq!(recorded[1].rule_id, "algebra.distribute_product_over_sum");
    assert!(recorded[1].conditions.is_empty());
    assert_eq!(
        render_proof_graph(&original, &assumptions, &graph).unwrap(),
        rendered
    );
}

#[test]
fn composite_cancellation_preserves_multiple_condition_dependencies() {
    let x = variable("x");
    let base = Expr::add([Expr::pow(x.clone(), (-1).into()), 1.into()]);
    let original = Expr::mul([base.clone(), Expr::pow(base.clone(), (-1).into())]);
    let assumptions =
        Assumptions::from_conditions([Condition::NonZero(x), Condition::NonZero(base)]).unwrap();
    let evidence = match cancel_composite_inverse_once(&original, &assumptions).unwrap() {
        ConditionalRewriteOutcome::Applied(step) => step,
        other => panic!("expected composite cancellation: {other:?}"),
    };
    assert_eq!(evidence.conditions().len(), 2);
    let mut graph = ProofGraph::new(original.clone());
    let conditions: Vec<_> = evidence
        .conditions()
        .iter()
        .map(|entry| {
            graph
                .add_condition(entry.condition().clone(), entry.status())
                .unwrap()
        })
        .collect();
    let end = graph
        .add_composite_cancellation(ProofNodeId::new(0), conditions.clone(), evidence)
        .unwrap();
    graph.select_conclusion(end);
    let rendered = render_proof_graph(&original, &assumptions, &graph).unwrap();
    let recorded = steps(&rendered);
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].rule_id, "algebra.cancel_composite_inverse");
    assert!(matches!(recorded[0].claim, RenderedClaim::Equal { .. }));
    assert_eq!(
        recorded[0]
            .conditions
            .iter()
            .map(|item| item.proof_node.unwrap())
            .collect::<Vec<_>>(),
        conditions
    );
    assert_eq!(
        render_proof_graph(&original, &assumptions, &graph).unwrap(),
        rendered
    );
}

#[test]
fn forged_missing_and_disconnected_graphs_are_rejected() {
    let x = variable("x");
    let original = Expr::mul([x.clone(), Expr::add([variable("y"), 1.into()])]);
    let distribution = distribute_product_once(&original).unwrap();
    let mut graph = ProofGraph::new(original.clone());
    let end = graph
        .add_distribution(ProofNodeId::new(0), distribution.clone())
        .unwrap();
    graph.select_conclusion(end);
    assert_eq!(
        render_proof_graph(&Expr::from(7), &Assumptions::new(), &graph),
        Err(DerivationRenderError::Proof(
            ProofGraphVerificationError::IncorrectInput
        ))
    );
    graph
        .add_condition(Condition::NonZero(x.clone()), ConditionStatus::Assumed)
        .unwrap();
    let premises = Assumptions::from_conditions([Condition::NonZero(x)]).unwrap();
    assert_eq!(
        render_proof_graph(&original, &premises, &graph),
        Err(DerivationRenderError::Proof(
            ProofGraphVerificationError::DisconnectedNode
        ))
    );
    let mut forward = ProofGraph::new(original.clone());
    let invalid = forward
        .add_distribution(ProofNodeId::new(1), distribution)
        .unwrap();
    forward.select_conclusion(invalid);
    assert_eq!(
        render_proof_graph(&original, &Assumptions::new(), &forward),
        Err(DerivationRenderError::Proof(
            ProofGraphVerificationError::InvalidDependency
        ))
    );
    let mut unknown = ProofGraph::new(original.clone());
    unknown
        .add_condition(
            Condition::NonZero(variable("unknown")),
            ConditionStatus::Assumed,
        )
        .unwrap();
    let tail = unknown
        .add_distribution(
            ProofNodeId::new(0),
            distribute_product_once(&original).unwrap(),
        )
        .unwrap();
    unknown.select_conclusion(tail);
    assert_eq!(
        render_proof_graph(&original, &Assumptions::new(), &unknown),
        Err(DerivationRenderError::Proof(
            ProofGraphVerificationError::UnjustifiedCondition
        ))
    );
}

#[test]
fn equation_equivalence_preserves_exact_division_condition() {
    let var = Symbol::new("x").unwrap();
    let source = Equation::new(
        Expr::add([Expr::mul([2.into(), variable("x")]), 4.into()]),
        10.into(),
    );
    let solved = solve_linear_equation(&source, &var).unwrap();
    let rendered = render_linear_solution(&solved, &var).unwrap();
    let recorded = steps(&rendered);
    assert!(!recorded.is_empty());
    assert!(
        recorded
            .iter()
            .all(|item| item.claim.relation() == Relation::Equivalent)
    );
    assert!(recorded.iter().all(|item| item.display.contains("<=>")));
    let final_step = recorded.last().unwrap();
    assert_eq!(final_step.rule_id, "equation.divide_by_nonzero_coefficient");
    assert_eq!(final_step.conditions.len(), 1);
    assert_eq!(
        final_step.conditions[0].condition,
        Condition::NonZero(2.into())
    );
    assert_eq!(final_step.conditions[0].status, ConditionStatus::Proven);
    assert!(matches!(
        rendered.conclusion,
        RenderedConclusion::LinearEquation {
            solution: LinearSolution::Unique(_),
            ..
        }
    ));
    assert_eq!(render_linear_solution(&solved, &var).unwrap(), rendered);
}

#[test]
fn degenerate_equations_do_not_invent_division_steps() {
    let var = Symbol::new("x").unwrap();
    for (a, b, expected) in [
        (1, 2, LinearSolution::NoSolution),
        (1, 1, LinearSolution::AllRationals),
    ] {
        let equation = Equation::new(a.into(), b.into());
        let solved = solve_linear_equation(&equation, &var).unwrap();
        let rendered = render_linear_solution(&solved, &var).unwrap();
        assert!(
            matches!(rendered.conclusion, RenderedConclusion::LinearEquation {
            ref solution, ..
        } if solution == &expected)
        );
        assert!(
            steps(&rendered)
                .iter()
                .all(|item| item.conditions.is_empty())
        );
        assert!(
            !steps(&rendered)
                .iter()
                .any(|item| item.rule_id == "equation.divide_by_nonzero_coefficient")
        );
    }
    let nonlinear = Equation::new(Expr::pow(variable("x"), 2.into()), 0.into());
    assert_eq!(
        solve_linear_equation(&nonlinear, &var),
        Err(LinearSolveError::NonlinearEquation)
    );
}

#[test]
fn derivatives_are_not_misrepresented_as_equalities() {
    let var = Symbol::new("x").unwrap();
    let source = Expr::add([Expr::pow(variable("x"), 3.into()), variable("x")]);
    let derivative = EvaluationContext::default()
        .differentiate_expression(&source, &var)
        .unwrap();
    let rendered = render_symbolic_derivative(&derivative).unwrap();
    let recorded = steps(&rendered);
    assert!(!recorded.is_empty());
    assert_eq!(recorded.len(), derivative.steps().len());
    assert!(
        recorded
            .iter()
            .all(|item| matches!(item.claim, RenderedClaim::DerivativeOf { .. }))
    );
    assert!(
        recorded
            .iter()
            .all(|item| item.claim.relation() == Relation::DerivativeOf)
    );
    assert!(
        recorded
            .iter()
            .all(|item| item.display.starts_with("d/d(x)"))
    );
    assert!(recorded.iter().all(|item| item.conditions.is_empty()));
    assert_eq!(
        rendered.conclusion,
        RenderedConclusion::Derivative {
            variable: var,
            derivative: derivative.derivative().clone(),
        }
    );
    assert_eq!(render_symbolic_derivative(&derivative).unwrap(), rendered);
}

#[test]
fn rejects_oversized_graphs_and_symbol_display() {
    let huge = variable(&"s".repeat(16_385));
    assert_eq!(
        render_proof_graph(&huge, &Assumptions::new(), &ProofGraph::new(huge.clone())),
        Err(DerivationRenderError::ResourceLimitExceeded)
    );
    let source = variable("x");
    let mut graph = ProofGraph::with_node_limit(source.clone(), 300).unwrap();
    for _ in 0..256 {
        graph
            .add_condition(Condition::NonZero(source.clone()), ConditionStatus::Unknown)
            .unwrap();
    }
    assert_eq!(
        render_proof_graph(&source, &Assumptions::new(), &graph),
        Err(DerivationRenderError::ResourceLimitExceeded)
    );
}

#[test]
fn nested_analytic_functions_are_counted_by_the_render_budget() {
    let mut expr = variable("x");
    for _ in 0..130 {
        expr = Expr::sin(expr);
    }
    let graph = ProofGraph::new(expr.clone());
    assert_eq!(
        render_proof_graph(&expr, &Assumptions::new(), &graph),
        Err(DerivationRenderError::ResourceLimitExceeded)
    );
}
