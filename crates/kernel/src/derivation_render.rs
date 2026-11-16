//! Checked, transport-independent rendering of existing mathematical evidence.
//!
//! Display strings are explanatory only. Mathematical authority comes from
//! the existing proof, equation, and derivative verifiers.

use std::error::Error;
use std::fmt;

use mathema::{Condition, Equation, Expr, ExprView, ScalarDomain, Symbol};

use crate::{
    Assumptions, ConditionStatus, DerivativeVerificationError, LinearSolution, LinearSolveResult,
    ProofGraph, ProofGraphVerificationError, ProofNodeId, ProofNodeKind, Relation,
    SymbolicDerivative, VerificationError, VerifiedCondition, verify_linear_solution,
    verify_proof_graph, verify_symbolic_derivative,
};

const MAX_LINES: usize = 256;
const MAX_EXPRESSION_NODES: usize = 32_768;
const MAX_DEPTH: usize = 128;
const MAX_CONDITIONS_PER_STEP: usize = 64;
const MAX_TEXT_BYTES: usize = 16_384;

/// A mathematical statement, without attaching a relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderedStatement {
    Expression(Expr),
    Equation(Equation),
}

/// Distinct verified claims: differentiation is not an equality rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderedClaim {
    Equal {
        before: Expr,
        after: Expr,
    },
    Equivalent {
        before: Equation,
        after: Equation,
    },
    DerivativeOf {
        expression: Expr,
        derivative: Expr,
        variable: Symbol,
    },
}

impl RenderedClaim {
    pub const fn relation(&self) -> Relation {
        match self {
            Self::Equal { .. } => Relation::Equal,
            Self::Equivalent { .. } => Relation::Equivalent,
            Self::DerivativeOf { .. } => Relation::DerivativeOf,
        }
    }
}

/// Established obligation and its provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedCondition {
    pub condition: Condition,
    pub status: ConditionStatus,
    /// The proof node recording this obligation, if applicable.
    pub proof_node: Option<ProofNodeId>,
    pub display: String,
}

/// A verified rule application. The display string is not a certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedStep {
    pub proof_node: Option<ProofNodeId>,
    pub source_node: Option<ProofNodeId>,
    pub claim: RenderedClaim,
    pub rule_id: &'static str,
    pub conditions: Vec<RenderedCondition>,
    pub explanation: &'static str,
    pub display: String,
}

/// A proof input, premise or checked mathematical step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderedLine {
    Input {
        proof_node: Option<ProofNodeId>,
        statement: RenderedStatement,
        display: String,
    },
    Condition(RenderedCondition),
    Step(RenderedStep),
}

/// Concludes a verified derivation without discarding equation classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderedConclusion {
    Expression(Expr),
    LinearEquation {
        equation: Equation,
        solution: LinearSolution,
    },
    Derivative {
        variable: Symbol,
        derivative: Expr,
    },
}

/// Lines retain evidence order, not an invented linear rewrite chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedDerivation {
    pub lines: Vec<RenderedLine>,
    pub conclusion: RenderedConclusion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivationRenderError {
    Proof(ProofGraphVerificationError),
    Equation(VerificationError),
    Derivative(DerivativeVerificationError),
    /// No presentation contract exists for the requested evidence.
    UnsupportedEvidence,
    ResourceLimitExceeded,
}

impl fmt::Display for DerivationRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proof(err) => write!(f, "invalid proof graph: {err}"),
            Self::Equation(err) => write!(f, "invalid equation derivation: {err}"),
            Self::Derivative(err) => write!(f, "invalid derivative derivation: {err}"),
            Self::UnsupportedEvidence => f.write_str("unsupported derivation evidence"),
            Self::ResourceLimitExceeded => {
                f.write_str("derivation rendering resource limit exceeded")
            }
        }
    }
}

impl Error for DerivationRenderError {}

fn bounded_text(text: String) -> Result<String, DerivationRenderError> {
    if text.len() > MAX_TEXT_BYTES {
        Err(DerivationRenderError::ResourceLimitExceeded)
    } else {
        Ok(text)
    }
}

