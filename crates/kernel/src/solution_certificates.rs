//! Source-bound solution-set certificates for univariate rational equations.
//!
//! Root proofs cover linear and quadratic factors over Q, interpreted over the
//! requested scalar domain. Higher-degree cofactors remain unresolved unless
//! the caller supplies exact supported factors. No factor search is performed.

use std::fmt;

use mathema::{DomainExpression, Integer, Rational, ScalarDomain, Symbol};

use crate::budget::WorkBudget;
use crate::{
    AlgebraicRootEvidence, AlgebraicRootResult, AlgebraicRootSet, EvaluationContext,
    EvaluationError, ExactQuadraticRoot, PolynomialRootMultiplicity, QuadraticBranch,
    RationalExpressionError, RationalFunction, RationalPolynomial, verify_algebraic_roots,
};

/// A complete empty or finite set, a punctured scalar domain, or an incomplete
/// set. Multiplicity is in the reduced residual numerator, at admissible roots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertifiedSolutionSet {
    None,
    Finite(Vec<PolynomialRootMultiplicity>),
    AllAdmissible,
    Partial {
        roots: Vec<PolynomialRootMultiplicity>,
        unresolved_cofactor: RationalPolynomial,
    },
}

/// Proposed evidence is not trusted until verified against the external source.
/// AllAdmissible and Partial retain the original exclusions in residual.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolutionSetCertificate {
    pub left: DomainExpression,
    pub right: DomainExpression,
    pub variable: Symbol,
    pub domain: ScalarDomain,
    pub residual: RationalFunction,
    pub factors: Vec<AlgebraicRootResult>,
    pub solutions: CertifiedSolutionSet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolutionCertificateError {
    Evaluation(EvaluationError),
    Expression(RationalExpressionError),
    UnsupportedDomain,
    IncorrectCertificate,
}

impl fmt::Display for SolutionCertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Expression(error) => write!(f, "{error}"),
            Self::UnsupportedDomain => {
                f.write_str("unsupported or mismatched solution-certificate domain")
            }
            Self::IncorrectCertificate => f.write_str("incorrect solution-set certificate"),
        }
    }
}

impl std::error::Error for SolutionCertificateError {}

impl From<EvaluationError> for SolutionCertificateError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl From<RationalExpressionError> for SolutionCertificateError {
    fn from(error: RationalExpressionError) -> Self {
        match error {
            RationalExpressionError::Evaluation(error) => Self::Evaluation(error),
            other => Self::Expression(other),
        }
    }
}

fn check_domain(domain: ScalarDomain) -> Result<(), SolutionCertificateError> {
    match domain {
        ScalarDomain::Rational | ScalarDomain::Real | ScalarDomain::Complex => Ok(()),
        ScalarDomain::Integer => Err(SolutionCertificateError::UnsupportedDomain),
    }
}

fn check_source_domain(
    left: &DomainExpression,
    right: &DomainExpression,
    domain: ScalarDomain,
) -> Result<(), SolutionCertificateError> {
    check_domain(domain)?;
    for source in [left, right] {
        source
            .check_construction_domain(domain)
            .map_err(|_| SolutionCertificateError::UnsupportedDomain)?;
    }
    Ok(())
}

// Check the reduced residual against the original partial functions without
// rerunning rational-function subtraction or trusting its cancellation.
fn check_residual(
    work: &mut WorkBudget,
    left: &RationalFunction,
    right: &RationalFunction,
    residual: &RationalFunction,
) -> Result<(), SolutionCertificateError> {
    let mut exclusions = left.excluded_zero_sets().to_vec();
    exclusions.extend_from_slice(right.excluded_zero_sets());
    work.charge(exclusions.len() as u64)?;
    exclusions.sort_by(crate::rational_function::condition_order);
    exclusions.dedup();
    if exclusions != residual.excluded_zero_sets() {
        return Err(SolutionCertificateError::IncorrectCertificate);
    }

    let common = work.multiply(left.denominator(), right.denominator())?;
    let (scale, remainder) = work.divide(&common, residual.denominator())?;
    if scale.is_zero() || !remainder.is_zero() {
        return Err(SolutionCertificateError::IncorrectCertificate);
    }
    let lhs = work.multiply(left.numerator(), right.denominator())?;
    let rhs = work.multiply(right.numerator(), left.denominator())?;
    let difference = work.subtract(&lhs, &rhs)?;
    if work.multiply(residual.numerator(), &scale)? != difference {
        return Err(SolutionCertificateError::IncorrectCertificate);
    }
    Ok(())
}

