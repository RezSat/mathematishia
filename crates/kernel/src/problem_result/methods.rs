//! Optional, checked native strategy results. No new mathematical algorithm.

use super::*;
use crate::{PolynomialEquationResult, RationalFactorization};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MethodId {
    LinearIsolation,
    RationalDiscriminant,
    SymbolicDerivative,
    DirectQuadratic,
    SquareFreeLinearFactor,
}

impl MethodId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::LinearIsolation => "equation.linear_isolation",
            Self::RationalDiscriminant => "equation.rational_discriminant",
            Self::SymbolicDerivative => "calculus.symbolic_derivative",
            Self::DirectQuadratic => "equation.direct_quadratic",
            Self::SquareFreeLinearFactor => "equation.square_free_linear_factor",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlternativeError {
    Problem(ProblemError),
    ResourceRefusal(ResourceLimitKind),
    NotApplicable,
    RepeatedMethod,
    Incomplete,
    DifferentResult,
}

impl From<ProblemError> for AlternativeError {
    fn from(error: ProblemError) -> Self {
        match error {
            ProblemError::Evaluation(EvaluationError::Limit(kind))
            | ProblemError::PolynomialRoots(PartialPolynomialRootError::Evaluation(
                EvaluationError::Limit(kind),
            )) => Self::ResourceRefusal(kind),
            other => Self::Problem(other),
        }
    }
}

/// Retains the actual native factorization, not an inferred sequence of steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedAlternative {
    result: ProblemResult,
    factorization: RationalFactorization,
}

impl VerifiedAlternative {
    pub const fn method(&self) -> MethodId {
        MethodId::SquareFreeLinearFactor
    }

    pub fn result(&self) -> &ProblemResult {
        &self.result
    }

    pub fn factorization(&self) -> &RationalFactorization {
        &self.factorization
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlternativeStatus {
    NotRequested,
    NotApplicable,
    PrimaryNotComplete,
    Failed(AlternativeError),
    Verified(Box<VerifiedAlternative>),
}

/// At most one optional alternative. A failed exploration does not change
/// the primary result or its completeness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodEnvelope {
    primary: ProblemResult,
    primary_method: Option<MethodId>,
    alternative: AlternativeStatus,
}

impl MethodEnvelope {
    pub fn primary(&self) -> &ProblemResult {
        &self.primary
    }

    pub const fn primary_method(&self) -> Option<MethodId> {
        self.primary_method
    }

