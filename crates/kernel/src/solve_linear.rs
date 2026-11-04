use std::error::Error;
use std::fmt;

use mathema::{Equation, Expr, Integer, Rational, Symbol};

use crate::poly_division::rational_quotient;
use crate::{PolynomialConversionError, RationalPolynomial, Relation};

/// Errors for the deliberately bounded exact univariate linear solver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinearSolveError {
    /// The input cannot be converted to an exact polynomial under the budget.
    UnsupportedExpression(PolynomialConversionError),
    /// A successfully converted residual has degree greater than one.
    NonlinearEquation,
}

impl fmt::Display for LinearSolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedExpression(error) => write!(f, "{error}"),
            Self::NonlinearEquation => f.write_str("equation is not linear in the chosen variable"),
        }
    }
}

impl Error for LinearSolveError {}

impl From<PolynomialConversionError> for LinearSolveError {
    fn from(value: PolynomialConversionError) -> Self {
        Self::UnsupportedExpression(value)
    }
}

/// Exhaustive solution classification for linear equations over Q.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinearSolution {
    Unique(Rational),
    NoSolution,
    AllRationals,
}

/// Stable identifiers for equivalence-preserving equation operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquationRuleId {
    CollectToZero,
    IsolateLinearTerm,
    DivideByNonzeroCoefficient,
}

impl EquationRuleId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::CollectToZero => "equation.collect_to_zero",
            Self::IsolateLinearTerm => "equation.isolate_linear_term",
            Self::DivideByNonzeroCoefficient => "equation.divide_by_nonzero_coefficient",
        }
    }
}

/// A condition verified by exact arithmetic, rather than assumed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifiedCondition {
    Nonzero(Rational),
}

/// A structured step relating two equations with the same solution set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquationStep {
    before: Equation,
    after: Equation,
    rule: EquationRuleId,
    relation: Relation,
    condition: Option<VerifiedCondition>,
}

impl EquationStep {
    pub fn before(&self) -> &Equation {
        &self.before
    }

    pub fn after(&self) -> &Equation {
        &self.after
    }

    pub const fn rule(&self) -> EquationRuleId {
        self.rule
    }

    pub const fn relation(&self) -> Relation {
        self.relation
    }

    pub fn verified_condition(&self) -> Option<&VerifiedCondition> {
        self.condition.as_ref()
    }
}

/// An exact classification, residual polynomial, and chain of equivalent equations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinearSolveResult {
    original: Equation,
    residual: RationalPolynomial,
    solution: LinearSolution,
    steps: Vec<EquationStep>,
}

impl LinearSolveResult {
    pub fn original(&self) -> &Equation {
        &self.original
    }

    /// Returns the exact polynomial representing left minus right.
    pub fn residual(&self) -> &RationalPolynomial {
        &self.residual
    }

    pub fn solution(&self) -> &LinearSolution {
        &self.solution
    }

    pub fn steps(&self) -> &[EquationStep] {
        &self.steps
    }
}

fn record_step(
    steps: &mut Vec<EquationStep>,
    current: &mut Equation,
    next: Equation,
    rule: EquationRuleId,
    condition: Option<VerifiedCondition>,
) {
    if *current != next {
        steps.push(EquationStep {
            before: current.clone(),
            after: next.clone(),
            rule,
            relation: Relation::Equivalent,
            condition,
        });
        *current = next;
    }
}

