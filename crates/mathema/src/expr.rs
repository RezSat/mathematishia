use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use crate::{Integer, Rational, Symbol};

/// An opaque immutable expression with canonical structural equality and hashing.
///
/// Equality is not general mathematical equivalence. Constructors perform only
/// local normalization; there is no public mathematical ordering or traversal API.
/// `Display` is basic human formatting, not a stable serialization format.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Expr(Arc<ExprNode>);

// Arc's equality and hashing delegate to the stored value, not its address.
#[derive(Debug, PartialEq, Eq, Hash)]
enum ExprNode {
    Integer(Integer),
    Rational(Rational),
    Symbol(Symbol),
    Add(Vec<Expr>),
    Mul(Vec<Expr>),
    Pow(Expr, Expr),
}

impl Expr {
    fn new(node: ExprNode) -> Self {
        Self(Arc::new(node))
    }

    /// Flattens sums, folds exact scalars, removes zero, and sorts structurally.
    /// An empty sum is zero; a singleton is its element. No terms are collected.
    ///
    /// ```
    /// use mathema::{Expr, Symbol};
    /// let x = Expr::from(Symbol::new("x")?);
    /// assert_eq!(Expr::add([0.into(), x.clone()]), x);
    /// # Ok::<(), mathema::EmptySymbolName>(())
    /// ```
    pub fn add(operands: impl IntoIterator<Item = Self>) -> Self {
        let mut pending: Vec<_> = operands.into_iter().collect();
        let mut terms = Vec::new();
        let mut scalar = Rational::from(Integer::zero());
        while let Some(operand) = pending.pop() {
            if let ExprNode::Add(children) = operand.0.as_ref() {
                pending.extend(children.iter().cloned());
            } else if let Some(value) = operand.scalar() {
                scalar = scalar + value;
            } else {
                terms.push(operand);
            }
        }
        if !scalar.numerator().is_zero() {
            terms.push(scalar.into());
        }
        terms.sort_by(Self::canonical_cmp);
        match terms.len() {
            0 => Integer::zero().into(),
            1 => terms.pop().unwrap(),
            _ => Self::new(ExprNode::Add(terms)),
        }
    }

    /// Flattens products, folds exact scalars, removes one, and sorts structurally.
    /// Zero annihilates a product; an empty product is one; a singleton is its
    /// element. Repeated symbolic factors are preserved and sums are not expanded.
    pub fn mul(operands: impl IntoIterator<Item = Self>) -> Self {
        let mut pending: Vec<_> = operands.into_iter().collect();
        let mut factors = Vec::new();
        let mut scalar = Rational::from(Integer::one());
        while let Some(operand) = pending.pop() {
            if let ExprNode::Mul(children) = operand.0.as_ref() {
                pending.extend(children.iter().cloned());
            } else if let Some(value) = operand.scalar() {
                if value.numerator().is_zero() {
                    return Integer::zero().into();
                }
                scalar = scalar * value;
            } else {
                factors.push(operand);
            }
        }
        if !(scalar.numerator().is_one() && scalar.denominator().is_one()) {
            factors.push(scalar.into());
        }
        factors.sort_by(Self::canonical_cmp);
        match factors.len() {
            0 => Integer::one().into(),
            1 => factors.pop().unwrap(),
            _ => Self::new(ExprNode::Mul(factors)),
        }
    }

    /// Returns the base for an exact integer exponent of one; otherwise preserves
    /// a power. In particular, zero exponents and numeric powers are not evaluated.
    pub fn pow(base: Self, exponent: Self) -> Self {
        if matches!(exponent.0.as_ref(), ExprNode::Integer(value) if value.is_one()) {
            base
        } else {
            Self::new(ExprNode::Pow(base, exponent))
        }
    }

    fn scalar(&self) -> Option<Rational> {
        match self.0.as_ref() {
            ExprNode::Integer(value) => Some(value.clone().into()),
            ExprNode::Rational(value) => Some(value.clone()),
            _ => None,
        }
    }

    // Canonical rank is scalar < symbol < power < product < sum. This is a
    // structural convention, not mathematical ordering or enum declaration order.
    fn rank(&self) -> u8 {
        match self.0.as_ref() {
            ExprNode::Integer(_) | ExprNode::Rational(_) => 0,
            ExprNode::Symbol(_) => 1,
            ExprNode::Pow(..) => 2,
            ExprNode::Mul(_) => 3,
            ExprNode::Add(_) => 4,
        }
    }