fn in_domain(root: &ExactQuadraticRoot, domain: ScalarDomain) -> bool {
    match domain {
        ScalarDomain::Complex => true,
        ScalarDomain::Real => !matches!(
            root.branch(),
            Some(QuadraticBranch::NegativeImaginary | QuadraticBranch::PositiveImaginary)
        ),
        ScalarDomain::Rational => root.as_rational().is_some(),
        ScalarDomain::Integer => false,
    }
}

fn minimal(root: &ExactQuadraticRoot) -> RationalPolynomial {
    match root {
        ExactQuadraticRoot::Rational(value) => {
            RationalPolynomial::new(vec![-value, Rational::from(Integer::one())])
        }
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial, ..
        } => minimal_polynomial.clone(),
    }
}

// A rational polynomial vanishes at an exact quadratic root precisely when
// its irreducible minimal polynomial divides it.
fn vanishes(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
    root: &ExactQuadraticRoot,
) -> Result<bool, EvaluationError> {
    match root {
        ExactQuadraticRoot::Rational(value) => {
            Ok(work.evaluate(polynomial, value)?.numerator().is_zero())
        }
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial, ..
        } => Ok(work.divide(polynomial, minimal_polynomial)?.1.is_zero()),
    }
}

fn admissible(
    work: &mut WorkBudget,
    side: &RationalFunction,
    root: &ExactQuadraticRoot,
) -> Result<bool, EvaluationError> {
    for restriction in side.excluded_zero_sets().iter().chain([side.denominator()]) {
        if vanishes(work, restriction, root)? {
            return Ok(false);
        }
    }
    Ok(true)
}

// Compare original operand values, not merely the residual numerator. For a
// quadratic root, evaluate in Q[t]/(minimal) without approximating either branch.
fn equal_at(
    work: &mut WorkBudget,
    left: &RationalFunction,
    right: &RationalFunction,
    root: &ExactQuadraticRoot,
) -> Result<bool, EvaluationError> {
    match root {
        ExactQuadraticRoot::Rational(value) => {
            let ln = work.evaluate(left.numerator(), value)?;
            let ld = work.evaluate(left.denominator(), value)?;
            let rn = work.evaluate(right.numerator(), value)?;
            let rd = work.evaluate(right.denominator(), value)?;
            Ok(work.multiply_scalar(&ln, &rd)? == work.multiply_scalar(&rn, &ld)?)
        }
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial, ..
        } => {
            let ln = work.divide(left.numerator(), minimal_polynomial)?.1;
            let ld = work.divide(left.denominator(), minimal_polynomial)?.1;
            let rn = work.divide(right.numerator(), minimal_polynomial)?.1;
            let rd = work.divide(right.denominator(), minimal_polynomial)?.1;
            let a = work.multiply(&ln, &rd)?;
            let b = work.multiply(&rn, &ld)?;
            let difference = work.subtract(&a, &b)?;
            Ok(work.divide(&difference, minimal_polynomial)?.1.is_zero())
        }
    }
}

fn divide_factor(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
    factor: &RationalPolynomial,
) -> Result<RationalPolynomial, SolutionCertificateError> {
    if !matches!(factor.degree(), Some(1 | 2)) {
        return Err(SolutionCertificateError::IncorrectCertificate);
    }
    let (quotient, remainder) = work.divide(polynomial, factor)?;
    if !remainder.is_zero() || polynomial.is_zero() {
        return Err(SolutionCertificateError::IncorrectCertificate);
    }
    Ok(quotient)
}

