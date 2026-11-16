//! Exact affine systems over a declared ordered tuple of rational variables.
//!
//! Only rational affine expressions are accepted. Source-domain restrictions
//! erased by Expr constructor normalization cannot be recovered.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use mathema::{Equation, Expr, ExprView, Integer, Rational, Symbol};

use crate::budget::WorkBudget;
use crate::{EvaluationContext, EvaluationError, ExactMatrix, LinearSystemSolution, MatrixError};

const MAX_SYSTEM_DIM: usize = 16;
const MAX_CONSTANT_POWER: usize = 32;

fn zero() -> Rational {
    Rational::from(Integer::zero())
}

fn one() -> Rational {
    Rational::from(Integer::one())
}

/// Explicit rejection, distinct from a mathematically empty solution set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinearSystemExprError {
    EmptyEquations,
    EmptyVariables,
    DuplicateVariable,
    DimensionLimit,
    UnlistedSymbol,
    NonAffine,
    UnsupportedPower,
    Evaluation(EvaluationError),
    Matrix(MatrixError),
    VerificationFailed,
}

impl fmt::Display for LinearSystemExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEquations => f.write_str("at least one equation is required"),
            Self::EmptyVariables => f.write_str("at least one variable is required"),
            Self::DuplicateVariable => f.write_str("variables must be distinct"),
            Self::DimensionLimit => f.write_str("system exceeds the 16 by 16 limit"),
            Self::UnlistedSymbol => f.write_str("expression contains an undeclared symbol"),
            Self::NonAffine => f.write_str("expression is not affine in the declared variables"),
            Self::UnsupportedPower => f.write_str("power is unsupported in a rational affine system"),
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Matrix(error) => write!(f, "{error}"),
            Self::VerificationFailed => f.write_str("exact solution verification failed"),
        }
    }
}

impl Error for LinearSystemExprError {}

impl From<EvaluationError> for LinearSystemExprError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl From<MatrixError> for LinearSystemExprError {
    fn from(error: MatrixError) -> Self {
        Self::Matrix(error)
    }
}