    fn canonical_cmp(&self, other: &Self) -> Ordering {
        self.rank()
            .cmp(&other.rank())
            .then_with(|| match (self.0.as_ref(), other.0.as_ref()) {
                (ExprNode::Integer(a), ExprNode::Integer(b)) => a.cmp(b),
                (ExprNode::Rational(a), ExprNode::Rational(b)) => a.cmp(b),
                (ExprNode::Integer(a), ExprNode::Rational(b)) => {
                    (a * b.denominator()).cmp(b.numerator())
                }
                (ExprNode::Rational(a), ExprNode::Integer(b)) => {
                    a.numerator().cmp(&(b * a.denominator()))
                }
                (ExprNode::Symbol(a), ExprNode::Symbol(b)) => a.cmp(b),
                (ExprNode::Pow(a, b), ExprNode::Pow(c, d)) => {
                    a.canonical_cmp(c).then_with(|| b.canonical_cmp(d))
                }
                (ExprNode::Add(a), ExprNode::Add(b)) | (ExprNode::Mul(a), ExprNode::Mul(b)) => {
                    for (left, right) in a.iter().zip(b) {
                        let order = left.canonical_cmp(right);
                        if order != Ordering::Equal {
                            return order;
                        }
                    }
                    a.len().cmp(&b.len())
                }
                _ => unreachable!("equal ranks have compatible node forms"),
            })
    }

    fn precedence(&self) -> u8 {
        match self.0.as_ref() {
            ExprNode::Add(_) => 1,
            ExprNode::Mul(_) => 2,
            ExprNode::Rational(_) => 3,
            ExprNode::Integer(value) if value < &Integer::zero() => 3,
            ExprNode::Pow(..) => 4,
            _ => 5,
        }
    }

    fn fmt_at(&self, f: &mut fmt::Formatter<'_>, parent: u8) -> fmt::Result {
        let parentheses = self.precedence() < parent;
        if parentheses {
            f.write_str("(")?;
        }
        match self.0.as_ref() {
            ExprNode::Integer(value) => write!(f, "{value}")?,
            ExprNode::Rational(value) => write!(f, "{value}")?,
            ExprNode::Symbol(value) => {
                // Quote names that could be confused with numbers or operators.
                // This is a display convention, not an identifier grammar.
                let mut chars = value.name().chars();
                let bare = chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
                    && chars.all(|c| c.is_alphanumeric() || c == '_');
                if bare {
                    write!(f, "{value}")?;
                } else {
                    write!(f, "{:?}", value.name())?;
                }
            }
            ExprNode::Add(children) | ExprNode::Mul(children) => {
                let separator = if matches!(self.0.as_ref(), ExprNode::Add(_)) {
                    " + "
                } else {
                    " * "
                };
                for (index, child) in children.iter().enumerate() {
                    if index != 0 {
                        f.write_str(separator)?;
                    }
                    child.fmt_at(f, self.precedence())?;
                }
            }
            ExprNode::Pow(base, exponent) => {
                // Parenthesize powers on either side rather than assume an
                // associativity convention. Fractions and negative atoms also
                // need parentheses here to preserve their scope.
                base.fmt_at(f, 5)?;
                f.write_str(" ^ ")?;
                exponent.fmt_at(f, 5)?;
            }
        }
        if parentheses {
            f.write_str(")")?;
        }
        Ok(())
    }
}

impl From<Integer> for Expr {
    fn from(value: Integer) -> Self {
        Self::new(ExprNode::Integer(value))
    }
}

impl From<Rational> for Expr {
    fn from(value: Rational) -> Self {
        if value.denominator().is_one() {
            value.numerator().clone().into()
        } else {
            Self::new(ExprNode::Rational(value))
        }
    }
}

impl From<Symbol> for Expr {
    fn from(value: Symbol) -> Self {
        Self::new(ExprNode::Symbol(value))
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_at(f, 0)
    }
}

impl From<i8> for Expr {
    fn from(value: i8) -> Self {
        Integer::from(value).into()
    }
}

impl From<i16> for Expr {
    fn from(value: i16) -> Self {
        Integer::from(value).into()
    }
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        Integer::from(value).into()
    }
}

impl From<i64> for Expr {
    fn from(value: i64) -> Self {
        Integer::from(value).into()
    }
}

impl From<i128> for Expr {
    fn from(value: i128) -> Self {
        Integer::from(value).into()
    }
}

impl From<isize> for Expr {
    fn from(value: isize) -> Self {
        Integer::from(value).into()
    }
}

impl From<u8> for Expr {
    fn from(value: u8) -> Self {
        Integer::from(value).into()
    }
}

impl From<u16> for Expr {
    fn from(value: u16) -> Self {
        Integer::from(value).into()
    }
}

impl From<u32> for Expr {
    fn from(value: u32) -> Self {
        Integer::from(value).into()
    }
}

impl From<u64> for Expr {
    fn from(value: u64) -> Self {
        Integer::from(value).into()
    }
}

impl From<u128> for Expr {
    fn from(value: u128) -> Self {
        Integer::from(value).into()
    }
}

impl From<usize> for Expr {
    fn from(value: usize) -> Self {
        Integer::from(value).into()
    }
}

#[cfg(test)]
mod tests;