fn remove_powers(
    work: &mut WorkBudget,
    polynomial: &mut RationalPolynomial,
    factor: &RationalPolynomial,
) -> Result<usize, EvaluationError> {
    let mut count = 0;
    while polynomial.degree() >= factor.degree() {
        let (quotient, remainder) = work.divide(polynomial, factor)?;
        if !remainder.is_zero() {
            break;
        }
        *polynomial = quotient;
        count += 1;
    }
    Ok(count)
}

fn classify(
    context: &EvaluationContext,
    left: &RationalFunction,
    right: &RationalFunction,
    residual: &RationalFunction,
    domain: ScalarDomain,
    factors: &[AlgebraicRootResult],
) -> Result<CertifiedSolutionSet, SolutionCertificateError> {
    use SolutionCertificateError::IncorrectCertificate;
    let mut work = WorkBudget::new(context.limits());
    work.charge(1)?;
    check_residual(&mut work, left, right, residual)?;
    let numerator = residual.numerator();
    let Some(degree) = numerator.degree() else {
        return if factors.is_empty() {
            Ok(CertifiedSolutionSet::AllAdmissible)
        } else {
            Err(IncorrectCertificate)
        };
    };
    if factors.len() > degree {
        return Err(IncorrectCertificate);
    }
    let mut remaining = numerator.clone();
    let mut candidates = Vec::new();
    for proof in factors {
        work.charge(128)?;
        context.check_polynomial(proof.original())?;
        remaining = divide_factor(&mut work, &remaining, proof.original())?;
        // Preflight the bounded degree-two arithmetic before invoking the
        // existing independent reference checker.
        crate::algebraic::checked_algebraic_roots(proof.original(), &mut work)?;
        verify_algebraic_roots(proof).map_err(|_| IncorrectCertificate)?;
        for item in proof.evidence() {
            match item {
                AlgebraicRootEvidence::MonicNormalization(poly) => {
                    context.check_polynomial(poly)?
                }
                AlgebraicRootEvidence::Discriminant(value) => context.check_rational(value)?,
                AlgebraicRootEvidence::Classification(_) => {}
            }
        }
        let AlgebraicRootSet::Finite(roots) = proof.roots() else {
            return Err(IncorrectCertificate);
        };
        for root in roots {
            match root {
                ExactQuadraticRoot::Rational(value) => context.check_rational(value)?,
                ExactQuadraticRoot::Algebraic {
                    minimal_polynomial, ..
                } => {
                    context.check_polynomial(minimal_polynomial)?;
                }
            }
            if !candidates.contains(root) {
                candidates.push(root.clone());
            }
        }
    }

    let mut solutions = Vec::new();
    // Removing each distinct minimal polynomial to its full multiplicity also
    // removes any additional copies in the unresolved cofactor. Conjugates
    // share one minimal polynomial and therefore one multiplicity.
    let mut unaccounted = numerator.clone();
    let mut multiplicities = Vec::new();
    for root in candidates {
        let factor = minimal(&root);
        let multiplicity =
            if let Some((_, count)) = multiplicities.iter().find(|(p, _)| p == &factor) {
                *count
            } else {
                let count = remove_powers(&mut work, &mut unaccounted, &factor)?;
                if count == 0 {
                    return Err(IncorrectCertificate);
                }
                multiplicities.push((factor, count));
                count
            };
        if in_domain(&root, domain)
            && admissible(&mut work, left, &root)?
            && admissible(&mut work, right, &root)?
        {
            if !equal_at(&mut work, left, right, &root)? {
                return Err(IncorrectCertificate);
            }
            solutions.push(PolynomialRootMultiplicity::new(root, multiplicity));
        }
    }
    // Root order is canonical presentation order, not an ordering on C.
    solutions.sort_by(|a, b| match (a.root(), b.root()) {
        (ExactQuadraticRoot::Rational(a), ExactQuadraticRoot::Rational(b)) => a.cmp(b),
        (ExactQuadraticRoot::Rational(_), _) => std::cmp::Ordering::Less,
        (_, ExactQuadraticRoot::Rational(_)) => std::cmp::Ordering::Greater,
        (
            ExactQuadraticRoot::Algebraic {
                minimal_polynomial: a,
                branch: ab,
            },
            ExactQuadraticRoot::Algebraic {
                minimal_polynomial: b,
                branch: bb,
            },
        ) => a
            .coefficients()
            .cmp(b.coefficients())
            .then_with(|| ab.stable_id().cmp(bb.stable_id())),
    });
    context.check_polynomial(&unaccounted)?;
    if unaccounted.degree() != Some(0) {
        Ok(CertifiedSolutionSet::Partial {
            roots: solutions,
            unresolved_cofactor: unaccounted,
        })
    } else if solutions.is_empty() {
        Ok(CertifiedSolutionSet::None)
    } else {
        Ok(CertifiedSolutionSet::Finite(solutions))
    }
}

