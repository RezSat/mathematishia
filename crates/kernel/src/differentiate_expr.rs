//! First exact symbolic differentiation rules over the immutable Expr IR.
//!
//! A derivative step is NOT an equality: it asserts a derivative relationship
//! with respect to one selected symbol. We support commutative sums, products,
//! and strictly positive integer powers, with no undefined negative-power
//! domains, function calls, implicit assumptions or general simplifier.

use std::error::Error;
use std::fmt;

use mathema::{Expr, ExprView, Integer, Symbol};

use crate::budget::WorkBudget;
use crate::{EvaluationError, Relation, ResourceLimitKind};

/// Stable mathematical differentiation rule identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DerivativeRuleId {
    Constant,
    SelectedSymbol,
    OtherSymbol,
    Sum,
    Product,
    PositiveIntegerPower,
}

impl DerivativeRuleId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Constant => "calculus.constant",
            Self::SelectedSymbol => "calculus.selected_symbol",
            Self::OtherSymbol => "calculus.other_symbol",
            Self::Sum => "calculus.sum",
            Self::Product => "calculus.product",
            Self::PositiveIntegerPower => "calculus.positive_integer_power",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolicDifferentiationError {
    /// Zero, negative, fractional and symbolic exponents are unsupported.
    UnsupportedPower,
}

impl fmt::Display for SymbolicDifferentiationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPower => {
                f.write_str("symbolic differentiation requires positive integer exponents")
            }
        }
    }
}
impl Error for SymbolicDifferentiationError {}

/// An assertion that the second Expr is the derivative of the first Expr.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DifferentiationStep {
    before: Expr,
    derivative: Expr,
    rule: DerivativeRuleId,
    relation: Relation,
}

impl DifferentiationStep {
    pub fn before(&self) -> &Expr {
        &self.before
    }

    pub fn derivative(&self) -> &Expr {
        &self.derivative
    }

    pub const fn rule(&self) -> DerivativeRuleId {
        self.rule
    }

    pub const fn relation(&self) -> Relation {
        self.relation
    }
}

/// Postorder rule record and checked derivative of an immutable expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolicDerivative {
    original: Expr,
    variable: Symbol,
    derivative: Expr,
    steps: Vec<DifferentiationStep>,
}

impl SymbolicDerivative {
    pub fn original(&self) -> &Expr {
        &self.original
    }

    pub fn variable(&self) -> &Symbol {
        &self.variable
    }

    pub fn derivative(&self) -> &Expr {
        &self.derivative
    }

    pub fn steps(&self) -> &[DifferentiationStep] {
        &self.steps
    }
}

fn positive_power(exponent: &Expr) -> Result<usize, SymbolicDifferentiationError> {
    let ExprView::Integer(value) = exponent.view() else {
        return Err(SymbolicDifferentiationError::UnsupportedPower);
    };
    if value <= &Integer::zero() {
        return Err(SymbolicDifferentiationError::UnsupportedPower);
    }
    value
        .to_string()
        .parse::<usize>()
        .map_err(|_| SymbolicDifferentiationError::UnsupportedPower)
}

