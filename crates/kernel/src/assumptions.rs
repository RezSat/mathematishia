//! Exact scalar facts and explicitly supplied mathematical premises.
//!
//! This is a conservative implication checker, not an unrestricted symbolic
//! theorem prover or a definedness analysis for arbitrary expressions.

use std::error::Error;
use std::fmt;

use mathema::{Condition, Expr, ExprView, Integer, Rational, ScalarDomain};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionStatus {
    /// Established by exact evaluation of a scalar literal.
    Proven,
    /// Follows from an explicit premise under the documented implication rules.
    Assumed,
    /// Neither established nor contradicted.
    Unknown,
    /// Contradicted by an exact scalar value.
    Refuted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssumptionError {
    RefutedPremise(Condition),
}

impl fmt::Display for AssumptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RefutedPremise(_) => f.write_str("premise contradicts an exact scalar value"),
        }
    }
}

impl Error for AssumptionError {}

/// A finite set of premises, retaining their first meaningful insertion order.
///
/// Redundant premises are not stored. A premise cannot override a fact refuted
/// by exact rational arithmetic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Assumptions {
    premises: Vec<Condition>,
}

impl Assumptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn premises(&self) -> &[Condition] {
        &self.premises
    }

    pub fn from_conditions(
        conditions: impl IntoIterator<Item = Condition>,
    ) -> Result<Self, AssumptionError> {
        let mut assumptions = Self::new();
        for condition in conditions {
            assumptions.add(condition)?;
        }
        Ok(assumptions)
    }

    pub fn add(&mut self, condition: Condition) -> Result<(), AssumptionError> {
        match self.assess(&condition) {
            ConditionStatus::Refuted => Err(AssumptionError::RefutedPremise(condition)),
            ConditionStatus::Unknown => {
                self.premises.push(condition);
                Ok(())
            }
            ConditionStatus::Proven | ConditionStatus::Assumed => Ok(()),
        }
    }

    /// Distinguishes exact facts from consequences of user-supplied premises.
    ///
    /// No symbolic sign inference, expression definedness, or general
    /// simplification is attempted.
    pub fn assess(&self, condition: &Condition) -> ConditionStatus {
        if let Some(status) = evaluate_exact_scalar(condition) {
            return status;
        }
        if self
            .premises
            .iter()
            .any(|premise| premise_implies(premise, condition))
        {
            ConditionStatus::Assumed
        } else {
            ConditionStatus::Unknown
        }
    }
}

fn scalar_value(expr: &Expr) -> Option<Rational> {
    match expr.view() {
        ExprView::Integer(value) => Some(Rational::from(value.clone())),
        ExprView::Rational(value) => Some(value.clone()),
        _ => None,
    }
}

fn evaluate_exact_scalar(condition: &Condition) -> Option<ConditionStatus> {
    let scalar = scalar_value(condition.expression())?;
    let true_condition = match condition {
        Condition::NonZero(_) => !scalar.numerator().is_zero(),
        Condition::Positive(_) => scalar > Rational::from(Integer::zero()),
        Condition::Real(_) => true,
        Condition::Integer(_) => scalar.denominator().is_one(),
        Condition::InDomain { domain, .. } => match domain {
            ScalarDomain::Integer => scalar.denominator().is_one(),
            ScalarDomain::Rational | ScalarDomain::Real | ScalarDomain::Complex => true,
        },
    };
    Some(if true_condition {
        ConditionStatus::Proven
    } else {
        ConditionStatus::Refuted
    })
}

const fn domain_rank(domain: ScalarDomain) -> u8 {
    match domain {
        ScalarDomain::Integer => 0,
        ScalarDomain::Rational => 1,
        ScalarDomain::Real => 2,
        ScalarDomain::Complex => 3,
    }
}

fn premise_implies(premise: &Condition, goal: &Condition) -> bool {
    if premise.expression() != goal.expression() {
        return false;
    }
    if premise == goal {
        return true;
    }
    match premise {
        Condition::NonZero(_) => false,
        Condition::Positive(_) => matches!(
            goal,
            Condition::NonZero(_)
                | Condition::Real(_)
                | Condition::InDomain {
                    domain: ScalarDomain::Real | ScalarDomain::Complex,
                    ..
                }
        ),
        Condition::Integer(_) => match goal {
            Condition::Real(_) => true,
            Condition::InDomain { .. } => true,
            _ => false,
        },
        Condition::Real(_) => matches!(
            goal,
            Condition::InDomain {
                domain: ScalarDomain::Real | ScalarDomain::Complex,
                ..
            }
        ),
        Condition::InDomain { domain, .. } => match goal {
            Condition::Integer(_) => *domain == ScalarDomain::Integer,
            Condition::Real(_) => domain_rank(*domain) <= domain_rank(ScalarDomain::Real),
            Condition::InDomain {
                domain: required, ..
            } => domain_rank(*domain) <= domain_rank(*required),
            _ => false,
        },
    }
}
