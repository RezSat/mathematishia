//! Conditional cancellation of a composite scalar and its reciprocal.

use std::error::Error;
use std::fmt;

use mathema::{Condition, Expr, ExprView};

use crate::{Assumptions, ConditionStatus, DefinednessError, Relation, definedness_obligations};

const MAX_INSPECTED_NODES: usize = 2_048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionalRuleId {
    CancelCompositeInverse,
}

impl ConditionalRuleId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::CancelCompositeInverse => "algebra.cancel_composite_inverse",
        }
    }
}

/// The condition and its justification at the time a rule is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionEvidence {
    condition: Condition,
    status: ConditionStatus,
}

impl ConditionEvidence {
    pub fn condition(&self) -> &Condition {
        &self.condition
    }

    pub const fn status(&self) -> ConditionStatus {
        self.status
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionalRewriteStep {
    before: Expr,
    after: Expr,
    rule: ConditionalRuleId,
    relation: Relation,
    evidence: Vec<ConditionEvidence>,
}

impl ConditionalRewriteStep {
    pub fn before(&self) -> &Expr {
        &self.before
    }

    pub fn after(&self) -> &Expr {
        &self.after
    }

    pub const fn rule(&self) -> ConditionalRuleId {
        self.rule
    }

    pub const fn relation(&self) -> Relation {
        self.relation
    }

    pub fn conditions(&self) -> &[ConditionEvidence] {
        &self.evidence
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConditionalRewriteOutcome {
    Applied(ConditionalRewriteStep),
    Blocked(Vec<Condition>),
    Refuted(Condition),
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionalRewriteVerificationError {
    InvalidStep,
    InvalidConditions,
    UnsupportedDomain,
}

impl fmt::Display for ConditionalRewriteVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStep => f.write_str("invalid composite inverse cancellation"),
            Self::InvalidConditions => f.write_str("incorrect or missing cancellation conditions"),
            Self::UnsupportedDomain => f.write_str("unsupported definedness domain"),
        }
    }
}

impl Error for ConditionalRewriteVerificationError {}

fn candidate(expression: &Expr) -> Option<(&[Expr], usize, usize, &Expr)> {
    let ExprView::Mul(factors) = expression.view() else {
        return None;
    };
    for (left, base) in factors.iter().enumerate() {
        if !matches!(
            base.view(),
            ExprView::Add(_) | ExprView::Mul(_) | ExprView::Pow { .. }
        ) {
            continue;
        }
        for (right, reciprocal) in factors.iter().enumerate() {
            if left == right {
                continue;
            }
            if matches!(reciprocal.view(), ExprView::Pow { base: inner, exponent }
                if inner == base && exponent == &Expr::from(-1))
            {
                return Some((factors, left, right, base));
            }
        }
    }
    None
}

fn required_conditions(base: &Expr) -> Result<Vec<Condition>, DefinednessError> {
    let mut conditions = definedness_obligations(base, MAX_INSPECTED_NODES)?;
    let nonzero = Condition::NonZero(base.clone());
    if !conditions.contains(&nonzero) {
        conditions.push(nonzero);
    }
    Ok(conditions)
}

/// Removes one `f * f^-1` pair where f is a composite expression.
///
/// A symbolic f may contain its own reciprocals. The rule therefore carries
/// both the definedness obligations for f and the condition f != 0.
/// Unsatisfied conditions are returned instead of being dropped.
pub fn cancel_composite_inverse_once(
    expression: &Expr,
    assumptions: &Assumptions,
) -> Result<ConditionalRewriteOutcome, DefinednessError> {
    let Some((factors, left, right, base)) = candidate(expression) else {
        return Ok(ConditionalRewriteOutcome::NotApplicable);
    };

    let conditions = required_conditions(base)?;
    let mut missing = Vec::new();
    let mut evidence = Vec::with_capacity(conditions.len());
    for condition in conditions {
        let status = assumptions.assess(&condition);
        match status {
            ConditionStatus::Refuted => {
                return Ok(ConditionalRewriteOutcome::Refuted(condition));
            }
            ConditionStatus::Unknown => missing.push(condition),
            ConditionStatus::Proven | ConditionStatus::Assumed => {
                evidence.push(ConditionEvidence { condition, status });
            }
        }
    }
    if !missing.is_empty() {
        return Ok(ConditionalRewriteOutcome::Blocked(missing));
    }

    let after = Expr::mul(
        factors
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != left && *index != right)
            .map(|(_, factor)| factor.clone()),
    );
    Ok(ConditionalRewriteOutcome::Applied(ConditionalRewriteStep {
        before: expression.clone(),
        after,
        rule: ConditionalRuleId::CancelCompositeInverse,
        relation: Relation::Equal,
        evidence,
    }))
}

/// Independently reconstructs the permitted factor removal and each
/// required condition. The generator is not called by the checker.
pub fn verify_composite_inverse_step(
    step: &ConditionalRewriteStep,
    assumptions: &Assumptions,
) -> Result<(), ConditionalRewriteVerificationError> {
    use ConditionalRewriteVerificationError as E;

    if step.rule != ConditionalRuleId::CancelCompositeInverse || step.relation != Relation::Equal {
        return Err(E::InvalidStep);
    }
    let ExprView::Mul(factors) = step.before.view() else {
        return Err(E::InvalidStep);
    };

    for (left, base) in factors.iter().enumerate() {
        if !matches!(
            base.view(),
            ExprView::Add(_) | ExprView::Mul(_) | ExprView::Pow { .. }
        ) {
            continue;
        }
        for (right, reciprocal) in factors.iter().enumerate() {
            if left == right {
                continue;
            }
            if !matches!(reciprocal.view(), ExprView::Pow { base: inner, exponent }
                if inner == base && exponent == &Expr::from(-1))
            {
                continue;
            }

            let expected = Expr::mul(
                factors
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != left && *index != right)
                    .map(|(_, factor)| factor.clone()),
            );
            if expected != step.after {
                continue;
            }

            let conditions = required_conditions(base).map_err(|_| E::UnsupportedDomain)?;
            if step.evidence.len() != conditions.len() {
                return Err(E::InvalidConditions);
            }
            for (evidence, condition) in step.evidence.iter().zip(&conditions) {
                let actual = assumptions.assess(condition);
                if evidence.condition != *condition
                    || evidence.status != actual
                    || !matches!(actual, ConditionStatus::Proven | ConditionStatus::Assumed)
                {
                    return Err(E::InvalidConditions);
                }
            }
            return Ok(());
        }
    }