fn produce(
    original: &Expr,
    selected: &Symbol,
    power_limit: usize,
    meter: &mut WorkBudget,
    steps: &mut Vec<DifferentiationStep>,
    check: &mut impl FnMut(&Expr) -> Result<(), EvaluationError>,
) -> Result<Expr, EvaluationError> {
    meter.charge(1)?;
    let (rule, derivative) = match original.view() {
        ExprView::Integer(_) | ExprView::Rational(_) => {
            (DerivativeRuleId::Constant, Expr::from(0))
        }
        ExprView::Symbol(name) if name == selected => {
            (DerivativeRuleId::SelectedSymbol, Expr::from(1))
        }
        ExprView::Symbol(_) => (DerivativeRuleId::OtherSymbol, Expr::from(0)),
        ExprView::Add(terms) => {
            let mut derivatives = Vec::with_capacity(terms.len());
            for term in terms {
                derivatives.push(produce(term, selected, power_limit, meter, steps, check)?);
            }
            meter.charge(terms.len() as u64)?;
            (DerivativeRuleId::Sum, Expr::add(derivatives))
        }
        ExprView::Mul(factors) => {
            // A flat n-factor product rule has O(n^2) structural work.
            let n = factors.len() as u64;
            meter.charge(n.saturating_mul(n))?;
            let mut derivatives = Vec::with_capacity(factors.len());
            for factor in factors {
                derivatives.push(produce(factor, selected, power_limit, meter, steps, check)?);
            }
            let terms = factors.iter().enumerate().map(|(index, _)| {
                Expr::mul(factors.iter().enumerate().map(|(other, factor)| {
                    if index == other {
                        derivatives[index].clone()
                    } else {
                        factor.clone()
                    }
                }))
            });
            (DerivativeRuleId::Product, Expr::add(terms))
        }
        ExprView::Pow { base, exponent } => {
            let power = positive_power(exponent)?;
            if power > power_limit {
                return Err(EvaluationError::Limit(ResourceLimitKind::SymbolicPower));
            }
            let inner = produce(base, selected, power_limit, meter, steps, check)?;
            meter.charge(3)?;
            let reduced = if power == 1 {
                Expr::from(1)
            } else {
                Expr::pow(base.clone(), Expr::from(power - 1))
            };
            (
                DerivativeRuleId::PositiveIntegerPower,
                Expr::mul([Expr::from(power), reduced, inner]),
            )
        }
    };
    check(&derivative)?;
    steps.push(DifferentiationStep {
        before: original.clone(),
        derivative: derivative.clone(),
        rule,
        relation: Relation::DerivativeOf,
    });
    Ok(derivative)
}

/// Generates typed, deterministic postorder derivative steps with a work meter.
pub(crate) fn generate_symbolic_derivative(
    original: &Expr,
    variable: &Symbol,
    power_limit: usize,
    meter: &mut WorkBudget,
    mut check: impl FnMut(&Expr) -> Result<(), EvaluationError>,
) -> Result<SymbolicDerivative, EvaluationError> {
    let mut steps = Vec::new();
    let derivative = produce(
        original,
        variable,
        power_limit,
        meter,
        &mut steps,
        &mut check,
    )?;
    Ok(SymbolicDerivative {
        original: original.clone(),
        variable: variable.clone(),
        derivative,
        steps,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DerivativeVerificationError {
    MissingStep,
    UnexpectedStep,
    IncorrectRule,
    IncorrectDerivative,
    UnsupportedPower,
    DepthLimit,
}

impl fmt::Display for DerivativeVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingStep => "missing derivative step",
            Self::UnexpectedStep => "unexpected derivative step or source",
            Self::IncorrectRule => "wrong differentiation rule or relation",
            Self::IncorrectDerivative => "claimed derivative is not justified",
            Self::UnsupportedPower => "unsupported derivative power in claimed proof",
            Self::DepthLimit => "derivative verification nesting limit exceeded",
        };
        f.write_str(message)
    }
}
impl Error for DerivativeVerificationError {}

