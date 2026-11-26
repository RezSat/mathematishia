//! Checked preconditions for the existing real and complex scalar grammar.

use std::{cmp::Ordering, error::Error, fmt};

use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ExprView, Integer, ScalarDomain,
    definedness_obligations, definedness_obligations_in_domain,
};

use crate::{Assumptions, ConditionStatus};

const MAX_NODES: usize = 2_048;
const MAX_DEPTH: usize = 64;
const MAX_CONDITIONS: usize = 128;
const MAX_SCALAR_BITS: u64 = 4_096;
const MAX_NAME_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainPreconditionError {
    UnsupportedDomain,
    UnboundSource,
    Definedness(DefinednessError),
    DepthLimit,
    ScalarLimit,
    NameLimit,
}

impl fmt::Display for DomainPreconditionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDomain => f.write_str("unsupported scalar domain"),
            Self::UnboundSource => f.write_str("source has unbound or externally supplied history"),
            Self::Definedness(error) => error.fmt(f),
            Self::DepthLimit => f.write_str("domain precondition depth limit exceeded"),
            Self::ScalarLimit => f.write_str("domain precondition scalar limit exceeded"),
            Self::NameLimit => f.write_str("domain precondition name limit exceeded"),
        }
    }
}

impl Error for DomainPreconditionError {}

