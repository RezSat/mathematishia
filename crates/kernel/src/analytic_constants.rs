//! Bounded, exact, one-step analytic constant identities with domain provenance.
//!
//! No transcendental approximation, general analytic evaluation or branch
//! inversion is performed. Conditions are prerequisites, not proved facts.

use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ExprView, ScalarDomain, Symbol,
    definedness_obligations_in_domain,
};

use crate::{Assumptions, ConditionStatus};

const HARD_MAX_NODES: usize = 4_096;
const HARD_MAX_DEPTH: usize = 128;
const HARD_MAX_CONDITIONS: usize = 256;
const HARD_MAX_SCALAR_BITS: u64 = 4_096;
const HARD_MAX_WORK: usize = 65_536;

/// Independent per-call bounds. Values above the hard caps are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalyticConstantLimits {
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_conditions: usize,
    pub max_scalar_bits: u64,
    pub max_work: usize,
}

impl Default for AnalyticConstantLimits {
    fn default() -> Self {
        Self {
            max_nodes: 2_048,
            max_depth: 64,
            max_conditions: 64,
            max_scalar_bits: 4_096,
            max_work: 16_384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalyticConstantLimit {
    Nodes,
    Depth,
    Conditions,
    ScalarBits,
    Work,
    Configuration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalyticConstantUnsupported {
    Domain(ScalarDomain),
    Function(Symbol),
    Exponent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalyticConstantError {
    Limit(AnalyticConstantLimit),
    Unsupported(AnalyticConstantUnsupported),
    ContradictedCondition(Condition),
    Definedness(DefinednessError),
    InvalidRulePath,
}

impl std::fmt::Display for AnalyticConstantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AnalyticConstantError {}

/// Stable identity IDs, with no implication of analytic evaluation elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnalyticConstantRule {
    SineZero,
    CosineZero,
    TangentZero,
    ExponentialZero,
    LogarithmOne,
}

impl AnalyticConstantRule {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::SineZero => "analytic.sin_zero",
            Self::CosineZero => "analytic.cos_zero",
            Self::TangentZero => "analytic.tan_zero",
            Self::ExponentialZero => "analytic.exp_zero",
            Self::LogarithmOne => "analytic.log_one",
        }
    }
}

/// A caller-inspectable certificate. Public fields are untrusted; the verifier
/// recomputes the first permitted rewrite and all domain conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyticConstantRewrite {
    pub source: DomainExpression,
    pub result: DomainExpression,
    pub domain: ScalarDomain,
    /// Preorder child indices: function argument 0; power base/exponent 0/1.
    pub path: Vec<usize>,
    pub rule: AnalyticConstantRule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalyticConstantOutcome {
    NoRule { retained_conditions: Vec<Condition> },
    Exact(AnalyticConstantRewrite),
    Unsupported(AnalyticConstantUnsupported),
}

struct Meter {
    nodes: usize,
    work: usize,
}

impl Meter {
    fn visit(
        &mut self,
        depth: usize,
        limits: AnalyticConstantLimits,
    ) -> Result<(), AnalyticConstantError> {
        if depth > limits.max_depth {
            return Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Depth));
        }
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > limits.max_nodes {
            return Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Nodes));
        }
        self.charge(1, limits)
    }

    fn charge(
        &mut self,
        amount: usize,
        limits: AnalyticConstantLimits,
    ) -> Result<(), AnalyticConstantError> {
        self.work = self.work.saturating_add(amount);
        if self.work > limits.max_work {
            return Err(AnalyticConstantError::Limit(AnalyticConstantLimit::Work));
        }
        Ok(())
    }
}

fn validate_limits(limits: AnalyticConstantLimits) -> Result<(), AnalyticConstantError> {
    if limits.max_nodes == 0
        || limits.max_depth == 0
        || limits.max_scalar_bits == 0
        || limits.max_work == 0
        || limits.max_nodes > HARD_MAX_NODES
        || limits.max_depth > HARD_MAX_DEPTH
        || limits.max_conditions > HARD_MAX_CONDITIONS
        || limits.max_scalar_bits > HARD_MAX_SCALAR_BITS
        || limits.max_work > HARD_MAX_WORK
    {
        return Err(AnalyticConstantError::Limit(
            AnalyticConstantLimit::Configuration,
        ));
    }
    Ok(())
}

