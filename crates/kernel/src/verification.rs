//! Independent, rule-specific checks for the first exact derivations.
//!
//! These checks reconstruct permitted transformations; they never call the
//! derivation generators. This is not a universal proof calculus.

use std::error::Error;
use std::fmt;

use mathema::{Equation, Expr, ExprView, Integer, Rational, Symbol};

use crate::poly_division::rational_quotient;
use crate::{
    DerivationStep, DistributionResult, EquationRuleId, EquationStep, LinearSolution,
    LinearSolveResult, RationalPolynomial, Relation, RuleId, VerifiedCondition,
};

/// A derivation fails an explicit mathematical or structural obligation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationError {
    InvalidStep,
    BrokenChain,
    IncorrectResult,
    IncorrectCompleteness,
    UnsupportedDomain,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidStep => "invalid claimed mathematical step",
            Self::BrokenChain => "derivation contains disconnected steps",
            Self::IncorrectResult => "result or solution does not match the derivation",
            Self::IncorrectCompleteness => "distribution completion flag is incorrect",
            Self::UnsupportedDomain => "verification input is outside exact rational-linear scope",
        };
        f.write_str(message)
    }
}

impl Error for VerificationError {}

/// Enumerates legal *single* applications of the distributive identity.
///
/// Unlike the generator, this enumerates all eligible factors and all
/// reachable Add/Mul contexts. No step is allowed inside a power. The
/// verifier independently reconstructs the algebraic consequence of the
/// identity; simply attaching a recognized rule ID never proves validity.
fn distribution_candidates(expr: &Expr) -> Vec<Expr> {
    let mut results = Vec::new();
    match expr.view() {
        ExprView::Add(children) => {
            for (index, child) in children.iter().enumerate() {
                for replacement in distribution_candidates(child) {
                    results.push(Expr::add(children.iter().enumerate().map(|(i, term)| {
                        if i == index {
                            replacement.clone()
                        } else {
                            term.clone()
                        }
                    })));
                }
            }
        }
        ExprView::Mul(factors) => {
            for (index, factor) in factors.iter().enumerate() {
                if let ExprView::Add(terms) = factor.view() {
                    let other: Vec<_> = factors
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != index)
                        .map(|(_, term)| term.clone())
                        .collect();
                    let expanded = Expr::add(terms.iter().map(|term| {
                        Expr::mul(other.iter().cloned().chain(std::iter::once(term.clone())))
                    }));
                    results.push(expanded);
                }
                for replacement in distribution_candidates(factor) {
                    results.push(Expr::mul(factors.iter().enumerate().map(|(i, term)| {
                        if i == index {
                            replacement.clone()
                        } else {
                            term.clone()
                        }
                    })));
                }
            }
        }
        ExprView::Integer(_)
        | ExprView::Rational(_)
        | ExprView::Symbol(_)
        | ExprView::Pow { .. } => {}
    }
    results
}

/// Verifies a claimed single distributive step without calling its generator.
pub fn verify_distribution_step(step: &DerivationStep) -> Result<(), VerificationError> {
    if step.rule() != RuleId::DistributeProductOverSum || step.relation() != Relation::Equal {
        return Err(VerificationError::InvalidStep);
    }
    if distribution_candidates(step.before())
        .iter()
        .any(|candidate| candidate == step.after())
    {
        Ok(())
    } else {
        Err(VerificationError::InvalidStep)
    }
}

/// Verifies the complete recorded chain and the narrow completion guarantee.
///
/// An empty chain is legitimate only if its result is the original input.
/// Completion refers only to the supported Add/Mul distribution traversal.
pub fn verify_distribution(
    original: &Expr,
    derivation: &DistributionResult,
) -> Result<(), VerificationError> {
    let mut current = original;
    for step in derivation.steps() {
        if step.before() != current {
            return Err(VerificationError::BrokenChain);
        }
        verify_distribution_step(step)?;
        current = step.after();
    }
    if current != derivation.result() {
        return Err(VerificationError::IncorrectResult);
    }
    let complete = distribution_candidates(current).is_empty();
    if derivation.is_complete() != complete {
        return Err(VerificationError::IncorrectCompleteness);
    }
    Ok(())
}

fn polynomial_of(expr: &Expr, variable: &Symbol) -> Result<RationalPolynomial, VerificationError> {
    RationalPolynomial::try_from_expr(expr, variable, 32)
        .map_err(|_| VerificationError::UnsupportedDomain)
}

