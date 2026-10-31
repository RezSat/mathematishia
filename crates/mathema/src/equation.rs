use crate::Expr;

/// A mathematical equation, distinct from scalar expressions.
///
/// No truth value or assumptions are attached at construction time.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Equation {
    left: Expr,
    right: Expr,
}

impl Equation {
    pub fn new(left: Expr, right: Expr) -> Self {
        Self { left, right }
    }

    pub fn left(&self) -> &Expr {
        &self.left
    }

    pub fn right(&self) -> &Expr {
        &self.right
    }
}