    Err(E::InvalidStep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathema::Symbol;

    #[test]
    fn rejects_forged_composite_rewrite_evidence() {
        let x = Expr::from(Symbol::new("x").unwrap());
        let base = Expr::add([x, 1.into()]);
        let expression = Expr::mul([base.clone(), Expr::pow(base.clone(), (-1).into())]);
        let assumptions = Assumptions::from_conditions([Condition::NonZero(base)]).unwrap();
        let valid = match cancel_composite_inverse_once(&expression, &assumptions).unwrap() {
            ConditionalRewriteOutcome::Applied(step) => step,
            other => panic!("unexpected result: {other:?}"),
        };
        assert_eq!(verify_composite_inverse_step(&valid, &assumptions), Ok(()));

        let mut forged = valid.clone();
        forged.after = Expr::from(7);
        assert_eq!(
            verify_composite_inverse_step(&forged, &assumptions),
            Err(ConditionalRewriteVerificationError::InvalidStep)
        );

        let mut forged = valid.clone();
        forged.evidence[0].status = ConditionStatus::Proven;
        assert_eq!(
            verify_composite_inverse_step(&forged, &assumptions),
            Err(ConditionalRewriteVerificationError::InvalidConditions)
        );

        let mut forged = valid.clone();
        forged.evidence[0].condition = Condition::NonZero(Expr::from(2));
        assert_eq!(
            verify_composite_inverse_step(&forged, &assumptions),
            Err(ConditionalRewriteVerificationError::InvalidConditions)
        );

        let mut forged = valid;
        forged.evidence.clear();
        assert_eq!(
            verify_composite_inverse_step(&forged, &assumptions),
            Err(ConditionalRewriteVerificationError::InvalidConditions)
        );
    }
}