    pub fn alternative(&self) -> &AlternativeStatus {
        &self.alternative
    }
}

fn dispatched_method(primary: &ProblemResult) -> Option<MethodId> {
    match primary.checked_by()? {
        CheckedBy::LinearEquivalence => Some(MethodId::LinearIsolation),
        CheckedBy::SymbolicDerivativeRules => Some(MethodId::SymbolicDerivative),
        CheckedBy::QuadraticRootCriterion => match &primary.request().statement {
            ProblemStatement::PolynomialZero(p) if p.degree() == Some(2) => {
                Some(MethodId::RationalDiscriminant)
            }
            _ => None,
        },
        CheckedBy::SourceBoundPolynomial => match &primary.request().statement {
            ProblemStatement::PolynomialZero(p) if p.degree() == Some(2) => {
                Some(MethodId::DirectQuadratic)
            }
            _ => None,
        },
        _ => None,
    }
}

fn complete(primary: &ProblemResult) -> bool {
    primary.checked_by().is_some()
        && matches!(
            primary.outcome(),
            ProblemOutcome::SolvedExact(_)
                | ProblemOutcome::SolvedConditional(_)
                | ProblemOutcome::NoSolutions
                | ProblemOutcome::AllAdmissibleValues
        )
}

fn eligible(primary: &ProblemResult) -> Option<&RationalPolynomial> {
    let request = primary.request();
    if request.operation != ProblemOperation::PolynomialEquation
        || request.domain != ScalarDomain::Complex
        || request.variables.len() != 1
        || primary.checked_by() != Some(CheckedBy::SourceBoundPolynomial)
        || !primary.restrictions().is_empty()
    {
        return None;
    }
    let ProblemStatement::PolynomialZero(polynomial) = &request.statement else {
        return None;
    };
    let ProblemOutcome::SolvedExact(ExactAnswer::ComplexRoots(roots)) = primary.outcome() else {
        return None;
    };
    if polynomial.degree() == Some(2)
        && matches!(roots.as_slice(), [root] if root.multiplicity() == 2 && root.root().as_rational().is_some())
    {
        Some(polynomial)
    } else {
        None
    }
}

fn verified_alternative(
    context: &EvaluationContext,
    primary: &ProblemResult,
    evidence: ProblemEvidence<'_>,
) -> Result<VerifiedAlternative, AlternativeError> {
    let candidate = match evidence {
        ProblemEvidence::ComplexRoots(candidate) => candidate,
        ProblemEvidence::PolynomialEquation(_) => return Err(AlternativeError::RepeatedMethod),
        _ => return Err(AlternativeError::NotApplicable),
    };
    if !complete(primary) {
        return Err(AlternativeError::Incomplete);
    }
    if eligible(primary).is_none() {
        return Err(AlternativeError::NotApplicable);
    }
    let mut request = primary.request().clone();
    request.operation = ProblemOperation::ComplexPolynomialRoots;
    let result =
        adapt_verified_problem(context, &request, ProblemEvidence::ComplexRoots(candidate))?;
    if let ProblemOutcome::ResourceRefusal(kind) = result.outcome() {
        return Err(AlternativeError::ResourceRefusal(*kind));
    }
    if !complete(&result) {
        return Err(AlternativeError::Incomplete);
    }
    if result.outcome() != primary.outcome()
        || result.result_domain() != primary.result_domain()
        || result.restrictions() != primary.restrictions()
    {
        return Err(AlternativeError::DifferentResult);
    }
    let factorization = candidate
        .factorization()
        .ok_or(AlternativeError::Incomplete)?;
    if !factorization.is_complete()
        || !matches!(factorization.factors(), [factor] if factor.polynomial().degree() == Some(1) && factor.multiplicity() == 2)
        || !matches!(candidate.root_evidence(), [proof] if proof.factor().degree() == Some(1) && proof.multiplicity() == 2)
    {
        return Err(AlternativeError::NotApplicable);
    }
    Ok(VerifiedAlternative {
        result,
        factorization: factorization.clone(),
    })
}

impl ProblemResult {
    /// The normal dispatcher remains the fast path. When requested, at most one
    /// independently checked native alternative is attempted.
    pub fn dispatch_with_methods(
        context: &EvaluationContext,
        request: &ProblemRequest,
        include_alternative: bool,
    ) -> Result<MethodEnvelope, ProblemError> {
        let primary = dispatch_problem(context, request)?;
        let primary_method = dispatched_method(&primary);
        let alternative = if !include_alternative {
            AlternativeStatus::NotRequested
        } else if !complete(&primary) {
            AlternativeStatus::PrimaryNotComplete
        } else if let Some(polynomial) = eligible(&primary) {
            match context
                .partial_polynomial_roots(polynomial)
                .map_err(ProblemError::PolynomialRoots)
                .map_err(AlternativeError::from)
                .and_then(|candidate| {
                    verified_alternative(
                        context,
                        &primary,
                        ProblemEvidence::ComplexRoots(&candidate),
                    )
                }) {
                Ok(result) => AlternativeStatus::Verified(Box::new(result)),
                Err(error) => AlternativeStatus::Failed(error),
            }
        } else {
            AlternativeStatus::NotApplicable
        };
        Ok(MethodEnvelope {
            primary,
            primary_method,
            alternative,
        })
    }

