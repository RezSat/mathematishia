//! Explicit, instance-local admission limits for exact mathematical work.
//!
//! Checks occur on input and at supported operation boundaries. They prevent
//! oversized admitted expressions and record-size overruns, but do not impose
//! hard wall-clock, memory, or intermediate-allocation guarantees.

use std::error::Error;
use std::fmt;

use mathema::{Equation, Expr, ExprView, Integer, Rational, Symbol};

use crate::{
    DerivationStep, DistributionResult, LinearSolution, LinearSolveResult,
    PolynomialConversionError, RationalPolynomial, SquareFreeDecomposition, SquareFreeError,
    SquareFreeVerificationError, VerificationError, ZeroPolynomialDivisor, distribute_first_once,
    expand_distributively, solve_linear_equation, verify_distribution,
    verify_linear_solution, verify_square_free_decomposition,
};

/// Explicit, configurable limits. None are process-global.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    pub max_expression_nodes: usize,
    pub max_expression_depth: usize,
    pub max_scalar_bits: u64,
    pub max_polynomial_degree: usize,
    pub max_distribution_steps: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_expression_nodes: 2_048,
            max_expression_depth: 128,
            max_scalar_bits: 4_096,
            max_polynomial_degree: 32,
            max_distribution_steps: 12,
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
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(kind) => write!(f, "resource limit exceeded: {kind:?}"),
            Self::Polynomial(error) => write!(f, "{error}"),
            Self::Linear(error) => write!(f, "{error}"),
            Self::DivisionByZero => f.write_str("cannot divide by the zero polynomial"),
            Self::SquareFree(error) => write!(f, "{error}"),
            Self::SquareFreeVerification(error) => write!(f, "square-free verification failed: {error}"),
            Self::Verification(error) => write!(f, "derivation verification failed: {error}"),
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
        let value =
            RationalPolynomial::try_from_expr(expr, variable, self.limits.max_polynomial_degree)?;
        self.check_polynomial(&value)?;
        Ok(value)
    }

    /// Applies at most the context's number of steps, validating every new
    /// expression and verifying the resulting derivation independently.
    pub fn expand(&self, original: &Expr) -> Result<DistributionResult, EvaluationError> {
        self.check_expression(original)?;
        let mut current = original.clone();
        let mut steps: Vec<DerivationStep> = Vec::new();
        for _ in 0..self.limits.max_distribution_steps {
            let Some(step) = distribute_first_once(&current) else {
                break;
            };
            self.check_expression(step.after())?;
            current = step.after().clone();
            steps.push(step);
        }
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
        self.polynomial(equation.left(), variable)?;
        self.polynomial(equation.right(), variable)?;
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
        let result = polynomial.differentiate();
        self.check_polynomial(&result)?;
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
        let result = polynomial.square_free_decomposition()?;
        self.check_rational(result.unit())?;
        for factor in result.factors() {
            self.check_polynomial(factor.polynomial())?;
        }
        verify_square_free_decomposition(polynomial, &result)?;
        Ok(result)
    }

    pub fn evaluate_polynomial(
        &self,
        polynomial: &RationalPolynomial,
        at: &Rational,
    ) -> Result<Rational, EvaluationError> {
        self.check_polynomial(polynomial)?;
        self.check_rational(at)?;
        let result = polynomial.evaluate(at);
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
        let (q, r) = dividend.divide_with_remainder(divisor)?;
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
        let result = left.monic_gcd(right);
        self.check_polynomial(&result)?;
        Ok(result)
    }
}
