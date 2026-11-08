//! Dependency-checked evidence graphs for expression equalities.
//!
//! The graph stores existing checked step records. Its verifier checks the
//! dependencies and delegates each mathematical claim to its rule-specific
//! independent checker.

use std::error::Error;
use std::fmt;

use mathema::{Condition, Expr};

use crate::{
    Assumptions, ConditionStatus, DerivationStep, GuardedRewriteStep, Relation,
    verify_distribution_step, verify_guarded_rewrite_step,
};

/// Stable index of a node within one proof graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProofNodeId(usize);

impl ProofNodeId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

/// A node has either an expression value or a condition value.
///
/// Rewrites reference an earlier expression node; guarded rewrites also
/// reference an earlier condition node. No free-form proof text is stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofNodeKind {
    Input(Expr),
    Condition {
        condition: Condition,
        status: ConditionStatus,
    },
    Distribution {
        source: ProofNodeId,
        step: DerivationStep,
    },
    GuardedCancellation {
        source: ProofNodeId,
        condition: ProofNodeId,
        step: GuardedRewriteStep,
    },
}

impl ProofNodeKind {
    pub fn expression(&self) -> Option<&Expr> {
        match self {
            Self::Input(expr) => Some(expr),
            Self::Distribution { step, .. } => Some(step.after()),
            Self::GuardedCancellation { step, .. } => Some(step.after()),
            Self::Condition { .. } => None,
        }
    }

    pub fn condition(&self) -> Option<&Condition> {
        match self {
            Self::Condition { condition, .. } => Some(condition),
            _ => None,
        }
    }

    pub fn relation(&self) -> Option<Relation> {
        match self {
            Self::Distribution { step, .. } => Some(step.relation()),
            Self::GuardedCancellation { step, .. } => Some(step.relation()),
            _ => None,
        }
    }

    pub fn rule_id(&self) -> Option<&'static str> {
        match self {
            Self::Distribution { step, .. } => Some(step.rule().stable_id()),
            Self::GuardedCancellation { step, .. } => Some(step.rule().stable_id()),
            _ => None,
        }
    }

    pub fn dependencies(&self) -> Vec<ProofNodeId> {
        match self {
            Self::Input(_) | Self::Condition { .. } => Vec::new(),
            Self::Distribution { source, .. } => vec![*source],
            Self::GuardedCancellation {
                source, condition, ..
            } => vec![*source, *condition],
        }
    }
}

/// A numbered proof node. IDs follow insertion order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofNode {
    id: ProofNodeId,
    kind: ProofNodeKind,
}

impl ProofNode {
    pub const fn id(&self) -> ProofNodeId {
        self.id
    }

    pub fn kind(&self) -> &ProofNodeKind {
        &self.kind
    }
}

/// Finite graph of expression equalities and their prerequisite conditions.
///
/// Node zero is the input expression. The selected conclusion is one of the
/// expression nodes. Adding a node does not certify it: use
/// `verify_proof_graph` before relying on the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofGraph {
    nodes: Vec<ProofNode>,
    conclusion: ProofNodeId,
    max_nodes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofGraphBuildError {
    InvalidNodeLimit,
    NodeLimitExceeded,
}

impl fmt::Display for ProofGraphBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNodeLimit => f.write_str("proof graph requires at least one node"),
            Self::NodeLimitExceeded => f.write_str("proof graph node limit exceeded"),
        }
    }
}

impl Error for ProofGraphBuildError {}

impl ProofGraph {
    pub fn new(input: Expr) -> Self {
        Self::with_node_limit(input, 128).expect("nonzero default proof node limit")
    }

    pub fn with_node_limit(input: Expr, max_nodes: usize) -> Result<Self, ProofGraphBuildError> {
        if max_nodes == 0 {
            return Err(ProofGraphBuildError::InvalidNodeLimit);
        }
        Ok(Self {
            nodes: vec![ProofNode {
                id: ProofNodeId::new(0),
                kind: ProofNodeKind::Input(input),
            }],
            conclusion: ProofNodeId::new(0),
            max_nodes,
        })
    }

    pub fn nodes(&self) -> &[ProofNode] {
        &self.nodes
    }

    pub const fn conclusion(&self) -> ProofNodeId {
        self.conclusion
    }

    pub fn select_conclusion(&mut self, node: ProofNodeId) {
        self.conclusion = node;
    }

    fn append(&mut self, kind: ProofNodeKind) -> Result<ProofNodeId, ProofGraphBuildError> {
        if self.nodes.len() >= self.max_nodes {
            return Err(ProofGraphBuildError::NodeLimitExceeded);
        }
        let id = ProofNodeId::new(self.nodes.len());
        self.nodes.push(ProofNode { id, kind });
        Ok(id)
    }

