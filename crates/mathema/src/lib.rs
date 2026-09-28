//! Syntax-independent semantic mathematical representation for Mathematishia.
//!
//! Exact integer and canonical rational values with basic value arithmetic.
//! Symbolic expressions, parsing, domain algorithms, and bindings are not yet
//! implemented. Scalar `Display` is human formatting, not a serialization format.

mod number;

pub use number::{Integer, Rational, ZeroDenominator};
