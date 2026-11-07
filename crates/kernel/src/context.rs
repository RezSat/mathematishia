//! Explicit, instance-local admission limits for exact mathematical work.
//!
//! Checks occur on input and at supported operation boundaries. They prevent
//! oversized admitted expressions and record-size overruns, but do not impose
//! hard wall-clock, memory, or intermediate-allocation guarantees.

use std::error::Error;
use std::fmt;

use mathema::{Equation, Expr, ExprView, Integer, Rational, Symbol};

use crate::budget::WorkBudget;
use crate::differentiate_expr::generate_symbolic_derivative;

use crate::{
    DerivationStep, DerivativeVerificationError, DistributionResult, LinearSolution, LinearSolveResult,
    PolynomialConversionError, RationalPolynomial, SquareFreeDecomposition, SquareFreeError,
    SymbolicDerivative, SymbolicDifferentiationError,
    SquareFreeVerificationError, VerificationError, ZeroPolynomialDivisor, distribute_first_once,
    expand_distributively, solve_linear_equation, verify_distribution, verify_linear_solution,
    verify_symbolic_derivative,
};

/// Explicit, configurable limits. None are process-global.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    pub max_expression_nodes: usize,
    pub max_expression_depth: usize,
    pub max_scalar_bits: u64,
    pub max_polynomial_degree: usize,
    pub max_distribution_steps: usize,
    /// Maximum deterministic scalar-operation/node-visit units per checked call.
    pub max_work_units: u64,
    /// Conservative maximum bit length of every intermediate scalar integer.
    pub max_intermediate_scalar_bits: u64,
    /// Explicit upper bound on positive powers differentiated symbolically.
    pub max_symbolic_power: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_expression_nodes: 2_048,
            max_expression_depth: 128,
            max_scalar_bits: 4_096,
            max_polynomial_degree: 32,
            max_distribution_steps: 12,
            max_work_units: 1_000_000,
            max_intermediate_scalar_bits: 16_384,
            max_symbolic_power: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceLimitKind {
    ExpressionNodes,
    ExpressionDepth,
    ScalarBits,
    PolynomialDegree,
    DistributionSteps,
    WorkUnits,
    IntermediateScalarBits,
    SymbolicPower,
}

/// Typed errors distinguish invalid mathematical input from exhausted limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationError {
    Limit(ResourceLimitKind),
    Polynomial(PolynomialConversionError),
    Linear(crate::LinearSolveError),
    DivisionByZero,
    SquareFree(SquareFreeError),
    SquareFreeVerification(SquareFreeVerificationError),
    Verification(VerificationError),
    SymbolicDifferentiation(SymbolicDifferentiationError),
    DerivativeVerification(DerivativeVerificationError),
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(kind) => write!(f, "resource limit exceeded: {kind:?}"),
            Self::Polynomial(error) => write!(f, "{error}"),
            Self::Linear(error) => write!(f, "{error}"),
            Self::DivisionByZero => f.write_str("cannot divide by the zero polynomial"),
            Self::SquareFree(error) => write!(f, "{error}"),
            Self::SquareFreeVerification(error) => {
                write!(f, "square-free verification failed: {error}")
            }
            Self::Verification(error) => write!(f, "derivation verification failed: {error}"),
            Self::SymbolicDifferentiation(error) => write!(f, "{error}"),
            Self::DerivativeVerification(error) => {
                write!(f, "derivative verification failed: {error}")
            }
        }
    }
}

impl Error for EvaluationError {}

impl From<PolynomialConversionError> for EvaluationError {
    fn from(value: PolynomialConversionError) -> Self {
        Self::Polynomial(value)
    }
}

impl From<crate::LinearSolveError> for EvaluationError {
    fn from(value: crate::LinearSolveError) -> Self {
        Self::Linear(value)
    }
}

impl From<ZeroPolynomialDivisor> for EvaluationError {
    fn from(_: ZeroPolynomialDivisor) -> Self {
        Self::DivisionByZero
    }
}

impl From<SymbolicDifferentiationError> for EvaluationError {
    fn from(value: SymbolicDifferentiationError) -> Self {
        Self::SymbolicDifferentiation(value)
    }
}

impl From<DerivativeVerificationError> for EvaluationError {
    fn from(value: DerivativeVerificationError) -> Self {
        Self::DerivativeVerification(value)
    }
}

impl From<VerificationError> for EvaluationError {
    fn from(value: VerificationError) -> Self {
        Self::Verification(value)
    }
}

impl From<SquareFreeError> for EvaluationError {
    fn from(value: SquareFreeError) -> Self {
        Self::SquareFree(value)
    }
}

impl From<SquareFreeVerificationError> for EvaluationError {
    fn from(value: SquareFreeVerificationError) -> Self {
        Self::SquareFreeVerification(value)
    }
}