/// Independent rule-specific reconstruction; does not call the generator.
///
/// Verifies each subtree in postorder, checks the relation and rule ID, and
/// requires all reported steps to be consumed exactly once. Every result is
/// built from the original children and independently verified derivatives.
fn verify_node<'a>(
    expr: &Expr,
    variable: &Symbol,
    steps: &mut impl Iterator<Item = &'a DifferentiationStep>,
    depth: usize,
) -> Result<Expr, DerivativeVerificationError> {
    use DerivativeVerificationError as E;

    if depth > 256 {
        return Err(E::DepthLimit);
    }
    let (rule, expected) = match expr.view() {
        ExprView::Integer(_) | ExprView::Rational(_) => {
            (DerivativeRuleId::Constant, Expr::from(0))
        }
        ExprView::Symbol(name) if name == variable => {
            (DerivativeRuleId::SelectedSymbol, Expr::from(1))
        }
        ExprView::Symbol(_) => (DerivativeRuleId::OtherSymbol, Expr::from(0)),
        ExprView::Add(terms) => {
            let mut expected_terms = Vec::with_capacity(terms.len());
            for term in terms {
                expected_terms.push(verify_node(term, variable, steps, depth + 1)?);
            }
            (DerivativeRuleId::Sum, Expr::add(expected_terms))
        }
        ExprView::Mul(factors) => {
            let mut gradients = Vec::with_capacity(factors.len());
            for factor in factors {
                gradients.push(verify_node(factor, variable, steps, depth + 1)?);
            }
            let mut sum = Vec::with_capacity(factors.len());
            for i in 0..factors.len() {
                let mut term = Vec::with_capacity(factors.len());
                for (j, factor) in factors.iter().enumerate() {
                    term.push(if i == j {
                        gradients[j].clone()
                    } else {
                        factor.clone()
                    });
                }
                sum.push(Expr::mul(term));
            }
            (DerivativeRuleId::Product, Expr::add(sum))
        }
        ExprView::Pow { base, exponent } => {
            let exponent = positive_power(exponent).map_err(|_| E::UnsupportedPower)?;
            let derivative = verify_node(base, variable, steps, depth + 1)?;
            let power = if exponent == 1 {
                Expr::from(1)
            } else {
                Expr::pow(base.clone(), Expr::from(exponent - 1))
            };
            (
                DerivativeRuleId::PositiveIntegerPower,
                Expr::mul([Expr::from(exponent), power, derivative]),
            )
        }
    };
    let step = steps.next().ok_or(E::MissingStep)?;
    if step.before != *expr {
        return Err(E::UnexpectedStep);
    }
    if step.relation != Relation::DerivativeOf || step.rule != rule {
        return Err(E::IncorrectRule);
    }
    if step.derivative != expected {
        return Err(E::IncorrectDerivative);
    }
    Ok(expected)
}

/// Independent exact rule verification; never compares sampled float values.
pub fn verify_symbolic_derivative(
    result: &SymbolicDerivative,
) -> Result<(), DerivativeVerificationError> {
    use DerivativeVerificationError as E;

    if result.steps.len() > 100_000 {
        return Err(E::UnexpectedStep);
    }
    let mut steps = result.steps.iter();
    let verified = verify_node(&result.original, &result.variable, &mut steps, 1)?;
    if steps.next().is_some() {
        return Err(E::UnexpectedStep);
    }
    if verified != result.derivative {
        return Err(E::IncorrectDerivative);
    }
    Ok(())
}

#[cfg(test)]
mod adversarial {
    use super::*;
    use crate::EvaluationContext;

    fn example() -> SymbolicDerivative {
        let x = Symbol::new("x").unwrap();
        let y = Expr::from(Symbol::new("y").unwrap());
        let expr = Expr::mul([Expr::from(x.clone()), y]);
        EvaluationContext::default()
            .differentiate_expression(&expr, &x)
            .unwrap()
    }

    #[test]
    fn rejects_forged_derivative_and_wrong_rule() {
        let mut claimed = example();
        claimed.derivative = Expr::from(99);
        assert_eq!(
            verify_symbolic_derivative(&claimed),
            Err(DerivativeVerificationError::IncorrectDerivative)
        );
        let mut claimed = example();
        claimed.steps[0].rule = DerivativeRuleId::Constant;
        assert_eq!(
            verify_symbolic_derivative(&claimed),
            Err(DerivativeVerificationError::IncorrectRule)
        );
    }

    #[test]
    fn rejects_wrong_relation_disconnected_and_extra_steps() {
        let mut claimed = example();
        claimed.steps[0].relation = Relation::Equal;
        assert_eq!(
            verify_symbolic_derivative(&claimed),
            Err(DerivativeVerificationError::IncorrectRule)
        );

        let mut claimed = example();
        claimed.steps.swap(0, 1);
        assert_eq!(
            verify_symbolic_derivative(&claimed),
            Err(DerivativeVerificationError::UnexpectedStep)
        );

        let mut claimed = example();
        let extra = claimed.steps[0].clone();
        claimed.steps.push(extra);
        assert_eq!(
            verify_symbolic_derivative(&claimed),
            Err(DerivativeVerificationError::UnexpectedStep)
        );

        let mut claimed = example();
        claimed.steps.pop();
        assert_eq!(
            verify_symbolic_derivative(&claimed),
            Err(DerivativeVerificationError::MissingStep)
        );
    }
}
