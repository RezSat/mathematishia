//! Syntax-independent semantic mathematical representation for Mathematishia.
//!
//! Exact scalars and immutable symbolic expressions with conservative constructor
//! normalization. Parsing, transformations, domain algorithms, and bindings are
//! not implemented. `Display` is human formatting, not a serialization format.

mod condition;
mod equation;
mod expr;
mod number;
mod symbol;

pub use condition::{Condition, ScalarDomain};
pub use equation::Equation;
pub use expr::{Expr, ExprView};
pub use number::{Integer, Rational, ZeroDenominator};
pub use symbol::{EmptySymbolName, Symbol};