/// Certifies a rational equation using optional exact linear/quadratic factor
/// hints of its reduced numerator. Hints must divide successively; any final
/// degree-one/two cofactor is handled automatically. Higher degrees are partial.
/// Unsupported expressions/conditions (including arbitrary parameter families)
/// produce typed errors, not empty solution sets.
/// A bound operand must match the requested scalar domain. Unbound operands
/// retain their explicit restrictions, but cannot recover erased source history.
pub fn certify_solution_set(
    context: &EvaluationContext,
    left: &DomainExpression,
    right: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    factor_hints: &[RationalPolynomial],
) -> Result<SolutionSetCertificate, SolutionCertificateError> {
    check_source_domain(left, right, domain)?;
    let lhs = context.rational_expression(left, variable)?;
    let rhs = context.rational_expression(right, variable)?;
    let residual = context.subtract_rational_functions(&lhs, &rhs)?;
    if factor_hints.len() > residual.numerator().degree().unwrap_or(0) {
        return Err(SolutionCertificateError::IncorrectCertificate);
    }
    let mut work = WorkBudget::new(context.limits());
    work.charge(1)?;
    let mut remaining = residual.numerator().clone();
    let mut factors = Vec::new();
    for factor in factor_hints {
        context.check_polynomial(factor)?;
        remaining = divide_factor(&mut work, &remaining, factor)?;
        work.charge(128)?;
        factors.push(context.algebraic_roots(factor)?);
    }
    if matches!(remaining.degree(), Some(1 | 2)) {
        factors.push(context.algebraic_roots(&remaining)?);
    }
    let mut solutions = classify(context, &lhs, &rhs, &residual, domain, &factors)?;
    // A hint may account for repeated roots and leave a small cofactor after
    // stripping their full multiplicities.
    if let CertifiedSolutionSet::Partial {
        unresolved_cofactor,
        ..
    } = &solutions
        && matches!(unresolved_cofactor.degree(), Some(1 | 2))
    {
        factors.push(context.algebraic_roots(unresolved_cofactor)?);
        solutions = classify(context, &lhs, &rhs, &residual, domain, &factors)?;
    }
    Ok(SolutionSetCertificate {
        left: left.clone(),
        right: right.clone(),
        variable: variable.clone(),
        domain,
        residual,
        factors,
        solutions,
    })
}

/// Verifies evidence against externally supplied source operands and domain.
/// Does not invoke the solution-set generator or a factor-search algorithm.
pub fn verify_solution_set(
    context: &EvaluationContext,
    left: &DomainExpression,
    right: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    proposed: &SolutionSetCertificate,
) -> Result<bool, SolutionCertificateError> {
    check_source_domain(left, right, domain)?;
    let lhs = context.rational_expression(left, variable)?;
    let rhs = context.rational_expression(right, variable)?;
    let residual = context.subtract_rational_functions(&lhs, &rhs)?;
    if proposed.left != *left
        || proposed.right != *right
        || proposed.variable != *variable
        || proposed.domain != domain
        || proposed.residual != residual
    {
        return Ok(false);
    }
    match classify(context, &lhs, &rhs, &residual, domain, &proposed.factors) {
        Ok(CertifiedSolutionSet::Partial {
            unresolved_cofactor,
            ..
        }) if matches!(unresolved_cofactor.degree(), Some(1 | 2)) => Ok(false),
        Ok(expected) => Ok(expected == proposed.solutions),
        Err(SolutionCertificateError::IncorrectCertificate) => Ok(false),
        Err(error) => Err(error),
    }
}
