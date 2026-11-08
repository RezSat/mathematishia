//! Exact univariate rational expressions with retained source-domain exclusions.

use std::error::Error;
use std::fmt;

use mathema::{Condition, DomainExpression, Expr, ExprView, Integer, Rational, Symbol};

use crate::{
    EvaluationContext, EvaluationError, RationalFunction, RationalFunctionError,
    RationalPolynomial,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalExpressionError {
    Evaluation(EvaluationError),
    RationalFunction(RationalFunctionError),
    OtherSymbol,
    UnsupportedExponent,
    ExponentLimitExceeded,
    UnsupportedCondition,
}

impl fmt::Display for RationalExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::RationalFunction(error) => write!(f, "{error}"),
            Self::OtherSymbol => f.write_str("rational expression contains another symbol"),
            Self::UnsupportedExponent => f.write_str("rational expression requires integer powers"),
            Self::ExponentLimitExceeded => f.write_str("rational exponent exceeds the configured degree limit"),
            Self::UnsupportedCondition => f.write_str("unsupported rational-expression domain condition"),
        }
    }
}

impl Error for RationalExpressionError {}

impl From<EvaluationError> for RationalExpressionError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

impl From<RationalFunctionError> for RationalExpressionError {
    fn from(value: RationalFunctionError) -> Self {
        Self::RationalFunction(value)
    }
}

fn constant(value: Rational) -> RationalFunction {
    RationalFunction::from_polynomial(RationalPolynomial::new(vec![value]))
}

fn one() -> RationalFunction {
    constant(Rational::from(Integer::one()))
}

fn zero() -> RationalFunction {
    constant(Rational::from(Integer::zero()))
}

fn interpret(
    ctx: &EvaluationContext,
    expression: &Expr,
    variable: &Symbol,
) -> Result<RationalFunction, RationalExpressionError> {
    let value = match expression.view() {
        ExprView::Integer(scalar) => constant(Rational::from(scalar.clone())),
        ExprView::Rational(scalar) => constant(scalar.clone()),
        ExprView::Symbol(name) if name == variable => {
            RationalFunction::from_polynomial(RationalPolynomial::new(vec![
                Rational::from(Integer::zero()),
                Rational::from(Integer::one()),
            ]))
        }
        ExprView::Symbol(_) => return Err(RationalExpressionError::OtherSymbol),
        ExprView::Add(terms) => {
            let mut value = zero();
            for term in terms {
                let next = interpret(ctx, term, variable)?;
                value = ctx.add_rational_functions(&value, &next)?;
            }
            value
        }
        ExprView::Mul(factors) => {
            let mut value = one();
            for factor in factors {
                let next = interpret(ctx, factor, variable)?;
                value = ctx.multiply_rational_functions(&value, &next)?;
            }
            value
        }
        ExprView::Pow { base, exponent } => {
            let ExprView::Integer(power) = exponent.view() else {
                return Err(RationalExpressionError::UnsupportedExponent);
            };
            let repr = power.to_string();
            let magnitude = repr.trim_start_matches('-');
            let count = magnitude
                .parse::<usize>()
                .map_err(|_| RationalExpressionError::ExponentLimitExceeded)?;
            if count > ctx.limits().max_polynomial_degree {
                return Err(RationalExpressionError::ExponentLimitExceeded);
            }
            let mut factor = interpret(ctx, base, variable)?;
            let mut result = one();
            let mut remaining = count;
            while remaining != 0 {
                if remaining & 1 != 0 {
                    result = ctx.multiply_rational_functions(&result, &factor)?;
                }
                remaining >>= 1;
                if remaining != 0 {
                    factor = ctx.multiply_rational_functions(&factor, &factor)?;
                }
            }
            if power < &Integer::zero() {
                ctx.divide_rational_functions(&one(), &result)?
            } else {
                result
            }
        }
    };
    ctx.check_rational_function(&value)?;
    Ok(value)
}

impl EvaluationContext {
    /// Converts a domain-aware scalar expression to a reduced partial
    /// rational function in Q(x). Original conditions are enforced after
    /// conversion, including restrictions lost by structural normalization.
    ///
    /// No other free symbols or noninteger exponents are admitted.
    pub fn rational_expression(
        &self,
        input: &DomainExpression,
        variable: &Symbol,
    ) -> Result<RationalFunction, RationalExpressionError> {
        self.check_expression(input.expression())?;
        if input.conditions().len() > self.limits().max_rational_conditions {
            return Err(RationalExpressionError::Evaluation(EvaluationError::Limit(
                crate::ResourceLimitKind::RationalConditions,
            )));
        }
        let mut result = interpret(self, input.expression(), variable)?;
        for condition in input.conditions() {
            self.check_expression(condition.expression())?;
            let Condition::NonZero(restricted) = condition else {
                return Err(RationalExpressionError::UnsupportedCondition);
            };
            let interpreted = interpret(self, restricted, variable)?;
            for exclusion in interpreted.excluded_zero_sets() {
                result = result.restrict_nonzero(exclusion.clone())?;
                self.check_rational_function(&result)?;
            }
            result = result.restrict_nonzero(interpreted.numerator().clone())?;
            self.check_rational_function(&result)?;
        }
        Ok(result)
    }
}
