//! Bounded, certified expansion in an explicitly chosen multivariate Q ring.
//!
//! Exact expansion uses the existing sparse polynomial bridge. Unsupported
//! expressions and resource refusals never masquerade as completed results.

use std::error::Error;
use std::fmt;

use mathema::{DomainExpression, Expr, Symbol};

use crate::{MultivariateError, MultivariateExprError, MultivariateLimits, MultivariatePolynomial};

/// A complete expansion retains its source, inherited conditions and ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialExpansion {
    source: DomainExpression,
    expanded: DomainExpression,
    polynomial: MultivariatePolynomial,
}

impl PolynomialExpansion {
    pub fn source(&self) -> &DomainExpression {
        &self.source
    }

    pub fn expanded(&self) -> &DomainExpression {
        &self.expanded
    }

    pub fn polynomial(&self) -> &MultivariatePolynomial {
        &self.polynomial
    }

    pub fn variables(&self) -> &[Symbol] {
        self.polynomial.variables()
    }

    /// Exactness means full polynomial expansion, not textual equality.
    pub fn complete(&self) -> bool {
        true
    }

    /// An already-expanded input may be algebraically equal and structurally unchanged.
    pub fn syntactically_unchanged(&self) -> bool {
        self.source.expression() == self.expanded.expression()
    }
}

/// Unsupported forms and exhausted budgets are distinct from successful work.
/// No partial expansion is emitted: the bridge either computes all terms or refuses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolynomialExpansionOutcome {
    Exact(PolynomialExpansion),
    Unsupported(MultivariateExprError),
    ResourceLimited(MultivariateExprError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolynomialExpansionError {
    Conversion(MultivariateExprError),
}

impl fmt::Display for PolynomialExpansionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conversion(error) => write!(f, "{error}"),
        }
    }
}

impl Error for PolynomialExpansionError {}

fn resource_refusal(error: MultivariateExprError) -> bool {
    matches!(
        error,
        MultivariateExprError::ExponentLimit
            | MultivariateExprError::ExpressionDepthLimit
            | MultivariateExprError::ExpressionNodeLimit
            | MultivariateExprError::ConditionLimit
            | MultivariateExprError::Polynomial(
                MultivariateError::VariableLimit
                    | MultivariateError::TotalDegreeLimit
                    | MultivariateError::TermLimit
                    | MultivariateError::InputTermLimit
                    | MultivariateError::ScalarBitLimit
                    | MultivariateError::WorkLimit
            )
    )
}

/// Fully expands supported positive-integer powers, sums and products.
/// All coefficients are exact rationals, and the ambient ring is explicit.
///
/// Historical source-domain obligations survive verbatim; they are not proved
/// or removed by the transformation. The source must be a DomainExpression
/// when normalization may have erased earlier undefined subexpressions.
pub fn expand_polynomial(
    source: &DomainExpression,
    variables: &[Symbol],
    limits: MultivariateLimits,
) -> PolynomialExpansionOutcome {
    let converted = MultivariatePolynomial::try_from_domain_expression(source, variables, limits);
    let converted = match converted {
        Ok(value) => value,
        Err(error) if resource_refusal(error) => {
            return PolynomialExpansionOutcome::ResourceLimited(error);
        }
        Err(error) => return PolynomialExpansionOutcome::Unsupported(error),
    };

    let polynomial = converted.polynomial().clone();
    let expanded = match converted.to_domain_expression(limits) {
        Ok(value) => value,
        Err(error) if resource_refusal(error) => {
            return PolynomialExpansionOutcome::ResourceLimited(error);
        }
        Err(error) => return PolynomialExpansionOutcome::Unsupported(error),
    };
    PolynomialExpansionOutcome::Exact(PolynomialExpansion {
        source: source.clone(),
        expanded,
        polynomial,
    })
}

/// Independently recomputes exact polynomials from both expressions.
///
/// Also checks the source, ring and inherited condition sequence against the
/// certificate. It rejects forged results even when the text looks plausible.
/// An invalid equality returns false; inadmissible inputs return typed errors.
pub fn verify_polynomial_expansion(
    source: &DomainExpression,
    candidate: &PolynomialExpansion,
    variables: &[Symbol],
    limits: MultivariateLimits,
) -> Result<bool, PolynomialExpansionError> {
    if source != candidate.source() || source.conditions() != candidate.expanded().conditions() {
        return Ok(false);
    }

    let original = MultivariatePolynomial::try_from_domain_expression(source, variables, limits)
        .map_err(PolynomialExpansionError::Conversion)?;
    let output =
        MultivariatePolynomial::try_from_domain_expression(candidate.expanded(), variables, limits)
            .map_err(PolynomialExpansionError::Conversion)?;
    if original.polynomial().variables() != candidate.variables()
        || output.polynomial().variables() != candidate.variables()
    {
        return Ok(false);
    }

    // The certificate is a polynomial identity, not an AST/string comparison.
    // Require an expanded canonical representation as well as equality.
    let canonical: Expr = candidate
        .polynomial()
        .to_expr(limits)
        .map_err(PolynomialExpansionError::Conversion)?;
    Ok(original.polynomial() == candidate.polynomial()
        && output.polynomial() == candidate.polynomial()
        && candidate.expanded().expression() == &canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathema::{Condition, Expr};

    #[test]
    fn independently_rejects_forged_coefficients_outputs_and_conditions() {
        let x = Expr::from(Symbol::new("x").unwrap());
        let ring = [Symbol::new("x").unwrap()];
        let source = DomainExpression::from_expr(
            Expr::pow(Expr::add([x.clone(), Expr::from(1)]), Expr::from(2)),
            128,
        )
        .unwrap();
        let PolynomialExpansionOutcome::Exact(good) =
            expand_polynomial(&source, &ring, MultivariateLimits::default())
        else {
            panic!("expected expansion");
        };
        let mut false_coefficient = good.clone();
        false_coefficient.polynomial = MultivariatePolynomial::try_from_expr(
            &Expr::add([Expr::pow(x.clone(), Expr::from(2)), Expr::from(1)]),
            &ring,
            MultivariateLimits::default(),
        )
        .unwrap();
        assert_eq!(
            verify_polynomial_expansion(
                &source,
                &false_coefficient,
                &ring,
                MultivariateLimits::default()
            ),
            Ok(false)
        );

        let mut forged_output = good.clone();
        forged_output.expanded = DomainExpression::from_expr(Expr::from(42), 128).unwrap();
        assert_eq!(
            verify_polynomial_expansion(
                &source,
                &forged_output,
                &ring,
                MultivariateLimits::default()
            ),
            Ok(false)
        );

        let mut erased_hole = good.clone();
        erased_hole.expanded = DomainExpression::with_conditions(
            good.expanded.expression().clone(),
            [Condition::NonZero(x)],
            128,
            16,
        )
        .unwrap();
        assert_eq!(
            verify_polynomial_expansion(
                &source,
                &erased_hole,
                &ring,
                MultivariateLimits::default()
            ),
            Ok(false)
        );
    }
}
