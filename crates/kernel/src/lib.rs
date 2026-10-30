//! Mathematical algorithms, domain operations, verification, and derivation
//! generation for Mathematishia.
//!
//! This crate consumes semantic values from `mathema`; it does not own surface
//! parsing, user-facing formatting, or application interfaces.

use mathema::{Expr, ExprView, Symbol};

/// A stable identifier for a mathematical transformation rule.
///
/// Identifiers are documented names, not display text or runtime lookup keys.
/// Adding a rule requires assigning and documenting a new stable identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RuleId {
    /// The distributive law for a product over a sum.
    ///
    /// Stable identifier: `algebra.distribute_product_over_sum`.
    DistributeProductOverSum,
}

impl RuleId {
    /// Returns the stable identifier for this rule.
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::DistributeProductOverSum => "algebra.distribute_product_over_sum",
        }
    }
}

/// The mathematical relation asserted between a step's input and output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Relation {
    /// The expressions denote equal values within the rule's documented scope.
    Equal,
}

/// A single structured transformation step with a typed mathematical justification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivationStep {
    before: Expr,
    after: Expr,
    rule: RuleId,
    relation: Relation,
}

impl DerivationStep {
    /// Returns the expression before the transformation.
    pub fn before(&self) -> &Expr {
        &self.before
    }

    /// Returns the expression after the transformation.
    pub fn after(&self) -> &Expr {
        &self.after
    }

    /// Returns the rule applied by this step.
    pub const fn rule(&self) -> RuleId {
        self.rule
    }

    /// Returns the relation between the before and after expressions.
    pub const fn relation(&self) -> Relation {
        self.relation
    }
}

/// Applies one distributive-law step to a top-level product containing a sum.
///
/// If `expr` is a top-level `Mul` with at least one `Add` factor, the first such
/// factor in the existing canonical child order is selected. The remaining
/// factors form `R`, and `R * (t1 + ... + tn)` becomes `R*t1 + ... + R*tn`.
/// Results are rebuilt only through `Expr::mul` and `Expr::add`, so their
/// existing scalar folding and canonical ordering apply.
///
/// Nested factors are not inspected or rewritten, and the transformation is
/// applied exactly once rather than repeatedly. If `expr` is not a `Mul` or has
/// no `Add` factor, this returns `None`.
///
/// This rule operates on Mathematishia's commutative scalar-expression
/// semantics. It is an unconditional formal identity; its side conditions are
/// vacuous. Future conditional rules must explicitly represent their
/// obligations rather than relying on prose.
pub fn distribute_product_once(expr: &Expr) -> Option<DerivationStep> {
    let ExprView::Mul(factors) = expr.view() else {
        return None;
    };
    let (selected, terms) = factors
        .iter()
        .enumerate()
        .find_map(|(index, factor)| match factor.view() {
            ExprView::Add(terms) => Some((index, terms)),
            _ => None,
        })?;

    let mut remaining = Vec::with_capacity(factors.len() - 1);
    remaining.extend(factors[..selected].iter().cloned());
    remaining.extend(factors[selected + 1..].iter().cloned());

    let after = Expr::add(terms.iter().map(|term| {
        Expr::mul(
            remaining
                .iter()
                .cloned()
                .chain(std::iter::once(term.clone())),
        )
    }));

    Some(DerivationStep {
        before: expr.clone(),
        after,
        rule: RuleId::DistributeProductOverSum,
        relation: Relation::Equal,
    })
}

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
