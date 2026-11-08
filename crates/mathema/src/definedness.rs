//! Definedness conditions for the integer-power scalar expression domain.

use std::error::Error;
use std::fmt;

use crate::{Condition, Expr, ExprView, Integer};

/// Errors are distinct from an unmet or contradicted nonzero condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinednessError {
    UnsupportedExponent,
    NodeLimitExceeded,
    ConditionLimitExceeded,
}

impl fmt::Display for DefinednessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedExponent => {
                f.write_str("definedness is unsupported for this exponent")
            }
            Self::NodeLimitExceeded => {
                f.write_str("definedness inspection exceeded the node limit")
            }
            Self::ConditionLimitExceeded => f.write_str("definedness condition limit exceeded"),
        }
    }
}

impl Error for DefinednessError {}

/// A normalized expression together with the domain restrictions of its
/// construction history.
///
/// Unlike Expr alone, this type keeps restrictions when normalizing a product
/// removes an undefined factor, as in 0 * x^-1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainExpression {
    expression: Expr,
    conditions: Vec<Condition>,
}

impl DomainExpression {
    pub fn from_expr(expression: Expr, max_nodes: usize) -> Result<Self, DefinednessError> {
        let conditions = definedness_obligations(&expression, max_nodes)?;
        Ok(Self {
            expression,
            conditions,
        })
    }

    /// Constructs an expression with additional restrictions from an earlier
    /// transformation or its source expression.
    pub fn with_conditions(
        expression: Expr,
        inherited: impl IntoIterator<Item = Condition>,
        max_nodes: usize,
        max_conditions: usize,
    ) -> Result<Self, DefinednessError> {
        let mut conditions = Vec::new();
        for condition in inherited {
            append_unique(&mut conditions, condition, max_conditions)?;
        }
        for condition in definedness_obligations(&expression, max_nodes)? {
            append_unique(&mut conditions, condition, max_conditions)?;
        }
        Ok(Self {
            expression,
            conditions,
        })
    }

    pub fn expression(&self) -> &Expr {
        &self.expression
    }

    pub fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    pub fn add(
        parts: impl IntoIterator<Item = Self>,
        max_conditions: usize,
    ) -> Result<Self, DefinednessError> {
        Self::combine(parts, max_conditions, true)
    }

    pub fn multiply(
        parts: impl IntoIterator<Item = Self>,
        max_conditions: usize,
    ) -> Result<Self, DefinednessError> {
        Self::combine(parts, max_conditions, false)
    }

    fn combine(
        parts: impl IntoIterator<Item = Self>,
        max_conditions: usize,
        sum: bool,
    ) -> Result<Self, DefinednessError> {
        let mut expressions = Vec::new();
        let mut conditions = Vec::new();
        for part in parts {
            for condition in part.conditions {
                append_unique(&mut conditions, condition, max_conditions)?;
            }
            expressions.push(part.expression);
        }
        Ok(Self {
            expression: if sum {
                Expr::add(expressions)
            } else {
                Expr::mul(expressions)
            },
            conditions,
        })
    }

    /// Power with an explicitly evaluated integer exponent. Restrictions on
    /// the exponent are retained even if its value was normalized to an integer.
    pub fn pow(self, exponent: Self, max_conditions: usize) -> Result<Self, DefinednessError> {
        let ExprView::Integer(value) = exponent.expression.view() else {
            return Err(DefinednessError::UnsupportedExponent);
        };
        let nonpositive = value <= &Integer::zero();
        let mut conditions = self.conditions;
        for condition in exponent.conditions {
            append_unique(&mut conditions, condition, max_conditions)?;
        }
        if nonpositive {
            append_unique(
                &mut conditions,
                Condition::NonZero(self.expression.clone()),
                max_conditions,
            )?;
        }
        if conditions.len() > max_conditions {
            return Err(DefinednessError::ConditionLimitExceeded);
        }
        Ok(Self {
            expression: Expr::pow(self.expression, exponent.expression),
            conditions,
        })
    }

    pub fn pow_integer(
        self,
        exponent: Integer,
        max_conditions: usize,
    ) -> Result<Self, DefinednessError> {
        let mut conditions = self.conditions;
        if exponent <= Integer::zero() {
            append_unique(
                &mut conditions,
                Condition::NonZero(self.expression.clone()),
                max_conditions,
            )?;
        }
        if conditions.len() > max_conditions {
            return Err(DefinednessError::ConditionLimitExceeded);
        }
        Ok(Self {
            expression: Expr::pow(self.expression, Expr::from(exponent)),
            conditions,
        })
    }
}

fn append_unique(
    conditions: &mut Vec<Condition>,
    condition: Condition,
    max_conditions: usize,
) -> Result<(), DefinednessError> {
    if !conditions.contains(&condition) {
        if conditions.len() >= max_conditions {
            return Err(DefinednessError::ConditionLimitExceeded);
        }
        conditions.push(condition);
    }
    Ok(())
}

/// Finds the conditions required to evaluate an expression over the complex
/// scalars with integer powers. A negative or zero exponent requires its base
/// to be nonzero; this convention leaves 0^0 undefined.
///
/// Conditions are listed in left-to-right depth-first order without duplicates.
/// An empty list means no restrictions in the supported expression grammar.
/// This function inspects an already constructed Expr: conditions discarded by
/// earlier constructor normalization cannot be recovered here.
pub fn definedness_obligations(
    expression: &Expr,
    max_nodes: usize,
) -> Result<Vec<Condition>, DefinednessError> {
    let mut pending = vec![expression];
    let mut conditions = Vec::new();
    let mut visited = 0usize;

    while let Some(expr) = pending.pop() {
        visited += 1;
        if visited > max_nodes {
            return Err(DefinednessError::NodeLimitExceeded);
        }
        match expr.view() {
            ExprView::Integer(_) | ExprView::Rational(_) | ExprView::Symbol(_) => {}
            ExprView::Add(children) | ExprView::Mul(children) => {
                pending.extend(children.iter().rev());
            }
            ExprView::Pow { base, exponent } => {
                let ExprView::Integer(n) = exponent.view() else {
                    return Err(DefinednessError::UnsupportedExponent);
                };
                if n <= &Integer::zero() {
                    let condition = Condition::NonZero(base.clone());
                    if !conditions.contains(&condition) {
                        conditions.push(condition);
                    }
                }
                pending.push(base);
            }
        }
    }

    Ok(conditions)
}
