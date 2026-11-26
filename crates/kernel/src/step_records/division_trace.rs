//! Executed and independently checked ordered division over native Q-polynomials.
//!
//! These inputs have no expression-domain exclusions. This API does not admit
//! parsed expressions or certify a unique normal form for arbitrary divisors.

use mathema::Rational;

use super::{Budget, RecordResult, StepLimits, StepRecordError};
use crate::{
    MultivariateDivisionError, MultivariateError, MultivariateLimits, MultivariatePolynomial,
};

type Polynomial = MultivariatePolynomial;

const LIMITS: MultivariateLimits = MultivariateLimits {
    max_variables: 8,
    max_terms: 64,
    max_input_terms: 4096,
    max_total_degree: 32,
    max_scalar_bits: 256,
    max_work_units: 250_000,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivisionReductionRule {
    Reduce { divisor_index: usize },
    MoveToRemainder,
}

impl DivisionReductionRule {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Reduce { .. } => "polynomial.reduce_leading_term",
            Self::MoveToRemainder => "polynomial.move_leading_term_to_remainder",
        }
    }
}

/// A pending-polynomial transition, not an equality of before and after alone.
/// For reduction, before = after + factor * divisor; for a move, before =
/// after + factor. The factor is also added to the corresponding accumulator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivisionReduction {
    pub index: usize,
    pub premise: Option<usize>,
    pub rule: DivisionReductionRule,
    pub before: Polynomial,
    pub after: Polynomial,
    pub factor: Polynomial,
}

/// Source-bound, output-only trace. Public fields must be verified before use.
/// Term order is lexicographic in the canonical ambient variable order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedDivisionTrace {
    pub dividend: Polynomial,
    pub ordered_divisors: Vec<Polynomial>,
    pub steps: Vec<DivisionReduction>,
    pub quotients: Vec<Polynomial>,
    pub remainder: Polynomial,
}

fn checked<T>(result: Result<T, MultivariateError>) -> RecordResult<T> {
    result.map_err(|error| MultivariateDivisionError::from(error).into())
}

fn admit(dividend: &Polynomial, divisors: &[Polynomial], budget: &mut Budget) -> RecordResult<()> {
    if divisors.len() > 8 {
        return Err(StepRecordError::ResourceLimit);
    }
    if divisors.is_empty() {
        return Err(StepRecordError::InvalidEvidence);
    }
    budget.multivariate(dividend)?;
    for divisor in divisors {
        budget.multivariate(divisor)?;
        if divisor.is_zero() || divisor.variables() != dividend.variables() {
            return Err(StepRecordError::InvalidEvidence);
        }
    }
    Ok(())
}

fn divides(divisor: &[u32], term: &[u32]) -> bool {
    divisor.iter().zip(term).all(|(a, b)| a <= b)
}

fn monomial(
    source: &Polynomial,
    powers: &[u32],
    coefficient: Rational,
) -> RecordResult<Polynomial> {
    let powers = powers
        .iter()
        .map(|power| i32::try_from(*power).map_err(|_| StepRecordError::ResourceLimit))
        .collect::<RecordResult<Vec<_>>>()?;
    checked(Polynomial::try_new(
        source.variables().to_vec(),
        [(powers, coefficient)],
        LIMITS,
    ))
}

struct Accumulators {
    quotients: Vec<Polynomial>,
    remainder: Polynomial,
}

impl Accumulators {
    fn new(dividend: &Polynomial, count: usize) -> RecordResult<Self> {
        let zero = checked(Polynomial::zero(dividend.variables().to_vec(), LIMITS))?;
        Ok(Self {
            quotients: vec![zero.clone(); count],
            remainder: zero,
        })
    }

    fn add(&mut self, rule: DivisionReductionRule, factor: &Polynomial) -> RecordResult<()> {
        let target = match rule {
            DivisionReductionRule::Reduce { divisor_index } => &mut self.quotients[divisor_index],
            DivisionReductionRule::MoveToRemainder => &mut self.remainder,
        };
        *target = checked(target.add(factor, LIMITS))?;
        Ok(())
    }
}

fn admit_step(step: &DivisionReduction, budget: &mut Budget) -> RecordResult<()> {
    // A conservative aggregate charge covers the bounded sparse operations,
    // divisor search and term comparisons in one transition.
    budget.work.charge(2048)?;
    for polynomial in [&step.before, &step.after, &step.factor] {
        budget.multivariate(polynomial)?;
    }
    Ok(())
}

