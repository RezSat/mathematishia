//! Exact equation dispatch for normalized, unrestricted Q[x] polynomials.

use std::fmt;

use mathema::{DefinednessError, DomainExpression, ScalarDomain, Symbol};

use crate::budget::WorkBudget;
use crate::{
    CertifiedSolutionSet, EvaluationContext, EvaluationError, ExactQuadraticRoot,
    PartialPolynomialRootError, PartialPolynomialRootResult, PolynomialZeroSet, QuadraticBranch,
    RationalPolynomial, SolutionCertificateError, SolutionSetCertificate, certify_solution_set,
    solve_partial_polynomial_roots, verify_partial_polynomial_roots, verify_solution_set,
};

/// The normalized source and its checked solution set in an explicit field.
/// Higher-degree inputs retain the complex factor-search evidence, even when
/// some of its roots are outside the requested field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialEquationResult {
    pub original: RationalPolynomial,
    pub certificate: SolutionSetCertificate,
    pub complex_search: Option<PartialPolynomialRootResult>,
}

impl PolynomialEquationResult {
    pub fn solutions(&self) -> &CertifiedSolutionSet {
        &self.certificate.solutions
    }

    pub fn is_complete(&self) -> bool {
        !matches!(self.solutions(), CertifiedSolutionSet::Partial { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolynomialEquationError {
    Evaluation(EvaluationError),
    Definedness(DefinednessError),
    FactorSearch(PartialPolynomialRootError),
    Certificate(SolutionCertificateError),
    UnsupportedDomain,
    IncorrectCertificate,
}

impl fmt::Display for PolynomialEquationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Definedness(error) => write!(f, "{error}"),
            Self::FactorSearch(error) => write!(f, "{error}"),
            Self::Certificate(error) => write!(f, "{error}"),
            Self::UnsupportedDomain => f.write_str("polynomial equations require Q, R or C"),
            Self::IncorrectCertificate => f.write_str("inconsistent polynomial equation evidence"),
        }
    }
}

impl std::error::Error for PolynomialEquationError {}

impl From<EvaluationError> for PolynomialEquationError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl From<PartialPolynomialRootError> for PolynomialEquationError {
    fn from(error: PartialPolynomialRootError) -> Self {
        match error {
            PartialPolynomialRootError::Evaluation(error) => Self::Evaluation(error),
            other => Self::FactorSearch(other),
        }
    }
}

impl From<SolutionCertificateError> for PolynomialEquationError {
    fn from(error: SolutionCertificateError) -> Self {
        match error {
            SolutionCertificateError::Evaluation(error) => Self::Evaluation(error),
            other => Self::Certificate(other),
        }
    }
}

fn operands(
    context: &EvaluationContext,
    original: &RationalPolynomial,
    variable: &Symbol,
    domain: ScalarDomain,
) -> Result<(DomainExpression, DomainExpression), PolynomialEquationError> {
    if domain == ScalarDomain::Integer {
        return Err(PolynomialEquationError::UnsupportedDomain);
    }
    context.check_polynomial(original)?;
    WorkBudget::new(context.limits()).charge(1)?;
    let expression = original.to_expr(variable);
    context.check_expression(&expression)?;
    let limit = context.limits().max_expression_nodes;
    let left = DomainExpression::from_expr(expression, limit)
        .map_err(PolynomialEquationError::Definedness)?;
    let right = DomainExpression::from_expr(0.into(), limit)
        .map_err(PolynomialEquationError::Definedness)?;
    Ok((left, right))
}

fn in_domain(root: &ExactQuadraticRoot, domain: ScalarDomain) -> bool {
    match domain {
        ScalarDomain::Rational => root.as_rational().is_some(),
        ScalarDomain::Real => !matches!(
            root.branch(),
            Some(QuadraticBranch::NegativeImaginary | QuadraticBranch::PositiveImaginary)
        ),
        ScalarDomain::Complex => true,
        ScalarDomain::Integer => false,
    }
}

// The search retains a monic cofactor, whereas the source-bound certificate
// retains the residual scalar unit. Their zero sets must agree exactly.
fn agrees_with_search(
    context: &EvaluationContext,
    search: &PartialPolynomialRootResult,
    certificate: &SolutionSetCertificate,
) -> Result<bool, PolynomialEquationError> {
    let PolynomialZeroSet::Finite {
        roots,
        unresolved_cofactor,
        complete,
    } = search.zero_set()
    else {
        return Ok(false);
    };
    let admitted = roots
        .iter()
        .filter(|entry| in_domain(entry.root(), certificate.domain));
    match (&certificate.solutions, unresolved_cofactor) {
        (CertifiedSolutionSet::None, None) => Ok(*complete && admitted.count() == 0),
        (CertifiedSolutionSet::Finite(values), None) => {
            Ok(*complete && admitted.eq(values.iter()))
        }
        (
            CertifiedSolutionSet::Partial {
                roots: values,
                unresolved_cofactor: residual,
            },
            Some(cofactor),
        ) => {
            if *complete || !admitted.eq(values.iter()) {
                return Ok(false);
            }
            let (quotient, remainder) = context.divide(residual, cofactor)?;
            Ok(remainder.is_zero() && quotient.degree() == Some(0))
        }
        _ => Ok(false),
    }
}

/// Solves P(x) = 0 over Q, R or C. Input must already be an unrestricted
/// normalized polynomial, not the numerator of a domain-restricted expression.
/// Degree at most two is handled directly; higher degrees use bounded rational
/// factor search. Unhandled factors remain partial in every field, even when
/// irreducibility over Q is known. No approximate or general algebraic roots
/// are returned.
pub fn solve_polynomial_equation(
    context: &EvaluationContext,
    original: &RationalPolynomial,
    variable: &Symbol,
    domain: ScalarDomain,
) -> Result<PolynomialEquationResult, PolynomialEquationError> {
    let (left, right) = operands(context, original, variable, domain)?;
    let complex_search = if original.degree().is_some_and(|degree| degree > 2) {
        Some(solve_partial_polynomial_roots(context, original)?)
    } else {
        None
    };
    let hints = complex_search.as_ref().map_or_else(Vec::new, |search| {
        search
            .root_evidence()
            .iter()
            .map(|proof| proof.factor().clone())
            .collect()
    });
    let certificate = certify_solution_set(context, &left, &right, variable, domain, &hints)?;
    if !verify_solution_set(context, &left, &right, variable, domain, &certificate)? {
        return Err(PolynomialEquationError::IncorrectCertificate);
    }
    if let Some(search) = &complex_search
        && !agrees_with_search(context, search, &certificate)?
    {
        return Err(PolynomialEquationError::IncorrectCertificate);
    }
    Ok(PolynomialEquationResult {
        original: original.clone(),
        certificate,
        complex_search,
    })
}

/// Checks the external source, field, factor-search result and source-bound
/// root certificate. A partial claim cannot omit roots found by the supported
/// factor search. Does not invoke the equation dispatcher.
pub fn verify_polynomial_equation(
    context: &EvaluationContext,
    original: &RationalPolynomial,
    variable: &Symbol,
    domain: ScalarDomain,
    proposed: &PolynomialEquationResult,
) -> Result<bool, PolynomialEquationError> {
    let (left, right) = operands(context, original, variable, domain)?;
    if proposed.original != *original
        || !verify_solution_set(context, &left, &right, variable, domain, &proposed.certificate)?
    {
        return Ok(false);
    }
    match (
        original.degree().is_some_and(|degree| degree > 2),
        &proposed.complex_search,
    ) {
        (false, None) => Ok(true),
        (true, Some(search)) => {
            if !verify_partial_polynomial_roots(context, original, search)? {
                return Ok(false);
            }
            agrees_with_search(context, search, &proposed.certificate)
        }
        _ => Ok(false),
    }
}
