use crate::Expr;

/// The number sets currently available for scalar membership conditions.
///
/// Membership is a mathematical claim, not a declaration that an expression
/// can be evaluated by a particular algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScalarDomain {
    Integer,
    Rational,
    Real,
    Complex,
}

/// A proposition about the value of a scalar expression.
///
/// A condition does not assert its own truth. The kernel determines whether it
/// is established by exact arithmetic, supplied as a premise, or unresolved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Condition {
    NonZero(Expr),
    Positive(Expr),
    Real(Expr),
    Integer(Expr),
    InDomain {
        expression: Expr,
        domain: ScalarDomain,
    },
}

impl Condition {
    pub fn expression(&self) -> &Expr {
        match self {
            Self::NonZero(expr) | Self::Positive(expr) | Self::Real(expr) | Self::Integer(expr) => {
                expr
            }
            Self::InDomain { expression, .. } => expression,
        }
    }
}
