//! A first condition-aware scalar rewrite with independently checked evidence.

use std::error::Error;
use std::fmt;

use mathema::{Condition, Expr, ExprView};

use crate::{Assumptions, ConditionStatus, Relation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardedRuleId {
    CancelSymbolInverse,
}

impl GuardedRuleId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::CancelSymbolInverse => "algebra.cancel_symbol_inverse",
        }
    }
}

/// An equality step valid only when its recorded condition holds.
///
/// `Assumed` means conditional validity, not an unconditional identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardedRewriteStep {
    before: Expr,
    after: Expr,
    rule: GuardedRuleId,
    relation: Relation,
    condition: Condition,
    justification: ConditionStatus,
}

impl GuardedRewriteStep {
    pub fn before(&self) -> &Expr {
        &self.before
    }

    pub fn after(&self) -> &Expr {
        &self.after
    }

    pub const fn rule(&self) -> GuardedRuleId {
        self.rule
    }

    pub const fn relation(&self) -> Relation {
        self.relation
    }

    pub fn condition(&self) -> &Condition {
        &self.condition
    }

    pub const fn justification(&self) -> ConditionStatus {
        self.justification
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuardedRewriteOutcome {
    Applied(GuardedRewriteStep),
    Blocked(Condition),
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardedRewriteVerificationError {
    InvalidStep,
    UnmetCondition,
}

impl fmt::Display for GuardedRewriteVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStep => f.write_str("invalid conditional transformation"),
            Self::UnmetCondition => f.write_str("conditional transformation lacks a valid premise"),
        }
    }
}

impl Error for GuardedRewriteVerificationError {}

fn is_symbol_reciprocal(symbol: &Expr, other: &Expr) -> bool {
    matches!(
        other.view(),
        ExprView::Pow { base, exponent }
            if base == symbol && exponent == &Expr::from(-1)
    )
}

/// Cancels one symbolic `x * x^-1` pair in a product.
///
/// Only atomic symbols are supported: general bases may contain undefined
/// subexpressions whose domain would need separate tracking. Without a
/// justified nonzero condition, the rewrite returns a typed obligation.
pub fn cancel_symbol_inverse_once(
    expression: &Expr,
    assumptions: &Assumptions,
) -> GuardedRewriteOutcome {
    let ExprView::Mul(factors) = expression.view() else {
        return GuardedRewriteOutcome::NotApplicable;
    };
    let mut blocked = None;
    for (symbol_index, symbol) in factors.iter().enumerate() {
        if !matches!(symbol.view(), ExprView::Symbol(_)) {
            continue;
        }
        for (inverse_index, inverse) in factors.iter().enumerate() {
            if !is_symbol_reciprocal(symbol, inverse) {
                continue;
            }
            let condition = Condition::NonZero(symbol.clone());
            let justification = assumptions.assess(&condition);
            if matches!(
                justification,
                ConditionStatus::Proven | ConditionStatus::Assumed
            ) {
                let after = Expr::mul(
                    factors
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index != symbol_index && *index != inverse_index)
                        .map(|(_, factor)| factor.clone()),
                );
                return GuardedRewriteOutcome::Applied(GuardedRewriteStep {
                    before: expression.clone(),
                    after,
                    rule: GuardedRuleId::CancelSymbolInverse,
                    relation: Relation::Equal,
                    condition,
                    justification,
                });
            }
            if justification == ConditionStatus::Unknown && blocked.is_none() {
                blocked = Some(condition);
            }
        }
    }
    match blocked {
        Some(condition) => GuardedRewriteOutcome::Blocked(condition),
        None => GuardedRewriteOutcome::NotApplicable,
    }
}

/// Checks the claimed pair, exact result, rule, relation and premise status.
///
/// Verification does not invoke the rewrite generator. A forged equality or
/// a missing condition is rejected even when the rule identifier is correct.
pub fn verify_guarded_rewrite_step(
    step: &GuardedRewriteStep,
    assumptions: &Assumptions,
) -> Result<(), GuardedRewriteVerificationError> {
    if step.rule != GuardedRuleId::CancelSymbolInverse || step.relation != Relation::Equal {
        return Err(GuardedRewriteVerificationError::InvalidStep);
    }
    let Condition::NonZero(base) = &step.condition else {
        return Err(GuardedRewriteVerificationError::InvalidStep);
    };
    if !matches!(base.view(), ExprView::Symbol(_)) {
        return Err(GuardedRewriteVerificationError::InvalidStep);
    }
    let observed = assumptions.assess(&step.condition);
    if !matches!(observed, ConditionStatus::Proven | ConditionStatus::Assumed)
        || observed != step.justification
    {
        return Err(GuardedRewriteVerificationError::UnmetCondition);
    }
    let ExprView::Mul(factors) = step.before.view() else {
        return Err(GuardedRewriteVerificationError::InvalidStep);
    };
    for (i, left) in factors.iter().enumerate() {
        if left != base {
            continue;
        }
        for (j, right) in factors.iter().enumerate() {
            let ExprView::Pow { base: reciprocal_base, exponent } = right.view() else {
                continue;
            };
            if reciprocal_base != base || exponent != &Expr::from(-1) {
                continue;
            }
            let expected = Expr::mul(
                factors
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != i && *index != j)
                    .map(|(_, factor)| factor.clone()),
            );
            if expected == step.after {
                return Ok(());
            }
        }
    }
    Err(GuardedRewriteVerificationError::InvalidStep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathema::Symbol;

    #[test]
    fn forged_evidence_is_rejected() {
        let x = Expr::from(Symbol::new("x").unwrap());
        let before = Expr::mul([x.clone(), Expr::pow(x.clone(), (-1).into())]);
        let assumptions = Assumptions::from_conditions([Condition::NonZero(x.clone())]).unwrap();
        let valid = match cancel_symbol_inverse_once(&before, &assumptions) {
            GuardedRewriteOutcome::Applied(step) => step,
            other => panic!("expected applied rewrite, got {other:?}"),
        };
        assert_eq!(verify_guarded_rewrite_step(&valid, &assumptions), Ok(()));

        let mut forged = valid.clone();
        forged.after = Expr::from(42);
        assert_eq!(
            verify_guarded_rewrite_step(&forged, &assumptions),
            Err(GuardedRewriteVerificationError::InvalidStep)
        );

        let mut false_proof = valid.clone();
        false_proof.justification = ConditionStatus::Proven;
        assert_eq!(
            verify_guarded_rewrite_step(&false_proof, &assumptions),
            Err(GuardedRewriteVerificationError::UnmetCondition)
        );

        let mut false_condition = valid;
        false_condition.condition = Condition::NonZero(Expr::from(2));
        assert_eq!(
            verify_guarded_rewrite_step(&false_condition, &assumptions),
            Err(GuardedRewriteVerificationError::InvalidStep)
        );
    }
}
