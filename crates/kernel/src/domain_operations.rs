//! Domain-preserving substitution and independently checked cancellation.

use std::error::Error;
use std::fmt;

use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ExprView, Symbol,
};

use crate::{
    Assumptions, ConditionStatus, ConditionalRewriteOutcome, GuardedRewriteOutcome, ProofGraph,
    ProofGraphBuildError, ProofGraphVerificationError, ProofNodeId,
    cancel_composite_inverse_once, cancel_symbol_inverse_once, substitute_symbol,
    verify_proof_graph,
};

const MAX_NODES: usize = 2_048;
const MAX_CONDITIONS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainOperationError {
    Definedness(DefinednessError),
    ProofConstruction(ProofGraphBuildError),
    ProofVerification(ProofGraphVerificationError),
}

impl fmt::Display for DomainOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Definedness(error) => write!(f, "{error}"),
            Self::ProofConstruction(error) => write!(f, "{error}"),
            Self::ProofVerification(error) => write!(f, "{error}"),
        }
    }
}

impl Error for DomainOperationError {}

impl From<DefinednessError> for DomainOperationError {
    fn from(error: DefinednessError) -> Self {
        Self::Definedness(error)
    }
}

impl From<ProofGraphBuildError> for DomainOperationError {
    fn from(error: ProofGraphBuildError) -> Self {
        Self::ProofConstruction(error)
    }
}

impl From<ProofGraphVerificationError> for DomainOperationError {
    fn from(error: ProofGraphVerificationError) -> Self {
        Self::ProofVerification(error)
    }
}

/// A checked rewrite and its domain-preserving result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainRewrite {
    result: DomainExpression,
    proof: ProofGraph,
}

impl DomainRewrite {
    pub fn result(&self) -> &DomainExpression {
        &self.result
    }

    pub fn proof(&self) -> &ProofGraph {
        &self.proof
    }

    pub fn rule_id(&self) -> Option<&'static str> {
        self.proof
            .nodes()
            .get(self.proof.conclusion().index())?
            .kind()
            .rule_id()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainRewriteOutcome {
    Applied(DomainRewrite),
    Blocked(Vec<Condition>),
    Refuted(Condition),
    NotApplicable,
}

/// Cancels one inverse pair only when the input domain and rule conditions
/// are established by the caller's explicit premises or exact arithmetic.
///
/// The returned expression retains the input's complete set of conditions.
/// The corresponding graph is checked before it is returned.
pub fn rewrite_domain_once(
    source: &DomainExpression,
    assumptions: &Assumptions,
) -> Result<DomainRewriteOutcome, DomainOperationError> {
    let mut missing = Vec::new();
    for condition in source.conditions() {
        match assumptions.assess(condition) {
            ConditionStatus::Refuted => return Ok(DomainRewriteOutcome::Refuted(condition.clone())),
            ConditionStatus::Unknown => missing.push(condition.clone()),
            ConditionStatus::Proven | ConditionStatus::Assumed => {}
        }
    }
    if !missing.is_empty() {
        return Ok(DomainRewriteOutcome::Blocked(missing));
    }

    let mut graph = ProofGraph::new(source.expression().clone());
    let root = ProofNodeId::new(0);

    match cancel_composite_inverse_once(source.expression(), assumptions)? {
        ConditionalRewriteOutcome::Applied(step) => {
            let mut parents = Vec::new();
            for evidence in step.conditions() {
                parents.push(graph.add_condition(
                    evidence.condition().clone(),
                    evidence.status(),
                )?);
            }
            let after = step.after().clone();
            let end = graph.add_composite_cancellation(root, parents, step)?;
            graph.select_conclusion(end);
            verify_proof_graph(source.expression(), assumptions, &graph)?;
            let result = DomainExpression::with_conditions(
                after,
                source.conditions().iter().cloned(),
                MAX_NODES,
                MAX_CONDITIONS,
            )?;
            return Ok(DomainRewriteOutcome::Applied(DomainRewrite { result, proof: graph }));
        }
        ConditionalRewriteOutcome::Blocked(conditions) => {
            return Ok(DomainRewriteOutcome::Blocked(conditions));
        }
        ConditionalRewriteOutcome::Refuted(condition) => {
            return Ok(DomainRewriteOutcome::Refuted(condition));
        }
        ConditionalRewriteOutcome::NotApplicable => {}
    }

    match cancel_symbol_inverse_once(source.expression(), assumptions) {
        GuardedRewriteOutcome::Applied(step) => {
            let condition = step.condition().clone();
            let parent = graph.add_condition(
                condition.clone(),
                assumptions.assess(&condition),
            )?;
            let after = step.after().clone();
            let end = graph.add_guarded_cancellation(root, parent, step)?;
            graph.select_conclusion(end);
            verify_proof_graph(source.expression(), assumptions, &graph)?;
            let result = DomainExpression::with_conditions(
                after,
                source.conditions().iter().cloned(),
                MAX_NODES,
                MAX_CONDITIONS,
            )?;
            Ok(DomainRewriteOutcome::Applied(DomainRewrite { result, proof: graph }))
        }
        GuardedRewriteOutcome::Blocked(condition) => {
            Ok(DomainRewriteOutcome::Blocked(vec![condition]))
        }
        GuardedRewriteOutcome::NotApplicable => Ok(DomainRewriteOutcome::NotApplicable),
    }
}

/// Rebuilds the value and every inherited restriction after substitution.
/// Restrictions of the replacement are included only if the substituted
/// symbol actually occurs in the source value or its retained conditions.
pub fn substitute_domain(
    source: &DomainExpression,
    symbol: &Symbol,
    replacement: &DomainExpression,
    max_nodes: usize,
    max_conditions: usize,
) -> Result<DomainExpression, DefinednessError> {
    let mut encountered = occurrence(source.expression(), symbol, max_nodes)?;
    if !encountered {
        for condition in source.conditions() {
            if occurrence(condition.expression(), symbol, max_nodes)? {
                encountered = true;
                break;
            }
        }
    }

    let replaced = substitute_symbol(source.expression(), symbol, replacement.expression());
    let mut conditions = Vec::new();
    for condition in source.conditions() {
        conditions.push(substitute_condition(condition, symbol, replacement.expression()));
    }
    if encountered {
        conditions.extend(replacement.conditions().iter().cloned());
    }

    DomainExpression::with_conditions(replaced, conditions, max_nodes, max_conditions)
}

fn substitute_condition(condition: &Condition, symbol: &Symbol, replacement: &Expr) -> Condition {
    let value = substitute_symbol(condition.expression(), symbol, replacement);
    match condition {
        Condition::NonZero(_) => Condition::NonZero(value),
        Condition::Positive(_) => Condition::Positive(value),
        Condition::Real(_) => Condition::Real(value),
        Condition::Integer(_) => Condition::Integer(value),
        Condition::InDomain { domain, .. } => Condition::InDomain {
            expression: value,
            domain: *domain,
        },
    }
}

fn occurrence(expr: &Expr, symbol: &Symbol, max_nodes: usize) -> Result<bool, DefinednessError> {
    let mut pending = vec![expr];
    let mut visited = 0usize;
    while let Some(current) = pending.pop() {
        visited += 1;
        if visited > max_nodes {
            return Err(DefinednessError::NodeLimitExceeded);
        }
        match current.view() {
            ExprView::Symbol(found) if found == symbol => return Ok(true),
            ExprView::Add(children) | ExprView::Mul(children) => {
                pending.extend(children.iter());
            }
            ExprView::Pow { base, exponent } => {
                pending.push(base);
                pending.push(exponent);
            }
            _ => {}
        }
    }
    Ok(false)
}