/// Explicit evaluation options and admission limits for selected algorithms.
///
/// The instance is immutable after construction; nothing depends on hidden
/// global state. Existing unbounded kernel APIs remain available for trusted
/// Rust callers. Checked methods reject work outside their documented scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EvaluationContext {
    limits: ResourceLimits,
}

/// Counts occurrence nodes without recursion; the context checks depth first.
fn expression_size(expr: &Expr) -> u64 {
    let mut count = 0u64;
    let mut stack = vec![expr];
    while let Some(current) = stack.pop() {
        count = count.saturating_add(1);
        match current.view() {
            ExprView::Add(children) | ExprView::Mul(children) => stack.extend(children.iter()),
            ExprView::Pow { base, exponent } => {
                stack.push(base);
                stack.push(exponent);
            }
            ExprView::Integer(_) | ExprView::Rational(_) | ExprView::Symbol(_) => {}
        }
    }
    count
}

impl EvaluationContext {
    pub const fn new(limits: ResourceLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(&self) -> ResourceLimits {
        self.limits
    }

    pub fn with_distribution_steps(mut self, steps: usize) -> Self {
        self.limits.max_distribution_steps = steps;
        self
    }

    fn check_integer(&self, value: &Integer) -> Result<(), EvaluationError> {
        if value.bit_len() > self.limits.max_scalar_bits {
            Err(EvaluationError::Limit(ResourceLimitKind::ScalarBits))
        } else {
            Ok(())
        }
    }

    pub fn check_rational(&self, value: &Rational) -> Result<(), EvaluationError> {
        self.check_integer(value.numerator())?;
        self.check_integer(value.denominator())
    }

    /// Traverses all expression occurrences iteratively, including powers.
    ///
    /// No recursion is used in this admission check. Repeated shared nodes
    /// count once per appearance, not once per Arc allocation.
    pub fn check_expression(&self, expr: &Expr) -> Result<(), EvaluationError> {
        let mut work = vec![(expr, 1usize)];
        let mut seen = 0usize;
        while let Some((current, depth)) = work.pop() {
            seen = seen.saturating_add(1);
            if seen > self.limits.max_expression_nodes {
                return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes));
            }
            if depth > self.limits.max_expression_depth {
                return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionDepth));
            }
            match current.view() {
                ExprView::Integer(value) => self.check_integer(value)?,
                ExprView::Rational(value) => self.check_rational(value)?,
                ExprView::Symbol(_) => {}
                ExprView::Add(children) | ExprView::Mul(children) => {
                    for child in children {
                        work.push((child, depth.saturating_add(1)));
                    }
                }
                ExprView::Pow { base, exponent } => {
                    work.push((base, depth.saturating_add(1)));
                    work.push((exponent, depth.saturating_add(1)));
                }
            }
        }
        Ok(())
    }

    pub fn check_equation(&self, equation: &Equation) -> Result<(), EvaluationError> {
        self.check_expression(equation.left())?;
        self.check_expression(equation.right())
    }

    pub fn check_polynomial(&self, polynomial: &RationalPolynomial) -> Result<(), EvaluationError> {
        if polynomial
            .degree()
            .is_some_and(|degree| degree > self.limits.max_polynomial_degree)
        {
            return Err(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree));
        }
        for coefficient in polynomial.coefficients() {
            self.check_rational(coefficient)?;
        }
        Ok(())
    }

    /// Converts expressions to exact polynomials with input/output checks.
    pub fn polynomial(
        &self,
        expr: &Expr,
        variable: &Symbol,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.check_expression(expr)?;
        let mut work = WorkBudget::new(self.limits);
        let value = work.convert(expr, variable)?;
        self.check_polynomial(&value)?;
        Ok(value)
    }

    /// Applies at most the context's number of steps, validating every new
    /// expression and verifying the resulting derivation independently.
    pub fn expand(&self, original: &Expr) -> Result<DistributionResult, EvaluationError> {
        self.check_expression(original)?;
        let mut work = WorkBudget::new(self.limits);
        let mut current = original.clone();
        let mut steps: Vec<DerivationStep> = Vec::new();
        for _ in 0..self.limits.max_distribution_steps {
            // Budget a conservative cubic upper bound for the traversal and
            // independent candidate enumeration before constructing a step.
            let count = expression_size(&current);
            work.charge(count.saturating_pow(3))?;
            let Some(step) = distribute_first_once(&current) else {
                break;
            };
            self.check_expression(step.after())?;
            current = step.after().clone();
            steps.push(step);
        }
        let count = expression_size(&current);
        work.charge(count.saturating_pow(3))?;
        let complete = expand_distributively(&current, 0).is_complete();
        let result = DistributionResult::from_parts(current, steps, complete);
        verify_distribution(original, &result)?;
        Ok(result)
    }

    /// Solves and independently verifies the exact rational-linear result.
    pub fn solve_linear(
        &self,
        equation: &Equation,
        variable: &Symbol,
    ) -> Result<LinearSolveResult, EvaluationError> {
        self.check_equation(equation)?;
        // Enforce this context's degree and coefficient policy on *each*
        // input side, even if high-degree terms cancel in the residual.
        let mut work = WorkBudget::new(self.limits);
        let left = work.convert(equation.left(), variable)?;
        let right = work.convert(equation.right(), variable)?;
        let residual = work.subtract(&left, &right)?;
        if residual.degree().is_some_and(|degree| degree > 1) {
            return Err(EvaluationError::Linear(
                crate::LinearSolveError::NonlinearEquation,
            ));
        }
        let zero = Rational::from(Integer::zero());
        let coefficient = residual.coefficients().get(1).unwrap_or(&zero);
        if !coefficient.numerator().is_zero() {
            // The solver's unique rational root must be admitted too, before
            // the unmetered legacy solver materializes its trace.
            let constant = residual.coefficients().first().unwrap_or(&zero);
            let root = work.quotient(&(-constant), coefficient)?;
            self.check_rational(&root)?;
        }
        let count =
            expression_size(equation.left()).saturating_add(expression_size(equation.right()));
        work.charge(count.saturating_pow(3).saturating_mul(8))?;
        let result = solve_linear_equation(equation, variable)?;
        self.check_polynomial(result.residual())?;
        for step in result.steps() {
            self.check_equation(step.before())?;
            self.check_equation(step.after())?;
        }
        if let LinearSolution::Unique(value) = result.solution() {
            self.check_rational(value)?;
        }
        verify_linear_solution(&result, variable)?;
        Ok(result)
    }

    /// Differentiates the exact rational-polynomial domain under this context.
    ///
    /// Limits are checked on the input and output. The operation is a formal
    /// derivative over Q[x], not a general Expr differentiation rule.
    pub fn differentiate(
        &self,
        polynomial: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.check_polynomial(polynomial)?;
        let result = WorkBudget::new(self.limits).differentiate(polynomial)?;
        self.check_polynomial(&result)?;
        Ok(result)
    }

    /// Computes and independently verifies a formal symbolic derivative.
    ///
    /// Supports constants, symbols, sums, products and positive integer
    /// powers only. Zero/negative/non-integer powers are intentionally
    /// rejected rather than silently ignoring domain restrictions.
    pub fn differentiate_expression(
        &self,
        original: &Expr,
        variable: &Symbol,
    ) -> Result<SymbolicDerivative, EvaluationError> {
        self.check_expression(original)?;
        let mut meter = WorkBudget::new(self.limits);
        let result = generate_symbolic_derivative(
            original,
            variable,
            self.limits.max_symbolic_power,
            &mut meter,
            |derivative| self.check_expression(derivative),
        )?;
        meter.charge(expression_size(original).saturating_pow(2))?;
        verify_symbolic_derivative(&result)?;
        self.check_expression(result.derivative())?;
        Ok(result)
    }

    /// Separates multiplicity classes, admitting exact input and output values
    /// and independently verifying the reported decomposition.
    ///
    /// Intermediate Euclidean arithmetic is not currently subject to a hard
    /// memory or time limit. This API does not claim full factorization.
    pub fn square_free_decomposition(
        &self,
        polynomial: &RationalPolynomial,
    ) -> Result<SquareFreeDecomposition, EvaluationError> {
        self.check_polynomial(polynomial)?;
        let mut work = WorkBudget::new(self.limits);
        let result = work.square_free(polynomial)?;
        self.check_rational(result.unit())?;
        for factor in result.factors() {
            self.check_polynomial(factor.polynomial())?;
        }
        work.verify_square_free(polynomial, &result)?;
        Ok(result)
    }

    pub fn evaluate_polynomial(
        &self,
        polynomial: &RationalPolynomial,
        at: &Rational,
    ) -> Result<Rational, EvaluationError> {
        self.check_polynomial(polynomial)?;
        self.check_rational(at)?;
        let result = WorkBudget::new(self.limits).evaluate(polynomial, at)?;
        self.check_rational(&result)?;
        Ok(result)
    }

    pub fn divide(
        &self,
        dividend: &RationalPolynomial,
        divisor: &RationalPolynomial,
    ) -> Result<(RationalPolynomial, RationalPolynomial), EvaluationError> {
        self.check_polynomial(dividend)?;
        self.check_polynomial(divisor)?;
        let (q, r) = WorkBudget::new(self.limits).divide(dividend, divisor)?;
        self.check_polynomial(&q)?;
        self.check_polynomial(&r)?;
        Ok((q, r))
    }

    pub fn gcd(
        &self,
        left: &RationalPolynomial,
        right: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.check_polynomial(left)?;
        self.check_polynomial(right)?;
        let result = WorkBudget::new(self.limits).gcd(left, right)?;
        self.check_polynomial(&result)?;
        Ok(result)
    }
}