    pub fn add_condition(
        &mut self,
        condition: Condition,
        status: ConditionStatus,
    ) -> Result<ProofNodeId, ProofGraphBuildError> {
        self.append(ProofNodeKind::Condition { condition, status })
    }

    pub fn add_distribution(
        &mut self,
        source: ProofNodeId,
        step: DerivationStep,
    ) -> Result<ProofNodeId, ProofGraphBuildError> {
        self.append(ProofNodeKind::Distribution { source, step })
    }

    pub fn add_guarded_cancellation(
        &mut self,
        source: ProofNodeId,
        condition: ProofNodeId,
        step: GuardedRewriteStep,
    ) -> Result<ProofNodeId, ProofGraphBuildError> {
        self.append(ProofNodeKind::GuardedCancellation {
            source,
            condition,
            step,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofGraphVerificationError {
    IncorrectInput,
    InvalidNodeId,
    InvalidDependency,
    UnjustifiedCondition,
    InvalidStep,
    InvalidConclusion,
    DisconnectedNode,
}

impl fmt::Display for ProofGraphVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::IncorrectInput => "proof graph input does not match",
            Self::InvalidNodeId => "proof graph node IDs are not sequential",
            Self::InvalidDependency => "proof graph has a missing or incorrectly typed parent",
            Self::UnjustifiedCondition => "proof graph condition is not established",
            Self::InvalidStep => "proof graph contains an invalid mathematical step",
            Self::InvalidConclusion => "proof graph conclusion is not an expression node",
            Self::DisconnectedNode => {
                "proof graph contains a node outside the conclusion dependency graph"
            }
        };
        f.write_str(message)
    }
}

impl Error for ProofGraphVerificationError {}

/// Checks the entire graph against its original input and supplied premises.
///
/// Backward references are required, which precludes cycles. Every condition
/// node is checked against the external assumptions, and every rewrite is
/// checked by its independent rule verifier. All nodes must be ancestors of
/// the selected conclusion.
pub fn verify_proof_graph(
    original: &Expr,
    assumptions: &Assumptions,
    graph: &ProofGraph,
) -> Result<(), ProofGraphVerificationError> {
    use ProofGraphVerificationError as E;

    let Some(first) = graph.nodes.first() else {
        return Err(E::IncorrectInput);
    };
    if !matches!(first.kind(), ProofNodeKind::Input(input) if input == original) {
        return Err(E::IncorrectInput);
    }

    for (index, node) in graph.nodes.iter().enumerate() {
        if node.id.index() != index {
            return Err(E::InvalidNodeId);
        }
        match node.kind() {
            ProofNodeKind::Input(_) if index == 0 => {}
            ProofNodeKind::Input(_) => return Err(E::IncorrectInput),
            ProofNodeKind::Condition { condition, status } => {
                let actual = assumptions.assess(condition);
                if !matches!(actual, ConditionStatus::Proven | ConditionStatus::Assumed)
                    || actual != *status
                {
                    return Err(E::UnjustifiedCondition);
                }
            }
            ProofNodeKind::Distribution { source, step } => {
                let Some(parent) = prior_expression(graph, *source, index) else {
                    return Err(E::InvalidDependency);
                };
                if step.before() != parent || verify_distribution_step(step).is_err() {
                    return Err(E::InvalidStep);
                }
            }
            ProofNodeKind::GuardedCancellation {
                source,
                condition,
                step,
            } => {
                let Some(parent) = prior_expression(graph, *source, index) else {
                    return Err(E::InvalidDependency);
                };
                let Some(required) = prior_condition(graph, *condition, index) else {
                    return Err(E::InvalidDependency);
                };
                if step.before() != parent || step.condition() != required {
                    return Err(E::InvalidStep);
                }
                if verify_guarded_rewrite_step(step, assumptions).is_err() {
                    return Err(E::InvalidStep);
                }
            }
        }
    }

    let end = graph.conclusion.index();
    if graph
        .nodes
        .get(end)
        .and_then(|node| node.kind.expression())
        .is_none()
    {
        return Err(E::InvalidConclusion);
    }

    let mut reachable = vec![false; graph.nodes.len()];
    let mut pending = vec![graph.conclusion];
    while let Some(id) = pending.pop() {
        let index = id.index();
        if reachable[index] {
            continue;
        }
        reachable[index] = true;
        pending.extend(graph.nodes[index].kind.dependencies());
    }
    if reachable.iter().any(|visited| !visited) {
        return Err(E::DisconnectedNode);
    }
    Ok(())
}

fn prior_expression(graph: &ProofGraph, id: ProofNodeId, current: usize) -> Option<&Expr> {
    if id.index() >= current {
        return None;
    }
    graph.nodes.get(id.index())?.kind.expression()
}

fn prior_condition(graph: &ProofGraph, id: ProofNodeId, current: usize) -> Option<&Condition> {
    if id.index() >= current {
        return None;
    }
    graph.nodes.get(id.index())?.kind.condition()
}