/// Solutions are indexed in exactly the supplied variable order. For a family,
/// every vector is particular + sum(parameter_i * basis_i), parameters in Q.
/// The basis is ordered by ascending free-variable column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpressionLinearSystemSet {
    Empty,
    Unique(Vec<Rational>),
    Affine {
        particular: Vec<Rational>,
        basis: Vec<Vec<Rational>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpressionLinearSystemSolution {
    variables: Vec<Symbol>,
    set: ExpressionLinearSystemSet,
}

impl ExpressionLinearSystemSolution {
    /// Constructs a candidate for independent verification; no validity is implied.
    pub fn from_parts(variables: Vec<Symbol>, set: ExpressionLinearSystemSet) -> Self {
        Self { variables, set }
    }

    pub fn variables(&self) -> &[Symbol] {
        &self.variables
    }

    pub fn set(&self) -> &ExpressionLinearSystemSet {
        &self.set
    }
}

#[derive(Clone)]
struct Affine {
    coefficients: Vec<Rational>,
    constant: Rational,
}

impl Affine {
    fn scalar(value: Rational, count: usize) -> Self {
        Self {
            coefficients: vec![zero(); count],
            constant: value,
        }
    }

    fn is_constant(&self) -> bool {
        self.coefficients.iter().all(|coefficient| coefficient.numerator().is_zero())
    }
}

fn extract(
    expression: &Expr,
    variables: &[Symbol],
    work: &mut WorkBudget,
) -> Result<Affine, LinearSystemExprError> {
    work.charge(1)?;
    match expression.view() {
        ExprView::Integer(value) => Ok(Affine::scalar(Rational::from(value.clone()), variables.len())),
        ExprView::Rational(value) => Ok(Affine::scalar(value.clone(), variables.len())),
        ExprView::Symbol(symbol) => {
            let Some(column) = variables.iter().position(|variable| variable == symbol) else {
                return Err(LinearSystemExprError::UnlistedSymbol);
            };
            let mut result = Affine::scalar(zero(), variables.len());
            result.coefficients[column] = one();
            Ok(result)
        }
        ExprView::Add(children) => {
            let mut result = Affine::scalar(zero(), variables.len());
            for child in children {
                let part = extract(child, variables, work)?;
                result.constant = work.add_scalar(&result.constant, &part.constant)?;
                for (coefficient, other) in result.coefficients.iter_mut().zip(part.coefficients) {
                    *coefficient = work.add_scalar(coefficient, &other)?;
                }
            }
            Ok(result)
        }
        ExprView::Mul(children) => {
            let mut result = Affine::scalar(one(), variables.len());
            for child in children {
                let part = extract(child, variables, work)?;
                if !result.is_constant() && !part.is_constant() {
                    return Err(LinearSystemExprError::NonAffine);
                }
                let constant = work.multiply_scalar(&result.constant, &part.constant)?;
                let coefficients = result
                    .coefficients
                    .iter()
                    .zip(&part.coefficients)
                    .map(|(left, right)| {
                        let a = work.multiply_scalar(left, &part.constant)?;
                        let b = work.multiply_scalar(right, &result.constant)?;
                        Ok(work.add_scalar(&a, &b)?)
                    })
                    .collect::<Result<Vec<_>, LinearSystemExprError>>()?;
                result = Affine {
                    coefficients,
                    constant,
                };
            }
            Ok(result)
        }
        ExprView::Pow { base, exponent } => {
            let ExprView::Integer(n) = exponent.view() else {
                return Err(LinearSystemExprError::UnsupportedPower);
            };
            // Powers of a symbolic affine form are deliberately not admitted,
            // including exponent zero: its domain may exclude base = 0.
            let base = extract(base, variables, work)?;
            if !base.is_constant() {
                return Err(LinearSystemExprError::UnsupportedPower);
            }
            let bound = Integer::from(MAX_CONSTANT_POWER);
            if n > &bound || n < &(-&bound) {
                return Err(LinearSystemExprError::UnsupportedPower);
            }
            if n <= &Integer::zero() && base.constant.numerator().is_zero() {
                return Err(LinearSystemExprError::UnsupportedPower);
            }
            let mut result = one();
            // Bound is small; Integer has no lossy integer conversion here.
            let mut magnitude = if n < &Integer::zero() { -n } else { n.clone() };
            while magnitude > Integer::zero() {
                result = work.multiply_scalar(&result, &base.constant)?;
                magnitude = magnitude - Integer::one();
            }
            if n < &Integer::zero() {
                result = work.quotient(&one(), &result)?;
            }
            Ok(Affine::scalar(result, variables.len()))
        }
    }
}

fn extract_system(
    context: &EvaluationContext,
    equations: &[Equation],
    variables: &[Symbol],
) -> Result<(ExactMatrix, Vec<Rational>), LinearSystemExprError> {
    if equations.is_empty() {
        return Err(LinearSystemExprError::EmptyEquations);
    }
    if variables.is_empty() {
        return Err(LinearSystemExprError::EmptyVariables);
    }
    if equations.len() > MAX_SYSTEM_DIM || variables.len() > MAX_SYSTEM_DIM {
        return Err(LinearSystemExprError::DimensionLimit);
    }
    if variables.iter().collect::<BTreeSet<_>>().len() != variables.len() {
        return Err(LinearSystemExprError::DuplicateVariable);
    }
    let mut work = WorkBudget::new(context.limits());
    let mut rows = Vec::with_capacity(equations.len());
    let mut rhs = Vec::with_capacity(equations.len());
    for equation in equations {
        context.check_equation(equation)?;
        let left = extract(equation.left(), variables, &mut work)?;
        let right = extract(equation.right(), variables, &mut work)?;
        let mut row = Vec::with_capacity(variables.len());
        for (a, b) in left.coefficients.iter().zip(&right.coefficients) {
            let coefficient = work.subtract_scalar(a, b)?;
            context.check_rational(&coefficient)?;
            row.push(coefficient);
        }
        let target = work.subtract_scalar(&right.constant, &left.constant)?;
        context.check_rational(&target)?;
        rows.push(row);
        rhs.push(target);
    }
    Ok((ExactMatrix::new(context, rows)?, rhs))
}

fn solve_unchecked(
    context: &EvaluationContext,
    equations: &[Equation],
    variables: &[Symbol],
) -> Result<ExpressionLinearSystemSolution, LinearSystemExprError> {
    let (matrix, rhs) = extract_system(context, equations, variables)?;
    let set = match matrix.solve(context, &rhs)? {
        LinearSystemSolution::Inconsistent => ExpressionLinearSystemSet::Empty,
        LinearSystemSolution::Unique(values) => ExpressionLinearSystemSet::Unique(values),
        LinearSystemSolution::Infinite {
            particular,
            nullspace_basis,
        } => ExpressionLinearSystemSet::Affine {
            particular,
            basis: nullspace_basis,
        },
    };
    Ok(ExpressionLinearSystemSolution {
        variables: variables.to_vec(),
        set,
    })
}

/// Rational solutions of a system of equalities, with checked exact residuals.
///
/// Uses existing exact matrix elimination. Rejection is conservative: unknown
/// symbols, non-affine products and symbolic powers are not reinterpreted.
pub fn solve_linear_system_expr(
    context: &EvaluationContext,
    equations: &[Equation],
    variables: &[Symbol],
) -> Result<ExpressionLinearSystemSolution, LinearSystemExprError> {
    let solution = solve_unchecked(context, equations, variables)?;
    if !verify_linear_system_expr(context, equations, variables, &solution)? {
        return Err(LinearSystemExprError::VerificationFailed);
    }
    Ok(solution)
}

// Evaluate original syntax at an exact vector, independently of coefficient
// extraction; negative powers and zero powers are validated at the point.
fn substitute(
    expr: &Expr,
    variables: &[Symbol],
    values: &[Rational],
    work: &mut WorkBudget,
) -> Result<Rational, LinearSystemExprError> {
    work.charge(1)?;
    match expr.view() {
        ExprView::Integer(value) => Ok(Rational::from(value.clone())),
        ExprView::Rational(value) => Ok(value.clone()),
        ExprView::Symbol(symbol) => {
            let position = variables.iter().position(|variable| variable == symbol)
                .ok_or(LinearSystemExprError::UnlistedSymbol)?;
            Ok(values[position].clone())
        }
        ExprView::Add(children) => {
            let mut sum = zero();
            for child in children {
                let value = substitute(child, variables, values, work)?;
                sum = work.add_scalar(&sum, &value)?;
            }
            Ok(sum)
        }
        ExprView::Mul(children) => {
            let mut product = one();
            for child in children {
                let value = substitute(child, variables, values, work)?;
                product = work.multiply_scalar(&product, &value)?;
            }
            Ok(product)
        }
        ExprView::Pow { base, exponent } => {
            let ExprView::Integer(power) = exponent.view() else {
                return Err(LinearSystemExprError::UnsupportedPower);
            };
            let bound = Integer::from(MAX_CONSTANT_POWER);
            if power > &bound || power < &(-&bound) {
                return Err(LinearSystemExprError::UnsupportedPower);
            }
            let base = substitute(base, variables, values, work)?;
            if power <= &Integer::zero() && base.numerator().is_zero() {
                return Err(LinearSystemExprError::UnsupportedPower);
            }
            let mut result = one();
            let mut magnitude = if power < &Integer::zero() { -power } else { power.clone() };
            while magnitude > Integer::zero() {
                result = work.multiply_scalar(&result, &base)?;
                magnitude = magnitude - Integer::one();
            }
            if power < &Integer::zero() {
                result = work.quotient(&one(), &result)?;
            }
            Ok(result)
        }
    }
}

fn satisfies(
    context: &EvaluationContext,
    equations: &[Equation],
    variables: &[Symbol],
    values: &[Rational],
) -> Result<bool, LinearSystemExprError> {
    if values.len() != variables.len() {
        return Ok(false);
    }
    let mut work = WorkBudget::new(context.limits());
    for value in values {
        context.check_rational(value)?;
    }
    for equation in equations {
        let left = substitute(equation.left(), variables, values, &mut work)?;
        let right = substitute(equation.right(), variables, values, &mut work)?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Recompute the complete canonical matrix classification and independently
/// substitute the particular point and each basis direction into original
/// equations. False (not success) is returned for forged classifications.
pub fn verify_linear_system_expr(
    context: &EvaluationContext,
    equations: &[Equation],
    variables: &[Symbol],
    proposed: &ExpressionLinearSystemSolution,
) -> Result<bool, LinearSystemExprError> {
    let expected = solve_unchecked(context, equations, variables)?;
    if proposed != &expected {
        return Ok(false);
    }
    match &proposed.set {
        ExpressionLinearSystemSet::Empty => Ok(true),
        ExpressionLinearSystemSet::Unique(values) => {
            satisfies(context, equations, variables, values)
        }
        ExpressionLinearSystemSet::Affine { particular, basis } => {
            if !satisfies(context, equations, variables, particular)? {
                return Ok(false);
            }
            for direction in basis {
                if direction.len() != particular.len() {
                    return Ok(false);
                }
                let mut offset = Vec::with_capacity(direction.len());
                let mut work = WorkBudget::new(context.limits());
                for (point, delta) in particular.iter().zip(direction) {
                    offset.push(work.add_scalar(point, delta)?);
                }
                if !satisfies(context, equations, variables, &offset)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}