/// Iterative preflight prevents unbounded recursive formatting or verification.
fn inspect_expr(expr: &Expr, remaining: &mut usize) -> Result<(), DerivationRenderError> {
    let mut pending = vec![(expr, 0_usize)];
    while let Some((current, depth)) = pending.pop() {
        if depth > MAX_DEPTH || *remaining == 0 {
            return Err(DerivationRenderError::ResourceLimitExceeded);
        }
        *remaining -= 1;
        match current.view() {
            ExprView::Add(items) | ExprView::Mul(items) => {
                pending.extend(items.iter().map(|item| (item, depth + 1)));
            }
            ExprView::Pow { base, exponent } => {
                pending.push((base, depth + 1));
                pending.push((exponent, depth + 1));
            }
            ExprView::Symbol(symbol) if symbol.name().len() > MAX_TEXT_BYTES => {
                return Err(DerivationRenderError::ResourceLimitExceeded);
            }
            _ => {}
        }
    }
    Ok(())
}

fn inspect_equation(
    equation: &Equation,
    remaining: &mut usize,
) -> Result<(), DerivationRenderError> {
    inspect_expr(equation.left(), remaining)?;
    inspect_expr(equation.right(), remaining)
}

fn condition_text(condition: &Condition) -> String {
    match condition {
        Condition::NonZero(expr) => format!("{expr} != 0"),
        Condition::Positive(expr) => format!("{expr} > 0"),
        Condition::Real(expr) => format!("{expr} in R"),
        Condition::Integer(expr) => format!("{expr} in Z"),
        Condition::InDomain { expression, domain } => {
            let domain = match domain {
                ScalarDomain::Integer => "Z",
                ScalarDomain::Rational => "Q",
                ScalarDomain::Real => "R",
                ScalarDomain::Complex => "C",
            };
            format!("{expression} in {domain}")
        }
    }
}

fn render_condition(
    condition: Condition,
    status: ConditionStatus,
    proof_node: Option<ProofNodeId>,
) -> Result<RenderedCondition, DerivationRenderError> {
    let suffix = match status {
        ConditionStatus::Proven => "proven",
        ConditionStatus::Assumed => "assumed",
        _ => return Err(DerivationRenderError::UnsupportedEvidence),
    };
    let display = bounded_text(format!("{} ({suffix})", condition_text(&condition)))?;
    Ok(RenderedCondition {
        condition,
        status,
        proof_node,
        display,
    })
}

fn equation_text(equation: &Equation) -> String {
    format!("{} = {}", equation.left(), equation.right())
}

fn statement_text(statement: &RenderedStatement) -> String {
    match statement {
        RenderedStatement::Expression(expr) => expr.to_string(),
        RenderedStatement::Equation(equation) => equation_text(equation),
    }
}

fn claim_text(claim: &RenderedClaim) -> String {
    match claim {
        RenderedClaim::Equal { before, after } => format!("({before}) = ({after})"),
        RenderedClaim::Equivalent { before, after } => {
            format!("({}) <=> ({})", equation_text(before), equation_text(after))
        }
        RenderedClaim::DerivativeOf {
            expression,
            derivative,
            variable,
        } => format!("d/d({variable}) ({expression}) = {derivative}"),
    }
}

fn render_step(
    proof_node: Option<ProofNodeId>,
    source_node: Option<ProofNodeId>,
    claim: RenderedClaim,
    rule_id: &'static str,
    explanation: &'static str,
    conditions: Vec<RenderedCondition>,
) -> Result<RenderedStep, DerivationRenderError> {
    let display = bounded_text(format!("{} [{}]", claim_text(&claim), explanation))?;
    Ok(RenderedStep {
        proof_node,
        source_node,
        claim,
        rule_id,
        conditions,
        explanation,
        display,
    })
}

