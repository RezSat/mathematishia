//! Mathematical algorithms, domain operations, verification, and derivation
//! generation for Mathematishia.
//!
//! This crate consumes semantic values from `mathema`; it does not own surface
//! parsing, user-facing formatting, or application interfaces.

use mathema::{Expr, ExprView, Symbol};

/// Replaces every free occurrence of `symbol` with `replacement`.
///
/// Scalars are unchanged; non-target symbols are unchanged; the target symbol
/// returns `replacement` exactly as supplied. Composite expressions recurse
/// over the original children only and rebuild through the canonical `Expr`
/// constructors, so existing normalization may fold the result. The
/// replacement itself is never rescanned, making substitution single-pass.
pub fn substitute_symbol(expr: &Expr, symbol: &Symbol, replacement: &Expr) -> Expr {
    match expr.view() {
        ExprView::Integer(_) | ExprView::Rational(_) => expr.clone(),
        ExprView::Symbol(found) => {
            if found == symbol {
                replacement.clone()
            } else {
                expr.clone()
            }
        }
        ExprView::Add(children) => Expr::add(
            children
                .iter()
                .map(|child| substitute_symbol(child, symbol, replacement)),
        ),
        ExprView::Mul(children) => Expr::mul(
            children
                .iter()
                .map(|child| substitute_symbol(child, symbol, replacement)),
        ),
        ExprView::Pow { base, exponent } => Expr::pow(
            substitute_symbol(base, symbol, replacement),
            substitute_symbol(exponent, symbol, replacement),
        ),
    }
}
