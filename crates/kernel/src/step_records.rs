//! Typed, bounded projections of independently checked mathematical evidence.
//!
//! Records are not certificates on their own. Verification requires the
//! original source and rule-specific evidence, never a display string.

use std::{error::Error, fmt};

use mathema::{
    Condition, DomainExpression, Equation, Expr, ExprView, Integer, ScalarDomain, Symbol,
    definedness_obligations_in_domain,
};

use crate::budget::WorkBudget;
use crate::{
    Assumptions, ConditionStatus, DerivativeRuleId, DistributionResult, DomainPreconditionStatus,
    EquationRuleId, EvaluationContext, EvaluationError, LinearSolution, LinearSolveResult,
    MultivariateDivision, MultivariateDivisionError, MultivariateLimits, MultivariatePolynomial,
    RationalPolynomial, Relation, ResourceLimits, RuleId, SymbolicDerivative, VerifiedCondition,
    check_domain_preconditions, verify_distribution, verify_linear_solution,
    verify_multivariate_division, verify_symbolic_derivative,
};

const MAX_STEPS: usize = 128;
const MAX_NODES: usize = 8_192;
const MAX_TREE_NODES: usize = 128;
const MAX_CONDITIONS: usize = 32;
const MAX_NAME_BYTES: usize = 256;
const MAX_WORK: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepLimits {
    pub max_steps: usize,
    pub max_nodes: usize,
    pub max_work_units: u64,
}