/// Checks the entire graph before rendering any of its nodes.
pub fn render_proof_graph(
    original: &Expr,
    assumptions: &Assumptions,
    graph: &ProofGraph,
) -> Result<RenderedDerivation, DerivationRenderError> {
    if graph.nodes().len() > MAX_LINES {
        return Err(DerivationRenderError::ResourceLimitExceeded);
    }
    let mut remaining = MAX_EXPRESSION_NODES;
    inspect_expr(original, &mut remaining)?;
    for node in graph.nodes() {
        match node.kind() {
            ProofNodeKind::Input(expr) => inspect_expr(expr, &mut remaining)?,
            ProofNodeKind::Condition { condition, .. } => {
                inspect_expr(condition.expression(), &mut remaining)?;
            }
            ProofNodeKind::Distribution { step, .. } => {
                inspect_expr(step.before(), &mut remaining)?;
                inspect_expr(step.after(), &mut remaining)?;
            }
            ProofNodeKind::GuardedCancellation { step, .. } => {
                inspect_expr(step.before(), &mut remaining)?;
                inspect_expr(step.after(), &mut remaining)?;
                inspect_expr(step.condition().expression(), &mut remaining)?;
            }
            ProofNodeKind::CompositeCancellation {
                conditions, step, ..
            } => {
                if conditions.len() > MAX_CONDITIONS_PER_STEP {
                    return Err(DerivationRenderError::ResourceLimitExceeded);
                }
                inspect_expr(step.before(), &mut remaining)?;
                inspect_expr(step.after(), &mut remaining)?;
                for evidence in step.conditions() {
                    inspect_expr(evidence.condition().expression(), &mut remaining)?;
                }
            }
        }
    }
    verify_proof_graph(original, assumptions, graph).map_err(DerivationRenderError::Proof)?;

    let mut lines = Vec::with_capacity(graph.nodes().len());
    for node in graph.nodes() {
        let id = node.id();
        let line = match node.kind() {
            ProofNodeKind::Input(expr) => {
                let statement = RenderedStatement::Expression(expr.clone());
                RenderedLine::Input {
                    proof_node: Some(id),
                    display: bounded_text(statement_text(&statement))?,
                    statement,
                }
            }
            ProofNodeKind::Condition { condition, status } => {
                RenderedLine::Condition(render_condition(condition.clone(), *status, Some(id))?)
            }
            ProofNodeKind::Distribution { source, step } => {
                if step.relation() != Relation::Equal {
                    return Err(DerivationRenderError::UnsupportedEvidence);
                }
                RenderedLine::Step(render_step(
                    Some(id),
                    Some(*source),
                    RenderedClaim::Equal {
                        before: step.before().clone(),
                        after: step.after().clone(),
                    },
                    step.rule().stable_id(),
                    "distributive law",
                    Vec::new(),
                )?)
            }
            ProofNodeKind::GuardedCancellation {
                source,
                condition,
                step,
            } => {
                if step.relation() != Relation::Equal {
                    return Err(DerivationRenderError::UnsupportedEvidence);
                }
                let dependency = render_condition(
                    step.condition().clone(),
                    step.justification(),
                    Some(*condition),
                )?;
                RenderedLine::Step(render_step(
                    Some(id),
                    Some(*source),
                    RenderedClaim::Equal {
                        before: step.before().clone(),
                        after: step.after().clone(),
                    },
                    step.rule().stable_id(),
                    "cancel inverse factors",
                    vec![dependency],
                )?)
            }
            ProofNodeKind::CompositeCancellation {
                source,
                conditions,
                step,
            } => {
                if step.relation() != Relation::Equal {
                    return Err(DerivationRenderError::UnsupportedEvidence);
                }
                let dependencies = conditions
                    .iter()
                    .zip(step.conditions())
                    .map(|(id, item)| {
                        render_condition(item.condition().clone(), item.status(), Some(*id))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                RenderedLine::Step(render_step(
                    Some(id),
                    Some(*source),
                    RenderedClaim::Equal {
                        before: step.before().clone(),
                        after: step.after().clone(),
                    },
                    step.rule().stable_id(),
                    "cancel composite inverse factors",
                    dependencies,
                )?)
            }
        };
        lines.push(line);
    }
    let conclusion = graph.nodes()[graph.conclusion().index()]
        .kind()
        .expression()
        .expect("verified expression conclusion")
        .clone();
    Ok(RenderedDerivation {
        lines,
        conclusion: RenderedConclusion::Expression(conclusion),
    })
}

/// Renders verified rational-linear evidence, preserving solution classification.
pub fn render_linear_solution(
    result: &LinearSolveResult,
    variable: &Symbol,
) -> Result<RenderedDerivation, DerivationRenderError> {
    if result.steps().len() + 1 > MAX_LINES {
        return Err(DerivationRenderError::ResourceLimitExceeded);
    }
    let mut remaining = MAX_EXPRESSION_NODES;
    inspect_equation(result.original(), &mut remaining)?;
    for step in result.steps() {
        inspect_equation(step.before(), &mut remaining)?;
        inspect_equation(step.after(), &mut remaining)?;
    }
    verify_linear_solution(result, variable).map_err(DerivationRenderError::Equation)?;
    let original = RenderedStatement::Equation(result.original().clone());
    let mut lines = vec![RenderedLine::Input {
        proof_node: None,
        display: bounded_text(statement_text(&original))?,
        statement: original,
    }];
    for step in result.steps() {
        if step.relation() != Relation::Equivalent {
            return Err(DerivationRenderError::UnsupportedEvidence);
        }
        let explanation = match step.rule() {
            crate::EquationRuleId::CollectToZero => "collect terms on one side",
            crate::EquationRuleId::IsolateLinearTerm => "isolate linear term",
            crate::EquationRuleId::DivideByNonzeroCoefficient => "divide by a nonzero coefficient",
        };
        let conditions = match step.verified_condition() {
            None => Vec::new(),
            Some(VerifiedCondition::Nonzero(value)) => vec![render_condition(
                Condition::NonZero(Expr::from(value.clone())),
                ConditionStatus::Proven,
                None,
            )?],
        };
        lines.push(RenderedLine::Step(render_step(
            None,
            None,
            RenderedClaim::Equivalent {
                before: step.before().clone(),
                after: step.after().clone(),
            },
            step.rule().stable_id(),
            explanation,
            conditions,
        )?));
    }
    let conclusion = result
        .steps()
        .last()
        .map_or_else(|| result.original(), |step| step.after());
    Ok(RenderedDerivation {
        lines,
        conclusion: RenderedConclusion::LinearEquation {
            equation: conclusion.clone(),
            solution: result.solution().clone(),
        },
    })
}

/// Renders verified postorder differentiation, not successive equality steps.
pub fn render_symbolic_derivative(
    result: &SymbolicDerivative,
) -> Result<RenderedDerivation, DerivationRenderError> {
    if result.steps().len() + 1 > MAX_LINES {
        return Err(DerivationRenderError::ResourceLimitExceeded);
    }
    let mut remaining = MAX_EXPRESSION_NODES;
    inspect_expr(result.original(), &mut remaining)?;
    inspect_expr(result.derivative(), &mut remaining)?;
    for step in result.steps() {
        inspect_expr(step.before(), &mut remaining)?;
        inspect_expr(step.derivative(), &mut remaining)?;
    }
    verify_symbolic_derivative(result).map_err(DerivationRenderError::Derivative)?;
    let original = RenderedStatement::Expression(result.original().clone());
    let mut lines = vec![RenderedLine::Input {
        proof_node: None,
        display: bounded_text(statement_text(&original))?,
        statement: original,
    }];
    for step in result.steps() {
        if step.relation() != Relation::DerivativeOf {
            return Err(DerivationRenderError::UnsupportedEvidence);
        }
        let explanation = match step.rule() {
            crate::DerivativeRuleId::Constant => "constant rule",
            crate::DerivativeRuleId::SelectedSymbol => "selected variable rule",
            crate::DerivativeRuleId::OtherSymbol => "constant symbol rule",
            crate::DerivativeRuleId::Sum => "sum rule",
            crate::DerivativeRuleId::Product => "product rule",
            crate::DerivativeRuleId::PositiveIntegerPower => "positive integer power rule",
        };
        lines.push(RenderedLine::Step(render_step(
            None,
            None,
            RenderedClaim::DerivativeOf {
                expression: step.before().clone(),
                derivative: step.derivative().clone(),
                variable: result.variable().clone(),
            },
            step.rule().stable_id(),
            explanation,
            Vec::new(),
        )?));
    }
    Ok(RenderedDerivation {
        lines,
        conclusion: RenderedConclusion::Derivative {
            variable: result.variable().clone(),
            derivative: result.derivative().clone(),
        },
    })
}
