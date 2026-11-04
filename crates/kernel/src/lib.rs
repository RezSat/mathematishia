//! Mathematical algorithms, domain operations, verification, and derivation
//! generation for Mathematishia.
//!
//! This crate consumes semantic values from `mathema`; it does not own surface
//! parsing, user-facing formatting, or application interfaces.

use mathema::{Expr, ExprView, Symbol};

mod poly_convert;
mod poly_division;
mod polynomial;
mod solve_linear;
mod verification;

pub use poly_convert::PolynomialConversionError;
pub use poly_division::ZeroPolynomialDivisor;
pub use polynomial::RationalPolynomial;
pub use solve_linear::{
    EquationRuleId, EquationStep, LinearSolution, LinearSolveError, LinearSolveResult,
    VerifiedCondition, solve_linear_equation,
};
pub use verification::{
    VerificationError, verify_distribution, verify_distribution_step, verify_equation_step,
    verify_linear_solution,
};

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
    /// The equations have exactly the same solution set in the documented domain.
    Equivalent,
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

/// The outcome of a bounded sequence of distributive-law steps.
///
/// Completion only means that no eligible distribution remains under the
/// supported Add/Mul traversal. Powers are intentionally not traversed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistributionResult {
    result: Expr,
    steps: Vec<DerivationStep>,
    complete: bool,
}

impl DistributionResult {
    /// Returns the expression after the recorded steps.
    pub fn result(&self) -> &Expr {
        &self.result
    }

    /// Returns consecutive equality-preserving steps, in application order.
    pub fn steps(&self) -> &[DerivationStep] {
        &self.steps
    }

    /// Reports whether further distribution is available in the supported scope.
    pub const fn is_complete(&self) -> bool {
        self.complete
    }
}

/// Applies one distributive law at the root or first eligible nested location.
///
/// Root-level products take priority. Otherwise Add/Mul children are searched
/// depth-first in their canonical order. Powers are not traversed. The returned
/// step describes the entire expression before and after the one local change.
pub fn distribute_first_once(expr: &Expr) -> Option<DerivationStep> {
    if let Some(step) = distribute_product_once(expr) {
        return Some(step);
    }

    let (children, is_sum) = match expr.view() {
        ExprView::Add(children) => (children, true),
        ExprView::Mul(children) => (children, false),
        _ => return None,
    };

    for (index, child) in children.iter().enumerate() {
        if let Some(inner) = distribute_first_once(child) {
            let updated = children.iter().enumerate().map(|(position, original)| {
                if position == index {
                    inner.after().clone()
                } else {
                    original.clone()
                }
            });
            let after = if is_sum {
                Expr::add(updated)
            } else {
                Expr::mul(updated)
            };
            return Some(DerivationStep {
                before: expr.clone(),
                after,
                rule: inner.rule(),
                relation: inner.relation(),
            });
        }
    }

    None
}

fn has_distributive_work(expr: &Expr) -> bool {
    match expr.view() {
        ExprView::Add(children) => children.iter().any(has_distributive_work),
        ExprView::Mul(factors) => {
            factors
                .iter()
                .any(|factor| matches!(factor.view(), ExprView::Add(_)))
                || factors.iter().any(has_distributive_work)
        }
        ExprView::Integer(_)
        | ExprView::Rational(_)
        | ExprView::Symbol(_)
        | ExprView::Pow { .. } => false,
    }
}

/// Applies at most max_steps distributive-law steps and preserves their chain.
///
/// A zero budget leaves the input unchanged. A completed result has no further
/// eligible distribution under the Add/Mul traversal, but is not necessarily
/// mathematically simplified. The step count does not bound expression size,
/// memory consumption, or recursion depth.
pub fn expand_distributively(expr: &Expr, max_steps: usize) -> DistributionResult {
    let mut current = expr.clone();
    let mut steps = Vec::new();

    for _ in 0..max_steps {
        let Some(step) = distribute_first_once(&current) else {
            return DistributionResult {
                result: current,
                steps,
                complete: true,
            };
        };
        current = step.after().clone();
        steps.push(step);
    }

    DistributionResult {
        complete: !has_distributive_work(&current),
        result: current,
        steps,
    }
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
