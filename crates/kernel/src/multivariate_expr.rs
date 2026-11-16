//! Exact conversion between semantic scalar expressions and a fixed Q[variables] ring.
//!
//! No restrictions erased by earlier Expr normalization can be reconstructed.
//! Use the DomainExpression entrypoint when construction history matters.

use std::error::Error;
use std::fmt;

use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ExprView, Integer, Rational, Symbol,
};

use crate::{MultivariateError, MultivariateLimits, MultivariatePolynomial};

const MAX_EXPRESSION_DEPTH: usize = 128;
const MAX_EXPRESSION_NODES: usize = 100_000;
const MAX_DOMAIN_CONDITIONS: usize = 128;

/// A conversion failure is distinct from an exact polynomial result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultivariateExprError {
    UndeclaredSymbol,
    UnsupportedAnalyticFunction,
    UnsupportedUserFunction,
    ZeroExponent,
    NegativeExponent,
    NonIntegerExponent,
    SymbolicExponent,
    ExponentLimit,
    ExpressionDepthLimit,
    ExpressionNodeLimit,
    ConditionLimit,
    Polynomial(MultivariateError),
    Domain(DefinednessError),
}

impl fmt::Display for MultivariateExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndeclaredSymbol => f.write_str("expression contains an undeclared variable"),
            Self::UnsupportedAnalyticFunction => {
                f.write_str("analytic functions are not rational polynomials")
            }
            Self::UnsupportedUserFunction => {
                f.write_str("user-defined functions are not rational polynomials")
            }
            Self::ZeroExponent => f.write_str("zero powers are not admitted (including 0^0)"),
            Self::NegativeExponent => f.write_str("negative powers are not polynomials"),
            Self::NonIntegerExponent => f.write_str("noninteger powers are not polynomials"),
            Self::SymbolicExponent => f.write_str("symbolic powers are not polynomials"),
            Self::ExponentLimit => f.write_str("positive exponent exceeds the degree limit"),
            Self::ExpressionDepthLimit => f.write_str("expression nesting limit exceeded"),
            Self::ExpressionNodeLimit => f.write_str("expression node limit exceeded"),
            Self::ConditionLimit => f.write_str("domain condition limit exceeded"),
            Self::Polynomial(error) => write!(f, "{error}"),
            Self::Domain(error) => write!(f, "{error}"),
        }
    }
}

impl Error for MultivariateExprError {}

impl From<MultivariateError> for MultivariateExprError {
    fn from(error: MultivariateError) -> Self {
        Self::Polynomial(error)
    }
}

impl From<DefinednessError> for MultivariateExprError {
    fn from(error: DefinednessError) -> Self {
        Self::Domain(error)
    }
}

/// Conversion result retaining the original domain conditions verbatim.
///
/// The conditions are obligations, not proofs of definedness. Their order and
/// exact expression structure are preserved through the conversion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainMultivariatePolynomial {
    polynomial: MultivariatePolynomial,
    conditions: Vec<Condition>,
}

impl DomainMultivariatePolynomial {
    pub fn polynomial(&self) -> &MultivariatePolynomial {
        &self.polynomial
    }

    pub fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    /// Rebuilds a domain expression without removing inherited obligations.
    pub fn to_domain_expression(
        &self,
        limits: MultivariateLimits,
    ) -> Result<DomainExpression, MultivariateExprError> {
        if self.conditions.len() > MAX_DOMAIN_CONDITIONS {
            return Err(MultivariateExprError::ConditionLimit);
        }
        let expression = self.polynomial.to_expr(limits)?;
        Ok(DomainExpression::with_conditions(
            expression,
            self.conditions.clone(),
            MAX_EXPRESSION_NODES,
            MAX_DOMAIN_CONDITIONS,
        )?)
    }
}

fn bounded(limits: MultivariateLimits) -> MultivariateLimits {
    let hard = MultivariateLimits::HARD_LIMITS;
    MultivariateLimits {
        max_variables: limits.max_variables.min(hard.max_variables),
        max_terms: limits.max_terms.min(hard.max_terms),
        max_input_terms: limits.max_input_terms.min(hard.max_input_terms),
        max_total_degree: limits.max_total_degree.min(hard.max_total_degree),
        max_scalar_bits: limits.max_scalar_bits.min(hard.max_scalar_bits),
        max_work_units: limits.max_work_units.min(hard.max_work_units),
    }
}

/// Conservative global conversion meter in addition to the polynomial domain's
/// per-operation admission checks. Work units are not wall-clock time.
struct BridgeMeter {
    limits: MultivariateLimits,
    remaining: u64,
    nodes: usize,
}

impl BridgeMeter {
    fn new(limits: MultivariateLimits) -> Self {
        let limits = bounded(limits);
        Self {
            remaining: limits.max_work_units,
            limits,
            nodes: 0,
        }
    }