fn map_definedness(error: DefinednessError) -> AnalyticConstantError {
    match error {
        DefinednessError::UnsupportedFunction => AnalyticConstantError::Unsupported(
            AnalyticConstantUnsupported::Function(Symbol::new("unknown").unwrap()),
        ),
        DefinednessError::UnsupportedExponent => {
            AnalyticConstantError::Unsupported(AnalyticConstantUnsupported::Exponent)
        }
        DefinednessError::NodeLimitExceeded => {
            AnalyticConstantError::Limit(AnalyticConstantLimit::Nodes)
        }
        DefinednessError::ConditionLimitExceeded => {
            AnalyticConstantError::Limit(AnalyticConstantLimit::Conditions)
        }
        other => AnalyticConstantError::Definedness(other),
    }
}

fn rule_at(node: &Expr) -> Option<AnalyticConstantRule> {
    let ExprView::Function { name, argument } = node.view() else {
        return None;
    };
    let ExprView::Integer(value) = argument.view() else {
        return None;
    };
    match name.name() {
        "sin" if value.is_zero() => Some(AnalyticConstantRule::SineZero),
        "cos" if value.is_zero() => Some(AnalyticConstantRule::CosineZero),
        "tan" if value.is_zero() => Some(AnalyticConstantRule::TangentZero),
        "exp" if value.is_zero() => Some(AnalyticConstantRule::ExponentialZero),
        "log" if value.is_one() => Some(AnalyticConstantRule::LogarithmOne),
        _ => None,
    }
}

fn rule_value(rule: AnalyticConstantRule) -> Expr {
    match rule {
        AnalyticConstantRule::SineZero
        | AnalyticConstantRule::TangentZero
        | AnalyticConstantRule::LogarithmOne => Expr::from(0),
        AnalyticConstantRule::CosineZero | AnalyticConstantRule::ExponentialZero => Expr::from(1),
    }
}

#[derive(Clone)]
struct Hit {
    path: Vec<usize>,
    rule: AnalyticConstantRule,
}

/// Scan the entire expression before selecting a rule. Unsupported functions
/// and powers anywhere in the source prohibit partial rewriting.
fn scan(
    expression: &Expr,
    meter: &mut Meter,
    limits: AnalyticConstantLimits,
) -> Result<Option<Hit>, AnalyticConstantError> {
    let mut stack = vec![(expression, Vec::<usize>::new(), 0usize)];
    let mut first = None;
    while let Some((node, path, depth)) = stack.pop() {
        meter.visit(depth, limits)?;
        match node.view() {
            ExprView::Integer(n) => {
                if n.bit_len() > limits.max_scalar_bits {
                    return Err(AnalyticConstantError::Limit(
                        AnalyticConstantLimit::ScalarBits,
                    ));
                }
            }
            ExprView::Rational(q) => {
                if q.numerator().bit_len() > limits.max_scalar_bits
                    || q.denominator().bit_len() > limits.max_scalar_bits
                {
                    return Err(AnalyticConstantError::Limit(
                        AnalyticConstantLimit::ScalarBits,
                    ));
                }
            }
            ExprView::Symbol(s) => {
                if s.name().len() > 4_096 {
                    return Err(AnalyticConstantError::Limit(
                        AnalyticConstantLimit::ScalarBits,
                    ));
                }
            }
            ExprView::Function { name, argument } => {
                if !matches!(name.name(), "sin" | "cos" | "tan" | "exp" | "log") {
                    return Err(AnalyticConstantError::Unsupported(
                        AnalyticConstantUnsupported::Function(name.clone()),
                    ));
                }
                if first.is_none() {
                    if let Some(rule) = rule_at(node) {
                        first = Some(Hit {
                            path: path.clone(),
                            rule,
                        });
                    }
                }
                let mut child = path;
                child.push(0);
                stack.push((argument, child, depth + 1));
            }
            ExprView::Pow { base, exponent } => {
                if !matches!(exponent.view(), ExprView::Integer(_)) {
                    return Err(AnalyticConstantError::Unsupported(
                        AnalyticConstantUnsupported::Exponent,
                    ));
                }
                let mut right = path.clone();
                right.push(1);
                stack.push((exponent, right, depth + 1));
                let mut left = path;
                left.push(0);
                stack.push((base, left, depth + 1));
            }
            ExprView::Add(children) | ExprView::Mul(children) => {
                for (index, child) in children.iter().enumerate().rev() {
                    let mut next = path.clone();
                    next.push(index);
                    stack.push((child, next, depth + 1));
                }
            }
        }
    }
    Ok(first)
}