    /// Checks externally supplied direct-quadratic and square-free evidence
    /// against the same original request. A caller cannot assert method IDs.
    pub fn adapt_with_alternative(
        context: &EvaluationContext,
        request: &ProblemRequest,
        direct: &PolynomialEquationResult,
        alternative: ProblemEvidence<'_>,
    ) -> Result<MethodEnvelope, AlternativeError> {
        let primary = adapt_verified_problem(
            context,
            request,
            ProblemEvidence::PolynomialEquation(direct),
        )?;
        if let ProblemOutcome::ResourceRefusal(kind) = primary.outcome() {
            return Err(AlternativeError::ResourceRefusal(*kind));
        }
        if !complete(&primary) {
            return Err(AlternativeError::Incomplete);
        }
        if eligible(&primary).is_none()
            || !matches!(direct.certificate.factors.as_slice(), [proof] if proof.original().degree() == Some(2))
        {
            return Err(AlternativeError::NotApplicable);
        }
        let alternative = verified_alternative(context, &primary, alternative)?;
        Ok(MethodEnvelope {
            primary,
            primary_method: Some(MethodId::DirectQuadratic),
            alternative: AlternativeStatus::Verified(Box::new(alternative)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CertifiedSolutionSet, PolynomialZeroSet, solve_polynomial_equation};

    fn request(coefficients: &[i64]) -> ProblemRequest {
        ProblemRequest {
            operation: ProblemOperation::PolynomialEquation,
            statement: ProblemStatement::PolynomialZero(RationalPolynomial::new(
                coefficients
                    .iter()
                    .map(|&n| Rational::from(mathema::Integer::from(n)))
                    .collect(),
            )),
            variables: vec![Symbol::new("x").unwrap()],
            domain: ScalarDomain::Complex,
        }
    }

    fn polynomial(request: &ProblemRequest) -> &RationalPolynomial {
        let ProblemStatement::PolynomialZero(polynomial) = &request.statement else {
            panic!("expected polynomial");
        };
        polynomial
    }

    #[test]
    fn legacy_fast_path_and_verified_alternative() {
        let context = EvaluationContext::default();
        let request = request(&[18, -12, 2]);
        let legacy = dispatch_problem(&context, &request).unwrap();
        let fast = ProblemResult::dispatch_with_methods(&context, &request, false).unwrap();
        assert_eq!(fast.primary(), &legacy);
        assert_eq!(fast.alternative(), &AlternativeStatus::NotRequested);
        assert_eq!(fast.primary_method(), Some(MethodId::DirectQuadratic));
        let checked = ProblemResult::dispatch_with_methods(&context, &request, true).unwrap();
        assert_eq!(checked.primary(), &legacy);
        let AlternativeStatus::Verified(alternative) = checked.alternative() else {
            panic!("expected checked alternate");
        };
        assert_eq!(alternative.result().outcome(), legacy.outcome());
        let ProblemOutcome::SolvedExact(ExactAnswer::ComplexRoots(roots)) = legacy.outcome() else {
            panic!("expected exact roots");
        };
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].multiplicity(), 2);
        assert_eq!(
            roots[0].root().as_rational(),
            Some(&Rational::from(mathema::Integer::from(3)))
        );
        assert_eq!(
            alternative.factorization().unit(),
            &Rational::from(mathema::Integer::from(2))
        );
        assert_eq!(alternative.method(), MethodId::SquareFreeLinearFactor);
        assert!(alternative.result().steps().is_empty());
        let json = checked.to_json().unwrap();
        assert!(json.contains("\"status\":\"verified\""));
        assert!(json.contains("\"method\":\"equation.square_free_linear_factor\""));
        assert!(json.contains(&legacy.to_json().unwrap()));
        assert_eq!(
            json,
            ProblemResult::dispatch_with_methods(&context, &request, true)
                .unwrap()
                .to_json()
                .unwrap()
        );
    }

    #[test]
    fn rational_repeated_root_and_other_cases() {
        let context = EvaluationContext::default();
        let repeated = request(&[1, -4, 4]);
        let result = ProblemResult::dispatch_with_methods(&context, &repeated, true).unwrap();
        assert!(matches!(
            result.alternative(),
            AlternativeStatus::Verified(_)
        ));
        for coefficients in [&[6, -5, 1][..], &[1, 0, 1], &[1], &[0], &[1, 1]] {
            let request = request(coefficients);
            let result = ProblemResult::dispatch_with_methods(&context, &request, true).unwrap();
            assert_eq!(result.alternative(), &AlternativeStatus::NotApplicable);
        }
        let mut rational = repeated;
        rational.domain = ScalarDomain::Rational;
        let result = ProblemResult::dispatch_with_methods(&context, &rational, true).unwrap();
        assert_eq!(result.alternative(), &AlternativeStatus::NotApplicable);
        rational.domain = ScalarDomain::Integer;
        let result = ProblemResult::dispatch_with_methods(&context, &rational, true).unwrap();
        assert_eq!(result.alternative(), &AlternativeStatus::PrimaryNotComplete);
    }

    #[test]
    fn rejects_forged_wrong_source_and_repeated_evidence() {
        let context = EvaluationContext::default();
        let original = request(&[9, -6, 1]);
        let p = polynomial(&original);
        let direct =
            solve_polynomial_equation(&context, p, &original.variables[0], original.domain)
                .unwrap();
        let alternative = context.partial_polynomial_roots(p).unwrap();
        let adapt = |candidate| {
            ProblemResult::adapt_with_alternative(&context, &original, &direct, candidate)
        };
        assert!(adapt(ProblemEvidence::ComplexRoots(&alternative)).is_ok());
        assert_eq!(
            adapt(ProblemEvidence::PolynomialEquation(&direct)),
            Err(AlternativeError::RepeatedMethod)
        );
        let wrong = context
            .partial_polynomial_roots(polynomial(&request(&[4, -4, 1])))
            .unwrap();
        assert!(adapt(ProblemEvidence::ComplexRoots(&wrong)).is_err());
        let forged = PartialPolynomialRootResult::from_unverified_parts(
            p.clone(),
            PolynomialZeroSet::Finite {
                roots: Vec::new(),
                unresolved_cofactor: None,
                complete: true,
            },
            alternative.factorization().cloned(),
            alternative.root_evidence().to_vec(),
        );
        assert!(adapt(ProblemEvidence::ComplexRoots(&forged)).is_err());
        let mut forged_direct = direct.clone();
        forged_direct.certificate.solutions = CertifiedSolutionSet::None;
        assert!(
            ProblemResult::adapt_with_alternative(
                &context,
                &original,
                &forged_direct,
                ProblemEvidence::ComplexRoots(&alternative),
            )
            .is_err()
        );
    }
}