impl From<DefinednessError> for DomainPreconditionError {
    fn from(error: DefinednessError) -> Self {
        Self::Definedness(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainPreconditionStatus {
    Proven,
    Conditional,
    Blocked,
    Refuted,
}

/// The assessment includes original restrictions, their nested obligations,
/// and explicit premises. It is not a certificate of expression equivalence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainPreconditionReport {
    status: DomainPreconditionStatus,
    construction_domain: Option<ScalarDomain>,
    external_history: bool,
    conditions: Vec<(Condition, ConditionStatus)>,
}

impl DomainPreconditionReport {
    pub fn status(&self) -> DomainPreconditionStatus {
        self.status
    }

    /// The domain declared during construction, not the caller's requested domain.
    pub fn construction_domain(&self) -> Option<ScalarDomain> {
        self.construction_domain
    }

    pub fn has_external_history(&self) -> bool {
        self.external_history
    }

    /// Requires a declared construction domain without externally supplied
    /// restrictions. This cannot recover history erased in a raw Expr before
    /// DomainExpression construction, or prove transformation equivalence.
    pub fn check_bound(
        source: &DomainExpression,
        domain: ScalarDomain,
        assumptions: &Assumptions,
    ) -> Result<Self, DomainPreconditionError> {
        source.check_construction_domain(domain)?;
        if source.construction_domain().is_none() || source.has_external_history() {
            return Err(DomainPreconditionError::UnboundSource);
        }
        check_domain_preconditions(source, domain, assumptions)
    }

    pub fn conditions(&self) -> &[(Condition, ConditionStatus)] {
        &self.conditions
    }
}

fn inspect(expression: &Expr, remaining: &mut usize) -> Result<(), DomainPreconditionError> {
    let mut pending = vec![(expression, 0)];
    while let Some((node, depth)) = pending.pop() {
        *remaining = remaining
            .checked_sub(1)
            .ok_or(DefinednessError::NodeLimitExceeded)?;
        if depth > MAX_DEPTH {
            return Err(DomainPreconditionError::DepthLimit);
        }
        match node.view() {
            ExprView::Integer(n) if n.bit_len() > MAX_SCALAR_BITS => {
                return Err(DomainPreconditionError::ScalarLimit);
            }
            ExprView::Rational(q)
                if q.numerator().bit_len() > MAX_SCALAR_BITS
                    || q.denominator().bit_len() > MAX_SCALAR_BITS =>
            {
                return Err(DomainPreconditionError::ScalarLimit);
            }
            ExprView::Symbol(s) if s.name().len() > MAX_NAME_BYTES => {
                return Err(DomainPreconditionError::NameLimit);
            }
            ExprView::Function { name, argument } => {
                if name.name().len() > MAX_NAME_BYTES {
                    return Err(DomainPreconditionError::NameLimit);
                }
                pending.push((argument, depth + 1));
            }
            ExprView::Pow { base, exponent } => {
                pending.push((exponent, depth + 1));
                pending.push((base, depth + 1));
            }
            ExprView::Add(children) | ExprView::Mul(children) => {
                if children.len() > remaining.saturating_sub(pending.len()) {
                    return Err(DefinednessError::NodeLimitExceeded.into());
                }
                pending.extend(children.iter().map(|child| (child, depth + 1)));
            }
            _ => {}
        }
        if pending.len() > *remaining {
            return Err(DefinednessError::NodeLimitExceeded.into());
        }
    }
    Ok(())
}

fn append(
    conditions: &mut Vec<Condition>,
    condition: Condition,
) -> Result<(), DomainPreconditionError> {
    if !conditions.contains(&condition) {
        if conditions.len() == MAX_CONDITIONS {
            return Err(DefinednessError::ConditionLimitExceeded.into());
        }
        conditions.push(condition);
    }
    Ok(())
}

fn collect(
    expression: &Expr,
    domain: Option<ScalarDomain>,
    conditions: &mut Vec<Condition>,
) -> Result<(), DomainPreconditionError> {
    let required = match domain {
        Some(domain) => definedness_obligations_in_domain(expression, MAX_NODES, domain),
        None => definedness_obligations(expression, MAX_NODES),
    }?;
    for condition in required {
        append(conditions, condition)?;
    }
    Ok(())
}

// Sign of exact literals, their integer powers and products, without expanding
// large powers. Zero to a nonpositive power has no sign.
fn exact_sign(expression: &Expr) -> Option<Ordering> {
    use Ordering::{Equal, Greater, Less};

    match expression.view() {
        ExprView::Integer(n) => Some(n.cmp(&Integer::zero())),
        ExprView::Rational(q) => Some(q.numerator().cmp(&Integer::zero())),
        ExprView::Pow { base, exponent } => {
            let ExprView::Integer(n) = exponent.view() else {
                return None;
            };
            match exact_sign(base)? {
                Equal if n <= &Integer::zero() => None,
                Equal => Some(Equal),
                Greater => Some(Greater),
                Less => {
                    if n.modulo_u32(2) == Some(0) {
                        Some(Greater)
                    } else {
                        Some(Less)
                    }
                }
            }
        }
        ExprView::Mul(factors) => {
            let mut negative = false;
            for factor in factors {
                match exact_sign(factor)? {
                    Equal => return Some(Equal),
                    Less => negative = !negative,
                    Greater => {}
                }
            }
            Some(if negative { Less } else { Greater })
        }
        _ => None,
    }
}

fn assess(condition: &Condition, assumptions: &Assumptions) -> ConditionStatus {
    let exact = match condition {
        Condition::NonZero(expr) => exact_sign(expr).map(|sign| sign != Ordering::Equal),
        Condition::Positive(expr) => exact_sign(expr).map(|sign| sign == Ordering::Greater),
        _ => None,
    };
    if let Some(holds) = exact {
        return if holds {
            ConditionStatus::Proven
        } else {
            ConditionStatus::Refuted
        };
    }
    // Exact refutations take precedence over implications from premises.
    let independent = Assumptions::new().assess(condition);
    if independent == ConditionStatus::Refuted {
        independent
    } else {
        assumptions.assess(condition)
    }
}

/// Inspect a normalized source and all retained and explicit conditions in a
/// selected real or complex domain. Symbols range over that domain; this
/// checks definedness, not complex branch identities or arbitrary membership.
/// Bound sources must be checked in their original construction domain.
/// For legacy unbound sources the caller supplies the domain; admission does
/// not recover missing provenance and is not a domain conversion API.
///
/// All input trees share a fixed node budget. Assumed conditions never become
/// unconditional facts. Unrecognized functions and noninteger powers refuse
/// admission even in erased construction history or unused premises.
///
/// Raw Expr normalization cannot recover history that was discarded before a
/// DomainExpression was constructed. Callers must retain the original source
/// and independently verify any proposed transformation.
pub fn check_domain_preconditions(
    source: &DomainExpression,
    domain: ScalarDomain,
    assumptions: &Assumptions,
) -> Result<DomainPreconditionReport, DomainPreconditionError> {
    if !matches!(domain, ScalarDomain::Real | ScalarDomain::Complex) {
        return Err(DomainPreconditionError::UnsupportedDomain);
    }
    source.check_construction_domain(domain)?;
    check_preconditions(source, Some(domain), assumptions)
}

/// The legacy rewrite API has no domain argument. Unbound sources must use
/// the domain-independent algebraic grammar, including historical conditions.
pub(crate) fn check_rewrite_preconditions(
    source: &DomainExpression,
    assumptions: &Assumptions,
) -> Result<DomainPreconditionReport, DefinednessError> {
    let domain = source
        .construction_domain()
        .filter(|domain| matches!(domain, ScalarDomain::Real | ScalarDomain::Complex));
    check_preconditions(source, domain, assumptions).map_err(|error| match error {
        DomainPreconditionError::Definedness(error) => error,
        _ => DefinednessError::NodeLimitExceeded,
    })
}

fn check_preconditions(
    source: &DomainExpression,
    domain: Option<ScalarDomain>,
    assumptions: &Assumptions,
) -> Result<DomainPreconditionReport, DomainPreconditionError> {
    if source
        .conditions()
        .len()
        .saturating_add(assumptions.premises().len())
        > MAX_CONDITIONS
    {
        return Err(DefinednessError::ConditionLimitExceeded.into());
    }
    let mut remaining = MAX_NODES;
    inspect(source.expression(), &mut remaining)?;
    let inputs = source.conditions().iter().chain(assumptions.premises());
    for condition in inputs.clone() {
        inspect(condition.expression(), &mut remaining)?;
    }

    let mut conditions = Vec::new();
    for condition in inputs {
        append(&mut conditions, condition.clone())?;
        collect(condition.expression(), domain, &mut conditions)?;
    }
    collect(source.expression(), domain, &mut conditions)?;

    let conditions: Vec<_> = conditions
        .into_iter()
        .map(|condition| {
            let status = assess(&condition, assumptions);
            (condition, status)
        })
        .collect();
    let has = |status| conditions.iter().any(|(_, actual)| *actual == status);
    let status = if has(ConditionStatus::Refuted) {
        DomainPreconditionStatus::Refuted
    } else if has(ConditionStatus::Unknown) {
        DomainPreconditionStatus::Blocked
    } else if has(ConditionStatus::Assumed) {
        DomainPreconditionStatus::Conditional
    } else {
        DomainPreconditionStatus::Proven
    };

    Ok(DomainPreconditionReport {
        status,
        construction_domain: source.construction_domain(),
        external_history: source.has_external_history(),
        conditions,
    })
}