fn add_condition(
    conditions: &mut Vec<Condition>,
    condition: Condition,
    meter: &mut Meter,
    limits: AnalyticConstantLimits,
) -> Result<(), AnalyticConstantError> {
    meter.charge(conditions.len().saturating_add(1), limits)?;
    if !conditions.contains(&condition) {
        if conditions.len() >= limits.max_conditions {
            return Err(AnalyticConstantError::Limit(
                AnalyticConstantLimit::Conditions,
            ));
        }
        conditions.push(condition);
    }
    Ok(())
}

fn validate_condition(
    condition: &Condition,
    meter: &mut Meter,
    limits: AnalyticConstantLimits,
) -> Result<(), AnalyticConstantError> {
    // Conditions are also user-controlled expression trees; do not exempt them
    // from node/bit/depth/unknown-function admission checks.
    scan(condition.expression(), meter, limits)?;
    if Assumptions::new().assess(condition) == ConditionStatus::Refuted {
        return Err(AnalyticConstantError::ContradictedCondition(
            condition.clone(),
        ));
    }
    Ok(())
}

fn conditions_for(
    source: &DomainExpression,
    domain: ScalarDomain,
    meter: &mut Meter,
    limits: AnalyticConstantLimits,
) -> Result<(Vec<Condition>, Option<Hit>), AnalyticConstantError> {
    let first = scan(source.expression(), meter, limits)?;
    let derived = definedness_obligations_in_domain(source.expression(), limits.max_nodes, domain)
        .map_err(map_definedness)?;
    let mut conditions = Vec::new();
    for condition in source.conditions().iter().cloned().chain(derived) {
        add_condition(&mut conditions, condition, meter, limits)?;
    }
    for condition in &conditions {
        validate_condition(condition, meter, limits)?;
    }
    Ok((conditions, first))
}