/// Solves exact rational-linear equations in one explicitly selected symbol.
///
/// Each side is interpreted in Q[x] with a fixed maximum degree and positive
/// power bound of 32. Only results of degree at most one are solved.
/// Unsupported symbols and ambiguous or negative powers are rejected.
/// Solution cases are unique, none, or every rational; equations are not
/// conflated with expression identities. Steps are true equivalences;
/// the nonzero condition on coefficient division is verified exactly.
pub fn solve_linear_equation(
    equation: &Equation,
    variable: &Symbol,
) -> Result<LinearSolveResult, LinearSolveError> {
    const CONVERSION_BOUND: usize = 32;
    let left = RationalPolynomial::try_from_expr(equation.left(), variable, CONVERSION_BOUND)?;
    let right = RationalPolynomial::try_from_expr(equation.right(), variable, CONVERSION_BOUND)?;
    let residual = left.subtract(&right);
    if residual.degree().is_some_and(|degree| degree > 1) {
        return Err(LinearSolveError::NonlinearEquation);
    }

    let zero = Rational::from(Integer::zero());
    let constant = residual
        .coefficients()
        .first()
        .cloned()
        .unwrap_or_else(|| zero.clone());
    let coefficient = residual
        .coefficients()
        .get(1)
        .cloned()
        .unwrap_or_else(|| zero.clone());
    let mut current = equation.clone();
    let mut steps = Vec::new();

    let reduced = Equation::new(residual.to_expr(variable), Expr::from(0));
    record_step(
        &mut steps,
        &mut current,
        reduced,
        EquationRuleId::CollectToZero,
        None,
    );

    let solution = if coefficient.numerator().is_zero() {
        if constant.numerator().is_zero() {
            LinearSolution::AllRationals
        } else {
            LinearSolution::NoSolution
        }
    } else {
        let neg_constant = -&constant;
        let isolated = Equation::new(
            Expr::mul([
                Expr::from(coefficient.clone()),
                Expr::from(variable.clone()),
            ]),
            Expr::from(neg_constant.clone()),
        );
        record_step(
            &mut steps,
            &mut current,
            isolated,
            EquationRuleId::IsolateLinearTerm,
            None,
        );

        let root = rational_quotient(&neg_constant, &coefficient);
        let solved = Equation::new(Expr::from(variable.clone()), Expr::from(root.clone()));
        record_step(
            &mut steps,
            &mut current,
            solved,
            EquationRuleId::DivideByNonzeroCoefficient,
            Some(VerifiedCondition::Nonzero(coefficient)),
        );
        LinearSolution::Unique(root)
    };

    Ok(LinearSolveResult {
        original: equation.clone(),
        residual,
        solution,
        steps,
    })
}

#[cfg(test)]
mod verification_adversarial_tests {
    use super::*;
    use crate::{VerificationError, verify_linear_solution};

    fn sample() -> (Symbol, LinearSolveResult) {
        let var = Symbol::new("x").unwrap();
        let equation = Equation::new(
            Expr::add([
                Expr::mul([Expr::from(2), Expr::from(var.clone())]),
                Expr::from(4),
            ]),
            Expr::from(10),
        );
        let result = solve_linear_equation(&equation, &var).unwrap();
        (var, result)
    }

    #[test]
    fn detects_a_forged_solution_classification() {
        let (variable, mut result) = sample();
        result.solution = LinearSolution::NoSolution;
        assert_eq!(
            verify_linear_solution(&result, &variable),
            Err(VerificationError::IncorrectResult)
        );
    }

    #[test]
    fn rejects_an_incorrect_equivalence_relation_and_division_condition() {
        let (variable, mut result) = sample();
        result.steps[0].relation = Relation::Equal;
        assert_eq!(
            verify_linear_solution(&result, &variable),
            Err(VerificationError::InvalidStep)
        );

        let (variable, mut result) = sample();
        let last = result.steps.last_mut().unwrap();
        last.condition = Some(VerifiedCondition::Nonzero(Rational::from(Integer::zero())));
        assert_eq!(
            verify_linear_solution(&result, &variable),
            Err(VerificationError::InvalidStep)
        );
    }

    #[test]
    fn rejects_disconnected_equation_steps() {
        let (variable, mut result) = sample();
        result.steps.swap(0, 1);
        assert_eq!(
            verify_linear_solution(&result, &variable),
            Err(VerificationError::BrokenChain)
        );
    }
}