impl Default for StepLimits {
    fn default() -> Self {
        Self {
            max_steps: MAX_STEPS,
            max_nodes: MAX_NODES,
            max_work_units: MAX_WORK,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepRule {
    Distribution(RuleId),
    Equation(EquationRuleId),
    Derivative(DerivativeRuleId),
    PolynomialDivision,
    MultivariateDivision,
}

impl StepRule {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Distribution(rule) => rule.stable_id(),
            Self::Equation(rule) => rule.stable_id(),
            Self::Derivative(rule) => rule.stable_id(),
            Self::PolynomialDivision => "polynomial.division_recomposition",
            Self::MultivariateDivision => "polynomial.ordered_division_recomposition",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepEvidenceSource {
    Distribution(usize),
    Linear(usize),
    Derivative(usize),
    PolynomialDivisionFinal,
    MultivariateDivisionFinal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepCondition {
    /// An index into the original source conditions, not a proved fact.
    Source(usize),
    /// An exact nonzero coefficient checked by the equation verifier.
    Proven(VerifiedCondition),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepClaim {
    Equal {
        before: Expr,
        after: Expr,
    },
    Equivalent {
        before: Equation,
        after: Equation,
        variable: Symbol,
    },
    DerivativeOf {
        before: Expr,
        after: Expr,
        variable: Symbol,
    },
    /// A final recomposition with remainder degree smaller than the divisor.
    PolynomialDivision {
        dividend: RationalPolynomial,
        divisor: RationalPolynomial,
        quotient: RationalPolynomial,
        remainder: RationalPolynomial,
    },
    /// Exact recomposition and a reduced remainder, not a unique normal form.
    MultivariateDivision {
        dividend: MultivariatePolynomial,
        ordered_divisors: Vec<MultivariatePolynomial>,
        quotients: Vec<MultivariatePolynomial>,
        remainder: MultivariatePolynomial,
    },
}

impl StepClaim {
    pub const fn relation(&self) -> Relation {
        match self {
            Self::Equal { .. }
            | Self::PolynomialDivision { .. }
            | Self::MultivariateDivision { .. } => Relation::Equal,
            Self::Equivalent { .. } => Relation::Equivalent,
            Self::DerivativeOf { .. } => Relation::DerivativeOf,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepRecord {
    pub index: usize,
    pub claim: StepClaim,
    pub rule: StepRule,
    /// Earlier record indices, in operand order for derivative rules.
    pub premises: Vec<usize>,
    pub conditions: Vec<StepCondition>,
    pub evidence: StepEvidenceSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepConclusion {
    Distribution {
        original: Expr,
        result: Expr,
        complete: bool,
    },
    Linear {
        original: Equation,
        variable: Symbol,
        solution: LinearSolution,
    },
    Derivative {
        original: Expr,
        variable: Symbol,
        result: Expr,
    },
    FinalDivision,
}

/// Output-only structural records. Public fields remain untrusted until
/// checked against the original evidence with verify_step_records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepRecords {
    pub domain: ScalarDomain,
    pub source_conditions: Vec<Condition>,
    pub steps: Vec<StepRecord>,
    pub conclusion: StepConclusion,
}

/// The source is supplied separately from the claimed result. Native
/// polynomial inputs denote unrestricted Q-polynomials; parsed expressions
/// must retain their original DomainExpression before normalization.
#[derive(Clone, Copy, Debug)]
pub enum StepEvidence<'a> {
    Distribution {
        source: &'a DomainExpression,
        domain: ScalarDomain,
        result: &'a DistributionResult,
    },
    Linear {
        left: &'a DomainExpression,
        right: &'a DomainExpression,
        variable: &'a Symbol,
        result: &'a LinearSolveResult,
    },
    Derivative {
        source: &'a DomainExpression,
        domain: ScalarDomain,
        variable: &'a Symbol,
        result: &'a SymbolicDerivative,
    },
    PolynomialDivision {
        dividend: &'a RationalPolynomial,
        divisor: &'a RationalPolynomial,
        quotient: &'a RationalPolynomial,
        remainder: &'a RationalPolynomial,
    },
    MultivariateDivision {
        dividend: &'a MultivariatePolynomial,
        divisors: &'a [MultivariatePolynomial],
        result: &'a MultivariateDivision,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepRecordError {
    ResourceLimit,
    SourceMismatch,
    UnsupportedDomain,
    InvalidDomain,
    InvalidEvidence,
    InvalidRecord,
    Evaluation(EvaluationError),
    Multivariate(MultivariateDivisionError),
}

impl fmt::Display for StepRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceLimit => f.write_str("step record resource limit exceeded"),
            Self::SourceMismatch => f.write_str("evidence does not match the original source"),
            Self::UnsupportedDomain => f.write_str("unsupported derivation domain"),
            Self::InvalidDomain => f.write_str("missing or refuted domain obligations"),
            Self::InvalidEvidence => f.write_str("invalid mathematical evidence"),
            Self::InvalidRecord => f.write_str("record does not match checked evidence"),
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Multivariate(error) => write!(f, "{error}"),
        }
    }
}

impl Error for StepRecordError {}

impl From<EvaluationError> for StepRecordError {
    fn from(error: EvaluationError) -> Self {
        match error {
            EvaluationError::Limit(_) => Self::ResourceLimit,
            _ => Self::Evaluation(error),
        }
    }
}

impl From<MultivariateDivisionError> for StepRecordError {
    fn from(error: MultivariateDivisionError) -> Self {
        Self::Multivariate(error)
    }
}

type RecordResult<T> = Result<T, StepRecordError>;

struct Budget {
    nodes: usize,
    steps: usize,
    context: EvaluationContext,
    work: WorkBudget,
}

impl Budget {
    fn new(limits: StepLimits) -> Self {
        let resources = ResourceLimits {
            max_expression_nodes: MAX_TREE_NODES,
            max_expression_depth: 32,
            max_scalar_bits: 256,
            max_intermediate_scalar_bits: 4_096,
            max_work_units: limits.max_work_units.min(MAX_WORK),
            ..ResourceLimits::default()
        };
        Self {
            nodes: limits.max_nodes.min(MAX_NODES),
            steps: limits.max_steps.min(MAX_STEPS),
            context: EvaluationContext::new(resources),
            work: WorkBudget::new(resources),
        }
    }

    fn consume(&mut self, count: usize) -> RecordResult<()> {
        self.nodes = self
            .nodes
            .checked_sub(count)
            .ok_or(StepRecordError::ResourceLimit)?;
        self.work.charge(count as u64)?;
        Ok(())
    }

    fn check_steps(&self, count: usize) -> RecordResult<()> {
        if count > self.steps {
            Err(StepRecordError::ResourceLimit)
        } else {
            Ok(())
        }
    }

    fn expression(&mut self, expression: &Expr) -> RecordResult<u64> {
        let mut pending = vec![(expression, 1_usize)];
        let mut count = 0_u64;
        while let Some((node, depth)) = pending.pop() {
            self.consume(1)?;
            count += 1;
            if depth > 32 || count > MAX_TREE_NODES as u64 {
                return Err(StepRecordError::ResourceLimit);
            }
            match node.view() {
                ExprView::Symbol(symbol) => check_name(symbol)?,
                ExprView::Function { name, argument } => {
                    check_name(name)?;
                    pending.push((argument, depth + 1));
                }
                ExprView::Add(children) | ExprView::Mul(children) => {
                    if children.len() > MAX_TREE_NODES.saturating_sub(pending.len()) {
                        return Err(StepRecordError::ResourceLimit);
                    }
                    pending.extend(children.iter().map(|child| (child, depth + 1)));
                }
                ExprView::Pow { base, exponent } => {
                    if let ExprView::Integer(value) = exponent.view()
                        && (value > &Integer::from(32) || value < &Integer::from(-32))
                    {
                        return Err(StepRecordError::ResourceLimit);
                    }
                    pending.push((base, depth + 1));
                    pending.push((exponent, depth + 1));
                }
                _ => {}
            }
        }
        self.context.check_expression(expression)?;
        Ok(count)
    }

    fn polynomial(&mut self, value: &RationalPolynomial) -> RecordResult<()> {
        self.context.check_polynomial(value)?;
        self.consume(value.coefficients().len())
    }

    fn equation(&mut self, equation: &Equation, variable: &Symbol) -> RecordResult<()> {
        for side in [equation.left(), equation.right()] {
            self.expression(side)?;
            let converted = self.work.convert(side, variable)?;
            self.context.check_polynomial(&converted)?;
        }
        Ok(())
    }

    fn multivariate(&mut self, value: &MultivariatePolynomial) -> RecordResult<()> {
        if value.variables().len() > 8
            || value.terms().len() > 64
            || value.total_degree().is_some_and(|degree| degree > 32)
        {
            return Err(StepRecordError::ResourceLimit);
        }
        for symbol in value.variables() {
            check_name(symbol)?;
        }
        self.consume(value.variables().len() + value.terms().len())?;
        for coefficient in value.terms().values() {
            self.context.check_rational(coefficient)?;
        }
        Ok(())
    }
}

fn check_name(symbol: &Symbol) -> RecordResult<()> {
    if symbol.name().len() > MAX_NAME_BYTES {
        Err(StepRecordError::ResourceLimit)
    } else {
        Ok(())
    }
}

fn source_assumptions(
    source: &DomainExpression,
    domain: ScalarDomain,
    budget: &mut Budget,
) -> RecordResult<Assumptions> {
    if !matches!(domain, ScalarDomain::Real | ScalarDomain::Complex) {
        return Err(StepRecordError::UnsupportedDomain);
    }
    if source.conditions().len() > MAX_CONDITIONS {
        return Err(StepRecordError::ResourceLimit);
    }
    budget.expression(source.expression())?;
    for condition in source.conditions() {
        budget.expression(condition.expression())?;
    }
    let assumptions = Assumptions::from_conditions(source.conditions().iter().cloned())
        .map_err(|_| StepRecordError::InvalidDomain)?;
    let report = check_domain_preconditions(source, domain, &assumptions)
        .map_err(|_| StepRecordError::InvalidDomain)?;
    if matches!(
        report.status(),
        DomainPreconditionStatus::Blocked | DomainPreconditionStatus::Refuted
    ) {
        return Err(StepRecordError::InvalidDomain);
    }
    Ok(assumptions)
}

fn check_definedness(
    expression: &Expr,
    domain: ScalarDomain,
    assumptions: &Assumptions,
) -> RecordResult<()> {
    let required = definedness_obligations_in_domain(expression, MAX_TREE_NODES, domain)
        .map_err(|_| StepRecordError::InvalidDomain)?;
    if required.len() > MAX_CONDITIONS {
        return Err(StepRecordError::ResourceLimit);
    }
    if required.iter().any(|condition| {
        !matches!(
            assumptions.assess(condition),
            ConditionStatus::Proven | ConditionStatus::Assumed
        )
    }) {
        return Err(StepRecordError::InvalidDomain);
    }
    Ok(())
}

fn source_conditions(source: &DomainExpression) -> Vec<StepCondition> {
    (0..source.conditions().len())
        .map(StepCondition::Source)
        .collect()
}

fn previous(index: usize) -> Vec<usize> {
    index.checked_sub(1).into_iter().collect()
}

fn distribution(
    source: &DomainExpression,
    domain: ScalarDomain,
    result: &DistributionResult,
    budget: &mut Budget,
) -> RecordResult<StepRecords> {
    budget.check_steps(result.steps().len())?;
    let assumptions = source_assumptions(source, domain, budget)?;
    for step in result.steps() {
        let size = budget.expression(step.before())?;
        budget.work.charge(size.saturating_pow(3))?;
        budget.expression(step.after())?;
        check_definedness(step.before(), domain, &assumptions)?;
        check_definedness(step.after(), domain, &assumptions)?;
    }
    let size = budget.expression(result.result())?;
    budget.work.charge(size.saturating_pow(3))?;
    check_definedness(result.result(), domain, &assumptions)?;
    verify_distribution(source.expression(), result).map_err(|_| StepRecordError::InvalidEvidence)?;
    let steps = result
        .steps()
        .iter()
        .enumerate()
        .map(|(index, step)| StepRecord {
            index,
            claim: StepClaim::Equal {
                before: step.before().clone(),
                after: step.after().clone(),
            },
            rule: StepRule::Distribution(step.rule()),
            premises: previous(index),
            conditions: source_conditions(source),
            evidence: StepEvidenceSource::Distribution(index),
        })
        .collect();
    Ok(StepRecords {
        domain,
        source_conditions: source.conditions().to_vec(),
        steps,
        conclusion: StepConclusion::Distribution {
            original: source.expression().clone(),
            result: result.result().clone(),
            complete: result.is_complete(),
        },
    })
}

fn linear(
    left: &DomainExpression,
    right: &DomainExpression,
    variable: &Symbol,
    result: &LinearSolveResult,
    budget: &mut Budget,
) -> RecordResult<StepRecords> {
    budget.check_steps(result.steps().len())?;
    if !left.conditions().is_empty() || !right.conditions().is_empty() {
        return Err(StepRecordError::UnsupportedDomain);
    }
    check_name(variable)?;
    budget.expression(left.expression())?;
    budget.expression(right.expression())?;
    if result.original().left() != left.expression()
        || result.original().right() != right.expression()
    {
        return Err(StepRecordError::SourceMismatch);
    }
    budget.equation(result.original(), variable)?;
    budget.polynomial(result.residual())?;
    for step in result.steps() {
        budget.equation(step.before(), variable)?;
        budget.equation(step.after(), variable)?;
        if let Some(VerifiedCondition::Nonzero(value)) = step.verified_condition() {
            budget.context.check_rational(value)?;
        }
    }
    if let LinearSolution::Unique(value) = result.solution() {
        budget.context.check_rational(value)?;
    }
    verify_linear_solution(result, variable).map_err(|_| StepRecordError::InvalidEvidence)?;
    let steps = result
        .steps()
        .iter()
        .enumerate()
        .map(|(index, step)| StepRecord {
            index,
            claim: StepClaim::Equivalent {
                before: step.before().clone(),
                after: step.after().clone(),
                variable: variable.clone(),
            },
            rule: StepRule::Equation(step.rule()),
            premises: previous(index),
            conditions: step
                .verified_condition()
                .cloned()
                .map(StepCondition::Proven)
                .into_iter()
                .collect(),
            evidence: StepEvidenceSource::Linear(index),
        })
        .collect();
    Ok(StepRecords {
        domain: ScalarDomain::Rational,
        source_conditions: Vec::new(),
        steps,
        conclusion: StepConclusion::Linear {
            original: result.original().clone(),
            variable: variable.clone(),
            solution: result.solution().clone(),
        },
    })
}

fn derivative(
    source: &DomainExpression,
    domain: ScalarDomain,
    variable: &Symbol,
    result: &SymbolicDerivative,
    budget: &mut Budget,
) -> RecordResult<StepRecords> {
    budget.check_steps(result.steps().len())?;
    let assumptions = source_assumptions(source, domain, budget)?;
    check_name(variable)?;
    if result.original() != source.expression()
        || result.variable() != variable
        || result.domain() != Some(domain)
        || result.conditions() != source.conditions()
    {
        return Err(StepRecordError::SourceMismatch);
    }
    budget.expression(result.derivative())?;
    for step in result.steps() {
        let size = budget.expression(step.before())?;
        budget.work.charge(size.saturating_pow(2))?;
        budget.expression(step.derivative())?;
        check_definedness(step.before(), domain, &assumptions)?;
        check_definedness(step.derivative(), domain, &assumptions)?;
    }
    verify_symbolic_derivative(result).map_err(|_| StepRecordError::InvalidEvidence)?;

    let mut roots = Vec::new();
    let mut steps = Vec::with_capacity(result.steps().len());
    for (index, step) in result.steps().iter().enumerate() {
        let arity = match step.before().view() {
            ExprView::Add(children) | ExprView::Mul(children) => children.len(),
            ExprView::Pow { .. } | ExprView::Function { .. } => 1,
            _ => 0,
        };
        let start = roots
            .len()
            .checked_sub(arity)
            .ok_or(StepRecordError::InvalidEvidence)?;
        let premises = roots.split_off(start);
        steps.push(StepRecord {
            index,
            claim: StepClaim::DerivativeOf {
                before: step.before().clone(),
                after: step.derivative().clone(),
                variable: variable.clone(),
            },
            rule: StepRule::Derivative(step.rule()),
            premises,
            conditions: source_conditions(source),
            evidence: StepEvidenceSource::Derivative(index),
        });
        roots.push(index);
    }
    if roots.len() != 1 {
        return Err(StepRecordError::InvalidEvidence);
    }
    Ok(StepRecords {
        domain,
        source_conditions: source.conditions().to_vec(),
        steps,
        conclusion: StepConclusion::Derivative {
            original: source.expression().clone(),
            variable: variable.clone(),
            result: result.derivative().clone(),
        },
    })
}

fn division_record(claim: StepClaim, rule: StepRule, evidence: StepEvidenceSource) -> StepRecords {
    StepRecords {
        domain: ScalarDomain::Rational,
        source_conditions: Vec::new(),
        steps: vec![StepRecord {
            index: 0,
            claim,
            rule,
            premises: Vec::new(),
            conditions: Vec::new(),
            evidence,
        }],
        conclusion: StepConclusion::FinalDivision,
    }
}

fn polynomial_division(
    dividend: &RationalPolynomial,
    divisor: &RationalPolynomial,
    quotient: &RationalPolynomial,
    remainder: &RationalPolynomial,
    budget: &mut Budget,
) -> RecordResult<StepRecords> {
    budget.check_steps(1)?;
    for polynomial in [dividend, divisor, quotient, remainder] {
        budget.polynomial(polynomial)?;
    }
    let degree = divisor
        .degree()
        .ok_or(StepRecordError::Evaluation(EvaluationError::DivisionByZero))?;
    if remainder.degree().is_some_and(|value| value >= degree) {
        return Err(StepRecordError::InvalidEvidence);
    }
    let product = budget.work.multiply(divisor, quotient)?;
    if &budget.work.add(&product, remainder)? != dividend {
        return Err(StepRecordError::InvalidEvidence);
    }
    Ok(division_record(
        StepClaim::PolynomialDivision {
            dividend: dividend.clone(),
            divisor: divisor.clone(),
            quotient: quotient.clone(),
            remainder: remainder.clone(),
        },
        StepRule::PolynomialDivision,
        StepEvidenceSource::PolynomialDivisionFinal,
    ))
}

fn multivariate_division(
    dividend: &MultivariatePolynomial,
    divisors: &[MultivariatePolynomial],
    result: &MultivariateDivision,
    budget: &mut Budget,
) -> RecordResult<StepRecords> {
    budget.check_steps(1)?;
    if divisors.len() > 8 {
        return Err(StepRecordError::ResourceLimit);
    }
    if result.quotients().len() != divisors.len() {
        return Err(StepRecordError::InvalidEvidence);
    }
    budget.multivariate(dividend)?;
    budget.multivariate(result.remainder())?;
    for polynomial in divisors.iter().chain(result.quotients()) {
        budget.multivariate(polynomial)?;
    }
    let limits = MultivariateLimits {
        max_variables: 8,
        max_terms: 64,
        max_input_terms: 4_096,
        max_total_degree: 32,
        max_scalar_bits: 4_096,
        max_work_units: budget.context.limits().max_work_units.min(250_000),
    };
    for (divisor, quotient) in divisors.iter().zip(result.quotients()) {
        let terms = divisor.terms().len() * quotient.terms().len();
        budget.work.charge(8 * (terms as u64 + 64))?;
    }
    if !verify_multivariate_division(dividend, divisors, result, limits)? {
        return Err(StepRecordError::InvalidEvidence);
    }
    Ok(division_record(
        StepClaim::MultivariateDivision {
            dividend: dividend.clone(),
            ordered_divisors: divisors.to_vec(),
            quotients: result.quotients().to_vec(),
            remainder: result.remainder().clone(),
        },
        StepRule::MultivariateDivision,
        StepEvidenceSource::MultivariateDivisionFinal,
    ))
}

/// Checks original evidence and returns its bounded, deterministic traversal.
/// Division certificates expose final claims only, not a reduction history.
pub fn record_steps(
    evidence: StepEvidence<'_>,
    limits: StepLimits,
) -> Result<StepRecords, StepRecordError> {
    let mut budget = Budget::new(limits);
    match evidence {
        StepEvidence::Distribution {
            source,
            domain,
            result,
        } => distribution(source, domain, result, &mut budget),
        StepEvidence::Linear {
            left,
            right,
            variable,
            result,
        } => linear(left, right, variable, result, &mut budget),
        StepEvidence::Derivative {
            source,
            domain,
            variable,
            result,
        } => derivative(source, domain, variable, result, &mut budget),
        StepEvidence::PolynomialDivision {
            dividend,
            divisor,
            quotient,
            remainder,
        } => polynomial_division(dividend, divisor, quotient, remainder, &mut budget),
        StepEvidence::MultivariateDivision {
            dividend,
            divisors,
            result,
        } => multivariate_division(dividend, divisors, result, &mut budget),
    }
}

/// Rechecks the mathematical certificate with its independent verifier and
/// compares every structural field, including order and premise references.
pub fn verify_step_records(
    evidence: StepEvidence<'_>,
    claimed: &StepRecords,
    limits: StepLimits,
) -> Result<(), StepRecordError> {
    if claimed.steps.len() > limits.max_steps.min(MAX_STEPS)
        || claimed.source_conditions.len() > MAX_CONDITIONS
    {
        return Err(StepRecordError::ResourceLimit);
    }
    let expected = record_steps(evidence, limits)?;
    if &expected == claimed {
        Ok(())
    } else {
        Err(StepRecordError::InvalidRecord)
    }
}