/// Execute the deterministic division procedure and record transitions at the
/// point of execution. A resource refusal never returns a partial certificate.
/// Existing final-only evidence is not converted into a historical trace.
pub fn record_ordered_division(
    dividend: &Polynomial,
    divisors: &[Polynomial],
    limits: StepLimits,
) -> RecordResult<OrderedDivisionTrace> {
    let mut budget = Budget::new(limits);
    admit(dividend, divisors, &mut budget)?;
    let mut accumulators = Accumulators::new(dividend, divisors.len())?;
    let mut pending = dividend.clone();
    let mut steps = Vec::new();
    while let Some((powers, coefficient)) = pending.terms().last_key_value() {
        let index = steps.len();
        budget.check_steps(index + 1)?;
        budget.work.charge(2048)?;
        let mut selected = None;
        for (divisor_index, divisor) in divisors.iter().enumerate() {
            let (leading, scalar) = divisor.terms().last_key_value().expect("nonzero divisor");
            if divides(leading, powers) {
                let difference: Vec<_> = powers.iter().zip(leading).map(|(a, b)| a - b).collect();
                let ratio = budget.work.quotient(coefficient, scalar)?;
                let factor = monomial(dividend, &difference, ratio)?;
                selected = Some((divisor_index, factor));
                break;
            }
        }
        let (rule, factor, removed) = if let Some((divisor_index, factor)) = selected {
            let product = checked(factor.multiply(&divisors[divisor_index], LIMITS))?;
            (
                DivisionReductionRule::Reduce { divisor_index },
                factor,
                product,
            )
        } else {
            let factor = monomial(dividend, powers, coefficient.clone())?;
            (
                DivisionReductionRule::MoveToRemainder,
                factor.clone(),
                factor,
            )
        };
        let after = checked(pending.subtract(&removed, LIMITS))?;
        accumulators.add(rule, &factor)?;
        let step = DivisionReduction {
            index,
            premise: index.checked_sub(1),
            rule,
            before: pending,
            after: after.clone(),
            factor,
        };
        admit_step(&step, &mut budget)?;
        steps.push(step);
        pending = after;
    }

    let trace = OrderedDivisionTrace {
        dividend: dividend.clone(),
        ordered_divisors: divisors.to_vec(),
        steps,
        quotients: accumulators.quotients,
        remainder: accumulators.remainder,
    };
    verify_ordered_division(dividend, divisors, &trace, limits)?;
    Ok(trace)
}

/// Check the supplied history against the original native polynomial inputs.
/// Does not invoke the producer or trust a reported iteration count.
pub fn verify_ordered_division(
    dividend: &Polynomial,
    divisors: &[Polynomial],
    trace: &OrderedDivisionTrace,
    limits: StepLimits,
) -> RecordResult<()> {
    let mut budget = Budget::new(limits);
    admit(dividend, divisors, &mut budget)?;
    admit(&trace.dividend, &trace.ordered_divisors, &mut budget)?;
    budget.check_steps(trace.steps.len())?;
    if &trace.dividend != dividend
        || trace.ordered_divisors != divisors
        || trace.quotients.len() != divisors.len()
    {
        return Err(StepRecordError::SourceMismatch);
    }
    budget.multivariate(&trace.remainder)?;
    for quotient in &trace.quotients {
        budget.multivariate(quotient)?;
    }

    let mut accumulators = Accumulators::new(dividend, divisors.len())?;
    let mut pending = dividend;
    for (index, step) in trace.steps.iter().enumerate() {
        admit_step(step, &mut budget)?;
        if step.index != index
            || step.premise != index.checked_sub(1)
            || &step.before != pending
            || step.factor.variables() != dividend.variables()
            || step.after.variables() != dividend.variables()
            || step.factor.terms().len() != 1
        {
            return Err(StepRecordError::InvalidRecord);
        }
        let leading = pending
            .terms()
            .last_key_value()
            .ok_or(StepRecordError::InvalidRecord)?;
        let priority = divisors.iter().position(|divisor| {
            let powers = divisor.terms().last_key_value().expect("nonzero divisor").0;
            divides(powers, leading.0)
        });

        let removed = match (step.rule, priority) {
            (DivisionReductionRule::Reduce { divisor_index }, Some(first))
                if divisor_index == first =>
            {
                checked(step.factor.multiply(&divisors[first], LIMITS))?
            }
            (DivisionReductionRule::MoveToRemainder, None) => step.factor.clone(),
            _ => return Err(StepRecordError::InvalidRecord),
        };
        if removed.terms().last_key_value() != Some(leading)
            || step
                .after
                .terms()
                .last_key_value()
                .is_some_and(|(powers, _)| powers >= leading.0)
            || checked(step.after.add(&removed, LIMITS))? != *pending
        {
            return Err(StepRecordError::InvalidRecord);
        }
        accumulators.add(step.rule, &step.factor)?;
        pending = &step.after;
    }

    if !pending.is_zero()
        || accumulators.quotients != trace.quotients
        || accumulators.remainder != trace.remainder
    {
        return Err(StepRecordError::InvalidRecord);
    }

    // Separately verify the final identity and reduced-remainder property.
    for powers in trace.remainder.terms().keys() {
        if divisors.iter().any(|divisor| {
            divides(
                divisor.terms().last_key_value().expect("nonzero divisor").0,
                powers,
            )
        }) {
            return Err(StepRecordError::InvalidEvidence);
        }
    }
    let mut reconstructed = trace.remainder.clone();
    for (quotient, divisor) in trace.quotients.iter().zip(divisors) {
        budget.work.charge(32_768)?;
        let product = checked(quotient.multiply(divisor, LIMITS))?;
        reconstructed = checked(reconstructed.add(&product, LIMITS))?;
    }
    if &reconstructed != dividend {
        return Err(StepRecordError::InvalidEvidence);
    }
    Ok(())
}