fn replace_at(
    node: &Expr,
    path: &[usize],
    replacement: &Expr,
) -> Result<Expr, AnalyticConstantError> {
    let Some((&index, rest)) = path.split_first() else {
        return Ok(replacement.clone());
    };
    match node.view() {
        ExprView::Function { name, argument } if index == 0 => Ok(Expr::function(
            name.clone(),
            replace_at(argument, rest, replacement)?,
        )),
        ExprView::Pow { base, exponent } if index < 2 => {
            let (base, exponent) = if index == 0 {
                (replace_at(base, rest, replacement)?, exponent.clone())
            } else {
                (base.clone(), replace_at(exponent, rest, replacement)?)
            };
            Ok(Expr::pow(base, exponent))
        }
        ExprView::Add(children) | ExprView::Mul(children) if index < children.len() => {
            let replaced = children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    if i == index {
                        replace_at(child, rest, replacement)
                    } else {
                        Ok(child.clone())
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            if matches!(node.view(), ExprView::Add(_)) {
                Ok(Expr::add(replaced))
            } else {
                Ok(Expr::mul(replaced))
            }
        }
        _ => Err(AnalyticConstantError::InvalidRulePath),
    }
}

fn build_result(
    after: Expr,
    inherited: Vec<Condition>,
    domain: ScalarDomain,
    limits: AnalyticConstantLimits,
) -> Result<DomainExpression, AnalyticConstantError> {
    let own = DomainExpression::from_expr_in_domain(
        after,
        domain,
        limits.max_nodes,
        limits.max_conditions,
    )
    .map_err(map_definedness)?;
    // Compose with a zero-valued carrier: Expr::add(0, after) = after.
    // This is the available domain-preserving constructor; it never removes
    // construction-history restrictions even if normalization removes a term.
    let carrier = DomainExpression::with_conditions(
        Expr::from(0),
        inherited,
        limits.max_nodes,
        limits.max_conditions,
    )
    .map_err(map_definedness)?;
    DomainExpression::add([carrier, own], limits.max_conditions).map_err(map_definedness)
}

/// Applies the first preorder identity, or explicitly reports absence/unsupported
/// semantics. No historical condition is discharged, including provable constants.
pub fn rewrite_analytic_constant_once(
    source: &DomainExpression,
    domain: ScalarDomain,
    limits: AnalyticConstantLimits,
) -> Result<AnalyticConstantOutcome, AnalyticConstantError> {
    validate_limits(limits)?;
    if !matches!(domain, ScalarDomain::Real | ScalarDomain::Complex) {
        return Ok(AnalyticConstantOutcome::Unsupported(
            AnalyticConstantUnsupported::Domain(domain),
        ));
    }
    let mut meter = Meter { nodes: 0, work: 0 };
    let (conditions, first) = match conditions_for(source, domain, &mut meter, limits) {
        Ok(value) => value,
        Err(AnalyticConstantError::Unsupported(unsupported)) => {
            return Ok(AnalyticConstantOutcome::Unsupported(unsupported));
        }
        Err(error) => return Err(error),
    };
    let Some(hit) = first else {
        return Ok(AnalyticConstantOutcome::NoRule {
            retained_conditions: conditions,
        });
    };
    let after = replace_at(source.expression(), &hit.path, &rule_value(hit.rule))?;
    let result = build_result(after, conditions, domain, limits)?;
    // Revalidate the result and its newly introduced obligations. This rejects
    // a rewrite whose parent becomes exactly undefined (e.g. log(sin(0))).
    let mut final_meter = Meter { nodes: 0, work: 0 };
    conditions_for(&result, domain, &mut final_meter, limits)?;
    Ok(AnalyticConstantOutcome::Exact(AnalyticConstantRewrite {
        source: source.clone(),
        result,
        domain,
        path: hit.path,
        rule: hit.rule,
    }))
}

/// Independently checks source identity, deterministic location, local rule,
/// exact recomposition, and *identical* accumulated domain restrictions.
pub fn verify_analytic_constant_rewrite(
    expected_source: &DomainExpression,
    certificate: &AnalyticConstantRewrite,
    limits: AnalyticConstantLimits,
) -> Result<bool, AnalyticConstantError> {
    validate_limits(limits)?;
    if expected_source != &certificate.source
        || !matches!(
            certificate.domain,
            ScalarDomain::Real | ScalarDomain::Complex
        )
    {
        return Ok(false);
    }
    let mut meter = Meter { nodes: 0, work: 0 };
    let (inherited, first) =
        conditions_for(expected_source, certificate.domain, &mut meter, limits)?;
    let Some(hit) = first else {
        return Ok(false);
    };
    if hit.path != certificate.path || hit.rule != certificate.rule {
        return Ok(false);
    }
    // Reconstruct the local identity separately; never trust the asserted
    // post-state or the rule label alone.
    let mut node = expected_source.expression();
    for &index in &certificate.path {
        node = match node.view() {
            ExprView::Function { argument, .. } if index == 0 => argument,
            ExprView::Pow { base, .. } if index == 0 => base,
            ExprView::Pow { exponent, .. } if index == 1 => exponent,
            ExprView::Add(children) | ExprView::Mul(children) => {
                let Some(child) = children.get(index) else {
                    return Ok(false);
                };
                child
            }
            _ => return Ok(false),
        };
    }
    if rule_at(node) != Some(certificate.rule) {
        return Ok(false);
    }
    let reconstructed = replace_at(
        expected_source.expression(),
        &certificate.path,
        &rule_value(certificate.rule),
    )?;
    if &reconstructed != certificate.result.expression() {
        return Ok(false);
    }
    let expected = build_result(reconstructed, inherited, certificate.domain, limits)?;
    let mut final_meter = Meter { nodes: 0, work: 0 };
    conditions_for(&expected, certificate.domain, &mut final_meter, limits)?;
    Ok(expected == certificate.result)
}