fn residual(
    equation: &Equation,
    variable: &Symbol,
) -> Result<RationalPolynomial, VerificationError> {
    let lhs = polynomial_of(equation.left(), variable)?;
    let rhs = polynomial_of(equation.right(), variable)?;
    let difference = lhs.subtract(&rhs);
    if difference.degree().is_some_and(|degree| degree > 1) {
        return Err(VerificationError::UnsupportedDomain);
    }
    Ok(difference)
}

fn classification(polynomial: &RationalPolynomial) -> LinearSolution {
    let zero = Rational::from(Integer::zero());
    let constant = polynomial.coefficients().first().unwrap_or(&zero);
    let coefficient = polynomial.coefficients().get(1).unwrap_or(&zero);
    if coefficient.numerator().is_zero() {
        if constant.numerator().is_zero() {
            LinearSolution::AllRationals
        } else {
            LinearSolution::NoSolution
        }
    } else {
        LinearSolution::Unique(rational_quotient(&(-constant), coefficient))
    }
}

/// Independently checks a single equation equivalence and its exact condition.
///
/// The three supported rules have specific permitted shapes, in addition to
/// the requirement that their rational solution sets match. Unsupported or
/// nonlinear expressions are rejected, not sampled numerically.
pub fn verify_equation_step(
    step: &EquationStep,
    variable: &Symbol,
) -> Result<(), VerificationError> {
    if step.relation() != Relation::Equivalent {
        return Err(VerificationError::InvalidStep);
    }
    let before = residual(step.before(), variable)?;
    let after = residual(step.after(), variable)?;
    if classification(&before) != classification(&after) {
        return Err(VerificationError::InvalidStep);
    }
    let zero = Rational::from(Integer::zero());
    let constant = before.coefficients().first().unwrap_or(&zero);
    let coefficient = before.coefficients().get(1).unwrap_or(&zero);
    let expected = match step.rule() {
        EquationRuleId::CollectToZero => {
            if step.verified_condition().is_some() {
                return Err(VerificationError::InvalidStep);
            }
            Equation::new(before.to_expr(variable), Expr::from(0))
        }
        EquationRuleId::IsolateLinearTerm => {
            if step.verified_condition().is_some()
                || coefficient.numerator().is_zero()
                || step.before().right() != &Expr::from(0)
                || step.before().left() != &before.to_expr(variable)
            {
                return Err(VerificationError::InvalidStep);
            }
            Equation::new(
                Expr::mul([
                    Expr::from(coefficient.clone()),
                    Expr::from(variable.clone()),
                ]),
                Expr::from(-constant),
            )
        }
        EquationRuleId::DivideByNonzeroCoefficient => {
            let lhs = polynomial_of(step.before().left(), variable)?;
            let rhs = polynomial_of(step.before().right(), variable)?;
            let a = lhs.coefficients().get(1).unwrap_or(&zero);
            let b = rhs.coefficients().first().unwrap_or(&zero);
            if a.numerator().is_zero()
                || lhs.coefficients().first().is_some_and(|c| !c.numerator().is_zero())
                || rhs.degree().is_some_and(|degree| degree != 0)
                || step.before().left()
                    != &Expr::mul([Expr::from(a.clone()), Expr::from(variable.clone())])
                || step.before().right() != &Expr::from(b.clone())
                || step.verified_condition() != Some(&VerifiedCondition::Nonzero(a.clone()))
            {
                return Err(VerificationError::InvalidStep);
            }
            Equation::new(
                Expr::from(variable.clone()),
                Expr::from(rational_quotient(b, a)),
            )
        }
    };
    if step.after() == &expected {
        Ok(())
    } else {
        Err(VerificationError::InvalidStep)
    }
}

/// Verifies a linear solver result, its chain, and the exact solution class.
///
/// It independently derives the residual from the source equation, checks
/// each rule and side condition, and compares solution classification. This
/// does not certify arbitrary user-defined rules or non-polynomial domains.
pub fn verify_linear_solution(
    result: &LinearSolveResult,
    variable: &Symbol,
) -> Result<(), VerificationError> {
    let expected_residual = residual(result.original(), variable)?;
    if &expected_residual != result.residual()
        || &classification(&expected_residual) != result.solution()
    {
        return Err(VerificationError::IncorrectResult);
    }
    let mut current = result.original();
    for step in result.steps() {
        if step.before() != current {
            return Err(VerificationError::BrokenChain);
        }
        verify_equation_step(step, variable)?;
        current = step.after();
    }
    if classification(&residual(current, variable)?) != *result.solution() {
        return Err(VerificationError::IncorrectResult);
    }
    if let LinearSolution::Unique(root) = result.solution() {
        let expected = Equation::new(Expr::from(variable.clone()), Expr::from(root.clone()));
        if current != &expected {
            return Err(VerificationError::IncorrectResult);
        }
    }
    Ok(())
}
