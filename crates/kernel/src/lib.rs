//! Mathematical algorithms, domain operations, verification, and derivation
//! generation for Mathematishia.
//!
//! This crate consumes semantic values from `mathema`; it does not own surface
//! parsing, user-facing formatting, or application interfaces.

use mathema::{Expr, ExprView, Symbol};

mod algebraic;
mod assumptions;
mod budget;
mod conditional_rewrite;
mod context;
mod differentiate_expr;
mod domain_operations;
mod extended_gcd;
mod guarded_rewrite;
mod poly_convert;
mod poly_division;
mod polynomial;
mod polynomial_residue;
mod proof_graph;
mod quadratic;
mod rational_equation;
mod rational_expression;
mod rational_function;
mod rational_quadratic;
mod solve_linear;
mod square_free;
mod verification;

pub use algebraic::{
    AlgebraicRootEvidence, AlgebraicRootResult, AlgebraicRootRule, AlgebraicRootSet,
    AlgebraicVerificationError, ExactQuadraticRoot, QuadraticBranch, verify_algebraic_roots,
};
pub use assumptions::{AssumptionError, Assumptions, ConditionStatus};
pub use conditional_rewrite::{
    ConditionEvidence, ConditionalRewriteOutcome, ConditionalRewriteStep,
    ConditionalRewriteVerificationError, ConditionalRuleId, cancel_composite_inverse_once,
    verify_composite_inverse_step,
};
pub use context::{EvaluationContext, EvaluationError, ResourceLimitKind, ResourceLimits};
pub use differentiate_expr::{
    DerivativeRuleId, DerivativeVerificationError, DifferentiationStep, SymbolicDerivative,
    SymbolicDifferentiationError, verify_symbolic_derivative,
};
pub use domain_operations::{
    DomainOperationError, DomainRewrite, DomainRewriteOutcome, rewrite_domain_once,
    substitute_domain,
};
pub use extended_gcd::{
    EuclideanStep, ExtendedGcdResult, ExtendedGcdVerificationError, verify_extended_gcd,
};
pub use guarded_rewrite::{
    GuardedRewriteOutcome, GuardedRewriteStep, GuardedRewriteVerificationError, GuardedRuleId,
    cancel_symbol_inverse_once, verify_guarded_rewrite_step,
};
pub use mathema::{DefinednessError, DomainExpression, definedness_obligations};
pub use poly_convert::PolynomialConversionError;
pub use poly_division::ZeroPolynomialDivisor;
pub use polynomial_residue::{PolynomialResidue, ResidueError, verify_residue_product};
pub use polynomial::RationalPolynomial;
pub use proof_graph::{
    ProofGraph, ProofGraphBuildError, ProofGraphVerificationError, ProofNode, ProofNodeId,
    ProofNodeKind, verify_proof_graph,
};
pub use quadratic::{
    QuadraticError, QuadraticResult, QuadraticRoots, QuadraticVerificationError,
    verify_quadratic_roots,
};
pub use rational_equation::{
    RationalEquationError, RationalEquationResult, RationalEquationSolution,
    RationalEquationVerificationError, verify_rational_equation,
};
pub use rational_expression::RationalExpressionError;
pub use rational_function::{RationalFunction, RationalFunctionError};
pub use rational_quadratic::{
    RationalQuadraticResult, RationalQuadraticRoots, verify_rational_quadratic_equation,
};
pub use solve_linear::{
    EquationRuleId, EquationStep, LinearSolution, LinearSolveError, LinearSolveResult,
    VerifiedCondition, solve_linear_equation,
};
pub use square_free::{
    SquareFreeDecomposition, SquareFreeError, SquareFreeFactor, SquareFreeVerificationError,
    verify_square_free_decomposition,
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
    /// The result is a formal derivative of the input in the selected variable.
    DerivativeOf,
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

    pub(crate) fn from_parts(result: Expr, steps: Vec<DerivationStep>, complete: bool) -> Self {
        Self {
            result,
            steps,
            complete,
        }
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

#[cfg(test)]
mod verification_adversarial_tests {
    use super::*;

    #[test]
    fn rejects_a_forged_distribution_and_false_completion_flag() {
        let variable = Expr::from(Symbol::new("x").unwrap());
        let before = Expr::mul([variable.clone(), Expr::add([variable, Expr::from(1)])]);
        let fake = DerivationStep {
            before: before.clone(),
            after: Expr::from(42),
            rule: RuleId::DistributeProductOverSum,
            relation: Relation::Equal,
        };
        assert_eq!(
            verify_distribution_step(&fake),
            Err(VerificationError::InvalidStep)
        );
        let false_completion = DistributionResult {
            result: before.clone(),
            steps: Vec::new(),
            complete: true,
        };
        assert_eq!(
            verify_distribution(&before, &false_completion),
            Err(VerificationError::IncorrectCompleteness)
        );
        let forged_chain = DistributionResult {
            result: Expr::from(42),
            steps: vec![fake],
            complete: true,
        };
        assert_eq!(
            verify_distribution(&before, &forged_chain),
            Err(VerificationError::InvalidStep)
        );
    }
}