    fn charge(&mut self, units: u64) -> Result<(), MultivariateExprError> {
        self.remaining = self
            .remaining
            .checked_sub(units)
            .ok_or(MultivariateError::WorkLimit)?;
        Ok(())
    }

    fn enter(&mut self, depth: usize) -> Result<(), MultivariateExprError> {
        if depth >= MAX_EXPRESSION_DEPTH {
            return Err(MultivariateExprError::ExpressionDepthLimit);
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or(MultivariateExprError::ExpressionNodeLimit)?;
        if self.nodes > MAX_EXPRESSION_NODES {
            return Err(MultivariateExprError::ExpressionNodeLimit);
        }
        self.charge(1)
    }

    fn admit(&self, polynomial: &MultivariatePolynomial) -> Result<(), MultivariateExprError> {
        if polynomial.variables().len() > self.limits.max_variables {
            return Err(MultivariateError::VariableLimit.into());
        }
        if polynomial.terms().len() > self.limits.max_terms {
            return Err(MultivariateError::TermLimit.into());
        }
        for (powers, coefficient) in polynomial.terms() {
            let degree = powers.iter().try_fold(0_u32, |sum, &power| {
                sum.checked_add(power)
                    .ok_or(MultivariateError::TotalDegreeLimit)
            })?;
            if degree > self.limits.max_total_degree {
                return Err(MultivariateError::TotalDegreeLimit.into());
            }
            if coefficient.numerator().bit_len() > self.limits.max_scalar_bits
                || coefficient.denominator().bit_len() > self.limits.max_scalar_bits
            {
                return Err(MultivariateError::ScalarBitLimit.into());
            }
        }
        Ok(())
    }

    fn multiply(
        &mut self,
        left: &MultivariatePolynomial,
        right: &MultivariatePolynomial,
    ) -> Result<MultivariatePolynomial, MultivariateExprError> {
        let pairs = left
            .terms()
            .len()
            .checked_mul(right.terms().len())
            .ok_or(MultivariateError::InputTermLimit)?;
        if pairs > self.limits.max_input_terms {
            return Err(MultivariateError::InputTermLimit.into());
        }
        // Upper bound for visited products, scalar multiplications, insertion
        // and term collection. Fail before materializing Cartesian products.
        let cost = u64::try_from(pairs).unwrap_or(u64::MAX).saturating_mul(4) + 1;
        self.charge(cost)?;
        Ok(left.multiply(right, self.limits)?)
    }

    fn sum(
        &mut self,
        variables: &[Symbol],
        parts: &[MultivariatePolynomial],
    ) -> Result<MultivariatePolynomial, MultivariateExprError> {
        let mut count = 0usize;
        for part in parts {
            count = count
                .checked_add(part.terms().len())
                .ok_or(MultivariateError::InputTermLimit)?;
            if count > self.limits.max_input_terms {
                return Err(MultivariateError::InputTermLimit.into());
            }
        }
        self.charge(u64::try_from(count).unwrap_or(u64::MAX).saturating_mul(2) + 1)?;
        let mut terms = Vec::with_capacity(count);
        for part in parts {
            for (powers, coefficient) in part.terms() {
                // All admitted powers are <= 128, so narrowing is exact.
                terms.push((
                    powers.iter().map(|&power| power as i32).collect(),
                    coefficient.clone(),
                ));
            }
        }
        Ok(MultivariatePolynomial::try_new(
            variables.to_vec(),
            terms,
            self.limits,
        )?)
    }
}

fn positive_power(
    exponent: &Expr,
    limits: MultivariateLimits,
) -> Result<u32, MultivariateExprError> {
    let ExprView::Integer(value) = exponent.view() else {
        return Err(match exponent.view() {
            ExprView::Rational(_) => MultivariateExprError::NonIntegerExponent,
            _ => MultivariateExprError::SymbolicExponent,
        });
    };
    if value == &Integer::zero() {
        return Err(MultivariateExprError::ZeroExponent);
    }
    if value < &Integer::zero() {
        return Err(MultivariateExprError::NegativeExponent);
    }
    if value > &Integer::from(limits.max_total_degree) {
        return Err(MultivariateExprError::ExponentLimit);
    }
    Ok(value
        .try_i64()
        .expect("admitted nonnegative exponent is at most 128") as u32)
}

fn convert(
    expression: &Expr,
    variables: &[Symbol],
    meter: &mut BridgeMeter,
    depth: usize,
) -> Result<MultivariatePolynomial, MultivariateExprError> {
    meter.enter(depth)?;
    let limits = meter.limits;
    match expression.view() {
        ExprView::Integer(value) => Ok(MultivariatePolynomial::constant(
            variables.to_vec(),
            Rational::from(value.clone()),
            limits,
        )?),
        ExprView::Rational(value) => Ok(MultivariatePolynomial::constant(
            variables.to_vec(),
            value.clone(),
            limits,
        )?),
        ExprView::Symbol(symbol) => {
            let index = variables
                .binary_search(symbol)
                .map_err(|_| MultivariateExprError::UndeclaredSymbol)?;
            let mut powers = vec![0_i32; variables.len()];
            powers[index] = 1;
            Ok(MultivariatePolynomial::try_new(
                variables.to_vec(),
                [(powers, Rational::from(Integer::one()))],
                limits,
            )?)
        }
        ExprView::Function { name, .. } => Err(match name.name() {
            "sin" | "cos" | "tan" | "exp" | "log" => {
                MultivariateExprError::UnsupportedAnalyticFunction
            }
            _ => MultivariateExprError::UnsupportedUserFunction,
        }),
        ExprView::Add(children) => {
            let mut parts = Vec::with_capacity(children.len().min(MAX_EXPRESSION_NODES));
            for child in children {
                parts.push(convert(child, variables, meter, depth + 1)?);
            }
            meter.sum(variables, &parts)
        }
        ExprView::Mul(children) => {
            let mut product = MultivariatePolynomial::constant(
                variables.to_vec(),
                Rational::from(Integer::one()),
                limits,
            )?;
            for child in children {
                let factor = convert(child, variables, meter, depth + 1)?;
                product = meter.multiply(&product, &factor)?;
            }
            Ok(product)
        }
        ExprView::Pow { base, exponent } => {
            let power = positive_power(exponent, limits)?;
            let mut factor = convert(base, variables, meter, depth + 1)?;
            if factor
                .total_degree()
                .is_some_and(|degree| degree.saturating_mul(power) > limits.max_total_degree)
            {
                return Err(MultivariateError::TotalDegreeLimit.into());
            }
            let mut result = MultivariatePolynomial::constant(
                variables.to_vec(),
                Rational::from(Integer::one()),
                limits,
            )?;
            let mut remaining = power;
            while remaining != 0 {
                if remaining & 1 != 0 {
                    result = meter.multiply(&result, &factor)?;
                }
                remaining >>= 1;
                if remaining != 0 {
                    factor = meter.multiply(&factor, &factor)?;
                }
            }
            Ok(result)
        }
    }
}

impl MultivariatePolynomial {
    /// Converts Expr exactly in the explicitly selected ambient Q[variables].
    ///
    /// Input variables may be in any order but must be distinct; terms always
    /// use the canonical Symbol order. Only strictly positive integer powers
    /// are supported (including for constant bases). A plain Expr cannot
    /// reconstruct domain restrictions lost during earlier normalization.
    pub fn try_from_expr(
        expression: &Expr,
        variables: &[Symbol],
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateExprError> {
        let mut meter = BridgeMeter::new(limits);
        let ring = Self::zero(variables.to_vec(), meter.limits)?;
        let polynomial = convert(expression, ring.variables(), &mut meter, 0)?;
        meter.admit(&polynomial)?;
        Ok(polynomial)
    }

    /// Converts a domain-aware expression without dropping historical conditions.
    pub fn try_from_domain_expression(
        input: &DomainExpression,
        variables: &[Symbol],
        limits: MultivariateLimits,
    ) -> Result<DomainMultivariatePolynomial, MultivariateExprError> {
        if input.conditions().len() > MAX_DOMAIN_CONDITIONS {
            return Err(MultivariateExprError::ConditionLimit);
        }
        let polynomial = Self::try_from_expr(input.expression(), variables, limits)?;
        Ok(DomainMultivariatePolynomial {
            polynomial,
            conditions: input.conditions().to_vec(),
        })
    }

    /// Converts a bounded polynomial to an equivalent semantic expression.
    ///
    /// The ambient ring is not encoded by Expr; callers must supply the same
    /// ring on conversion back. The returned AST need not match original syntax.
    pub fn to_expr(&self, limits: MultivariateLimits) -> Result<Expr, MultivariateExprError> {
        let mut meter = BridgeMeter::new(limits);
        meter.admit(self)?;
        meter.charge(1)?;
        let mut terms = Vec::with_capacity(self.terms().len());
        for (powers, coefficient) in self.terms() {
            meter.charge(1)?;
            let mut factors = vec![Expr::from(coefficient.clone())];
            for (symbol, &power) in self.variables().iter().zip(powers) {
                if power == 0 {
                    continue;
                }
                meter.charge(2)?;
                let variable = Expr::from(symbol.clone());
                factors.push(if power == 1 {
                    variable
                } else {
                    Expr::pow(variable, Expr::from(power))
                });
            }
            terms.push(Expr::mul(factors));
        }
        Ok(Expr::add(terms))
    }
}
