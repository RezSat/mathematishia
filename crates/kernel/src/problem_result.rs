//! Typed, domain-explicit results for existing checked mathematical operations.
//!
//! This module does not parse source text or infer missing source restrictions.
//! Inputs are already-normalized mathematical statements; conditions inherited
//! from construction must be carried by `DomainExpression` or `RationalFunction`.

pub mod methods;
mod native;
mod wire;
pub use wire::{ProblemWireError, SchoolError, checked_infinity_json, checked_periodic_json, checked_piecewise_json, checked_school_json, checked_sqrt_json, checked_trig_identity_json};

use std::error::Error;
use std::fmt;

use mathema::{
    Condition, DefinednessError, DomainExpression, Equation, Expr, ExprView, Rational,
    ScalarDomain, Symbol,
};

use crate::{
    AbsoluteEquation, AbsoluteEquationResult, GroebnerCertificate, IdealMembership, PiMultiple,
    SpecialAngleCertificate, TrigFunction,
};
use crate::{
    AnalyticConstantError, AnalyticConstantLimit, AnalyticConstantLimits, AnalyticConstantOutcome,
    Assumptions, ComparisonOperator, ConditionStatus, DispatchedAntiderivative,
    ElementaryIntegrationError, EvaluationContext, EvaluationError, InequalityError,
    InequalityResult, IntegrationDispatchError, IntegrationError, LimitDirection, LimitError,
    LimitValue, LimitVerificationError, LinearSolution, LinearSolveResult, MultivariateError,
    MultivariateExprError, MultivariateLimits, PartialPolynomialRootError,
    PartialPolynomialRootResult, PolynomialExpansion, PolynomialExpansionError,
    PolynomialExpansionOutcome, PolynomialRootMultiplicity, PolynomialZeroSet, QuadraticResult,
    QuadraticRoots, RationalEquationResult, RationalEquationSolution, RationalExpressionError,
    RationalFunction, RationalFunctionError, RationalLimit, RationalPolynomial,
    RationalQuadraticRoots, RealInterval, ResourceLimitKind, SymbolicDerivative,
    SymbolicDifferentiationError, expand_polynomial, rewrite_analytic_constant_once,
    verify_analytic_constant_rewrite, verify_linear_solution, verify_partial_polynomial_roots,
    verify_polynomial_expansion, verify_quadratic_roots, verify_rational_equation,
    verify_rational_limit, verify_rational_quadratic_equation, verify_symbolic_derivative,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProblemOperation {
    SolveLinear,
    SolveRational,
    RationalRoots,
    ComplexPolynomialRoots,
    PolynomialEquation,
    Differentiate,
    EvaluateRational,
    Integrate,
    Expand,
    Limit,
    Inequality,
    SolveAuto,
    SolveSourceEquation,
    EvaluateExact,
    SolveAbsolute,
    EvaluateSpecialAngle,
    TrigZeros,
    GroebnerBasis,
    IdealMembership,
}

impl ProblemOperation {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::SolveLinear => "equation.linear",
            Self::SolveRational => "equation.rational_linear",
            Self::RationalRoots => "polynomial.rational_roots_degree_two",
            Self::ComplexPolynomialRoots => "polynomial.complex_roots_partial",
            Self::PolynomialEquation => "polynomial.equation",
            Self::Differentiate => "calculus.differentiate",
            Self::EvaluateRational => "rational.evaluate",
            Self::Integrate => "calculus.integrate",
            Self::Expand => "polynomial.expand",
            Self::Limit => "calculus.rational_limit",
            Self::Inequality => "inequality.rational",
            Self::SolveAuto => "equation.auto",
            Self::SolveSourceEquation => "equation.source_bound",
            Self::EvaluateExact => "analytic.evaluate_exact",
            Self::SolveAbsolute => "equation.absolute_affine",
            Self::EvaluateSpecialAngle => "trig.special_angle",
            Self::TrigZeros => "trig.zeros_partial",
            Self::GroebnerBasis => "polynomial.groebner_basis",
            Self::IdealMembership => "polynomial.ideal_membership",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProblemStatement {
    Equation(Equation),
    PolynomialZero(RationalPolynomial),
    Absolute(Box<AbsoluteEquation>),
    SpecialAngle {
        angle: PiMultiple,
        function: TrigFunction,
    },
    TrigZeros(TrigFunction),
    Ideal {
        generators: Vec<DomainExpression>,
        target: Option<DomainExpression>,
    },
    RationalEquation {
        left: RationalFunction,
        right: RationalFunction,
    },
    RationalEvaluation {
        function: RationalFunction,
        at: Rational,
    },
    Expression(DomainExpression),
    Limit {
        source: DomainExpression,
        at: Rational,
        direction: LimitDirection,
    },
    SourceEquation {
        left: DomainExpression,
        right: DomainExpression,
    },
    Comparison {
        left: DomainExpression,
        right: DomainExpression,
        operator: ComparisonOperator,
    },
}

/// The selected variable and ambient domain are explicit. Polynomial
/// coefficients are exact rationals even when roots are sought over C.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemRequest {
    pub operation: ProblemOperation,
    pub statement: ProblemStatement,
    pub variables: Vec<Symbol>,
    pub domain: ScalarDomain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProblemRestriction {
    Condition(Condition),
    PolynomialNonZero(RationalPolynomial),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactAnswer {
    RationalRoots(Vec<Rational>),
    ComplexRoots(Vec<PolynomialRootMultiplicity>),
    Expression(Expr),
    Rational(Rational),
    Limit(LimitValue),
    Intervals(Vec<RealInterval>),
    AllAdmissibleValues,
    Absolute(AbsoluteEquationResult),
    SpecialAngle(Box<SpecialAngleCertificate>),
    TrigZeros(Vec<SpecialAngleCertificate>),
    Groebner(Box<GroebnerCertificate>),
    Membership {
        basis: Box<GroebnerCertificate>,
        membership: Box<IdealMembership>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason {
    Domain,
    SeveralVariables,
    AlgorithmNotAvailable,
    ExpressionOutsideAlgorithm,
    DegreeOutsideAlgorithm,
    RewriteStepLimit,
    PeriodicCompletenessUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndefinedReason {
    ExcludedPoint,
    ContradictedDomainCondition,
    TangentPole,
}

/// `NoSolutions` is always relative to `result_domain`; in particular a
/// quadratic without rational roots is not claimed root-free over R or C.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProblemOutcome {
    SolvedExact(ExactAnswer),
    SolvedConditional(ExactAnswer),
    Partial {
        known: ExactAnswer,
        unresolved: RationalPolynomial,
    },
    PartialScope {
        known: ExactAnswer,
        reason: UnsupportedReason,
    },
    Unsupported(UnsupportedReason),
    NoSolutions,
    AllAdmissibleValues,
    Undefined(UndefinedReason),
    ResourceRefusal(ResourceLimitKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckedBy {
    LinearEquivalence,
    QuadraticRootCriterion,
    RationalEquationDomain,
    PartialComplexFactorization,
    SourceBoundPolynomial,
    SourceBoundRationalEquation,
    SymbolicDerivativeRules,
    RationalEvaluation,
    IntegrationTerms,
    PolynomialIdentity,
    RationalLimitFactorization,
    RationalInequalitySigns,
    RationalQuadraticEquation,
    AnalyticConstantRules,
    RationalLiteral,
    AbsoluteSignCases,
    SpecialAngleIdentity,
    TrigZeroSubstitution,
    GroebnerCriterion,
    IdealReduction,
}

/// An index into a verified source proof, not a reconstructed prose step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepReference {
    pub index: usize,
    pub rule_id: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProblemResult {
    request: ProblemRequest,
    result_domain: ScalarDomain,
    outcome: ProblemOutcome,
    restrictions: Vec<ProblemRestriction>,
    proof: Option<CheckedBy>,
    detail: Option<ExactAnswer>,
    steps: Vec<StepReference>,
}

impl ProblemResult {
    pub fn request(&self) -> &ProblemRequest {
        &self.request
    }

    pub const fn result_domain(&self) -> ScalarDomain {
        self.result_domain
    }

    pub fn outcome(&self) -> &ProblemOutcome {
        &self.outcome
    }

    /// Necessary source restrictions. None is asserted or silently discharged.
    pub fn restrictions(&self) -> &[ProblemRestriction] {
        &self.restrictions
    }

    /// Only successful independent verification produces a proof marker.
    pub const fn checked_by(&self) -> Option<CheckedBy> {
        self.proof
    }

    /// Checked evidence retained for outcomes without an answer (for example a pole).
    pub fn evidence_detail(&self) -> Option<&ExactAnswer> {
        self.detail.as_ref()
    }

    pub fn steps(&self) -> &[StepReference] {
        &self.steps
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProblemError {
    MissingVariable,
    StatementMismatch,
    InvalidEvidence,
    MissingDomainRestrictions,
    Evaluation(EvaluationError),
    PolynomialRoots(PartialPolynomialRootError),
}

impl fmt::Display for ProblemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVariable => f.write_str("one selected variable is required"),
            Self::StatementMismatch => f.write_str("operation does not match the statement"),
            Self::InvalidEvidence => f.write_str("mathematical evidence failed verification"),
            Self::MissingDomainRestrictions => f.write_str("source-domain obligations are missing"),
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::PolynomialRoots(error) => write!(f, "{error}"),
        }
    }
}

impl Error for ProblemError {}

/// Evidence from supported algorithms. Callers cannot assert a checked result
/// by constructing `ProblemResult`; adapters verify evidence before publication.
#[derive(Clone, Copy, Debug)]
pub enum ProblemEvidence<'a> {
    Linear(&'a LinearSolveResult),
    Quadratic(&'a QuadraticResult),
    RationalEquation(&'a RationalEquationResult),
    ComplexRoots(&'a PartialPolynomialRootResult),
    PolynomialEquation(&'a crate::PolynomialEquationResult),
    SourceEquation(&'a crate::SolutionSetCertificate),
    Derivative(&'a SymbolicDerivative),
    RationalEvaluation(&'a Rational),
    Integration(&'a DispatchedAntiderivative),
    Expansion(&'a PolynomialExpansion),
    Limit(&'a RationalLimit),
    Inequality(&'a InequalityResult),
    Absolute(&'a AbsoluteEquationResult),
    SpecialAngle(&'a SpecialAngleCertificate),
    TrigZeros(&'a [SpecialAngleCertificate]),
    Groebner(&'a GroebnerCertificate),
    Membership(&'a GroebnerCertificate, &'a IdealMembership),
}

fn restrictions(statement: &ProblemStatement) -> Vec<ProblemRestriction> {
    match statement {
        ProblemStatement::Expression(source) | ProblemStatement::Limit { source, .. } => source
            .conditions()
            .iter()
            .cloned()
            .map(ProblemRestriction::Condition)
            .collect(),
        ProblemStatement::RationalEquation { left, right } => {
            let mut conditions = Vec::new();
            for polynomial in left
                .excluded_zero_sets()
                .iter()
                .chain(right.excluded_zero_sets())
            {
                let restriction = ProblemRestriction::PolynomialNonZero(polynomial.clone());
                if !conditions.contains(&restriction) {
                    conditions.push(restriction);
                }
            }
            conditions
        }
        ProblemStatement::RationalEvaluation { function, .. } => function
            .excluded_zero_sets()
            .iter()
            .cloned()
            .map(ProblemRestriction::PolynomialNonZero)
            .collect(),
        ProblemStatement::Comparison { left, right, .. }
        | ProblemStatement::SourceEquation { left, right } => {
            let mut conditions = Vec::new();
            for condition in left.conditions().iter().chain(right.conditions()) {
                let restriction = ProblemRestriction::Condition(condition.clone());
                if !conditions.contains(&restriction) {
                    conditions.push(restriction);
                }
            }
            conditions
        }
        ProblemStatement::Ideal { generators, target } => {
            let mut conditions = Vec::new();
            for source in generators.iter().chain(target.iter()) {
                for condition in source.conditions() {
                    let restriction = ProblemRestriction::Condition(condition.clone());
                    if !conditions.contains(&restriction) {
                        conditions.push(restriction);
                    }
                }
            }
            conditions
        }
        ProblemStatement::Equation(_)
        | ProblemStatement::PolynomialZero(_)
        | ProblemStatement::Absolute(_)
        | ProblemStatement::SpecialAngle { .. }
        | ProblemStatement::TrigZeros(_) => Vec::new(),
    }
}

fn result(request: &ProblemRequest, outcome: ProblemOutcome) -> ProblemResult {
    ProblemResult {
        request: request.clone(),
        result_domain: request.domain,
        outcome,
        restrictions: restrictions(&request.statement),
        proof: None,
        detail: None,
        steps: Vec::new(),
    }
}

fn unsupported(request: &ProblemRequest, reason: UnsupportedReason) -> ProblemResult {
    result(request, ProblemOutcome::Unsupported(reason))
}

fn handle_evaluation_error(
    request: &ProblemRequest,
    error: EvaluationError,
) -> Result<ProblemResult, ProblemError> {
    let outcome = match error {
        EvaluationError::Limit(kind) => ProblemOutcome::ResourceRefusal(kind),
        EvaluationError::Linear(crate::LinearSolveError::NonlinearEquation)
        | EvaluationError::Quadratic(crate::QuadraticError::DegreeExceeded)
        | EvaluationError::RationalEquation(crate::RationalEquationError::NonlinearNumerator) => {
            ProblemOutcome::Unsupported(UnsupportedReason::DegreeOutsideAlgorithm)
        }
        EvaluationError::Polynomial(crate::PolynomialConversionError::DegreeLimitExceeded) => {
            ProblemOutcome::ResourceRefusal(ResourceLimitKind::PolynomialDegree)
        }
        EvaluationError::Polynomial(_)
        | EvaluationError::Linear(crate::LinearSolveError::UnsupportedExpression(_))
        | EvaluationError::SymbolicDifferentiation(
            SymbolicDifferentiationError::UnsupportedPower
            | SymbolicDifferentiationError::UnsupportedFunction,
        ) => ProblemOutcome::Unsupported(UnsupportedReason::ExpressionOutsideAlgorithm),
        EvaluationError::SymbolicDifferentiation(
            SymbolicDifferentiationError::MissingDomainCondition,
        ) => return Err(ProblemError::MissingDomainRestrictions),
        EvaluationError::RationalFunction(RationalFunctionError::UndefinedAtPoint) => {
            ProblemOutcome::Undefined(UndefinedReason::ExcludedPoint)
        }
        EvaluationError::RationalFunction(
            RationalFunctionError::ZeroDenominator
            | RationalFunctionError::ZeroDivisor
            | RationalFunctionError::EmptyDomain,
        ) => ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
        other => return Err(ProblemError::Evaluation(other)),
    };
    Ok(result(request, outcome))
}

fn source_is_refuted(source: &DomainExpression) -> bool {
    let assumptions = Assumptions::new();
    source
        .conditions()
        .iter()
        .any(|condition| assumptions.assess(condition) == ConditionStatus::Refuted)
}

fn conditional_answer(answer: ExactAnswer, conditions: &[Condition]) -> ProblemOutcome {
    let assumptions = Assumptions::new();
    if conditions
        .iter()
        .all(|condition| assumptions.assess(condition) == ConditionStatus::Proven)
    {
        ProblemOutcome::SolvedExact(answer)
    } else {
        ProblemOutcome::SolvedConditional(answer)
    }
}

fn with_conditions(mut published: ProblemResult, conditions: &[Condition]) -> ProblemResult {
    for condition in conditions {
        let restriction = ProblemRestriction::Condition(condition.clone());
        if !published.restrictions.contains(&restriction) {
            published.restrictions.push(restriction);
        }
    }
    published
}

fn with_exclusions(
    mut published: ProblemResult,
    exclusions: &[RationalPolynomial],
) -> ProblemResult {
    for polynomial in exclusions {
        let restriction = ProblemRestriction::PolynomialNonZero(polynomial.clone());
        if !published.restrictions.contains(&restriction) {
            published.restrictions.push(restriction);
        }
    }
    published
}

fn refusal(
    request: &ProblemRequest,
    kind: ResourceLimitKind,
) -> Result<ProblemResult, ProblemError> {
    Ok(result(request, ProblemOutcome::ResourceRefusal(kind)))
}

fn definedness_failure(
    request: &ProblemRequest,
    error: DefinednessError,
) -> Result<ProblemResult, ProblemError> {
    match error {
        DefinednessError::NodeLimitExceeded => refusal(request, ResourceLimitKind::ExpressionNodes),
        DefinednessError::ConditionLimitExceeded => {
            refusal(request, ResourceLimitKind::RationalConditions)
        }
        DefinednessError::ZeroDenominator => Ok(result(
            request,
            ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
        )),
        _ => Ok(unsupported(
            request,
            UnsupportedReason::ExpressionOutsideAlgorithm,
        )),
    }
}

fn integration_failure(
    request: &ProblemRequest,
    error: IntegrationDispatchError,
) -> Result<ProblemResult, ProblemError> {
    use ElementaryIntegrationError as E;
    use IntegrationDispatchError as I;
    match error {
        I::Evaluation(error)
        | I::Polynomial(IntegrationError::Evaluation(error))
        | I::Elementary(E::Evaluation(error)) => handle_evaluation_error(request, error),
        I::Definedness(error) | I::Elementary(E::Definedness(error)) => {
            definedness_failure(request, error)
        }
        I::TooManyTerms => refusal(request, ResourceLimitKind::ExpressionNodes),
        I::MissingDomainCondition | I::Elementary(E::MissingDomainCondition) => {
            Err(ProblemError::MissingDomainRestrictions)
        }
        I::UnsupportedScalarDomain | I::Elementary(E::UnsupportedScalarDomain) => {
            Ok(unsupported(request, UnsupportedReason::Domain))
        }
        I::Polynomial(IntegrationError::UnsupportedIntegrand(error))
        | I::Elementary(E::UnsupportedArgument(error)) => {
            handle_evaluation_error(request, EvaluationError::Polynomial(error))
        }
        I::Polynomial(IntegrationError::RestrictedDomain)
        | I::RestrictedDefiniteDomain
        | I::UnsupportedAnalyticDefiniteIntegral
        | I::Elementary(
            E::UnsupportedForm
            | E::UnsupportedFunction
            | E::NonAffineArgument
            | E::ZeroSlope
            | E::UnsupportedNormalization,
        ) => Ok(unsupported(
            request,
            UnsupportedReason::ExpressionOutsideAlgorithm,
        )),
        _ => Err(ProblemError::InvalidEvidence),
    }
}

fn conversion_failure(
    request: &ProblemRequest,
    error: RationalExpressionError,
) -> Result<ProblemResult, ProblemError> {
    match error {
        RationalExpressionError::Evaluation(error) => handle_evaluation_error(request, error),
        RationalExpressionError::ExponentLimitExceeded => {
            refusal(request, ResourceLimitKind::PolynomialDegree)
        }
        RationalExpressionError::RationalFunction(
            RationalFunctionError::ZeroDenominator
            | RationalFunctionError::ZeroDivisor
            | RationalFunctionError::EmptyDomain,
        ) => Ok(result(
            request,
            ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
        )),
        RationalExpressionError::RationalFunction(_) => Err(ProblemError::InvalidEvidence),
        _ => Ok(unsupported(
            request,
            UnsupportedReason::ExpressionOutsideAlgorithm,
        )),
    }
}

fn inequality_failure(
    request: &ProblemRequest,
    error: InequalityError,
) -> Result<ProblemResult, ProblemError> {
    match error {
        InequalityError::Evaluation(error) => handle_evaluation_error(request, error),
        InequalityError::Conversion(error) => conversion_failure(request, error),
        InequalityError::UnsupportedDegree => Ok(unsupported(
            request,
            UnsupportedReason::DegreeOutsideAlgorithm,
        )),
        InequalityError::IncorrectCertificate => Err(ProblemError::InvalidEvidence),
    }
}

fn expansion_limits(context: &EvaluationContext) -> MultivariateLimits {
    let configured = context.limits();
    let limits = MultivariateLimits::default();
    MultivariateLimits {
        max_variables: limits.max_variables.min(configured.max_expression_nodes),
        max_terms: limits.max_terms.min(configured.max_expression_nodes),
        max_input_terms: limits.max_input_terms.min(configured.max_expression_nodes),
        max_total_degree: limits
            .max_total_degree
            .min(u32::try_from(configured.max_polynomial_degree).unwrap_or(u32::MAX)),
        max_scalar_bits: limits
            .max_scalar_bits
            .min(configured.max_scalar_bits)
            .min(configured.max_intermediate_scalar_bits),
        max_work_units: limits.max_work_units.min(configured.max_work_units),
    }
}

fn expansion_failure(
    request: &ProblemRequest,
    error: MultivariateExprError,
) -> Result<ProblemResult, ProblemError> {
    use MultivariateError as M;
    use MultivariateExprError as E;
    match error {
        E::ExponentLimit | E::Polynomial(M::TotalDegreeLimit) => {
            refusal(request, ResourceLimitKind::PolynomialDegree)
        }
        E::ExpressionDepthLimit => refusal(request, ResourceLimitKind::ExpressionDepth),
        E::ExpressionNodeLimit
        | E::Polynomial(M::VariableLimit | M::TermLimit | M::InputTermLimit) => {
            refusal(request, ResourceLimitKind::ExpressionNodes)
        }
        E::ConditionLimit => refusal(request, ResourceLimitKind::RationalConditions),
        E::Polynomial(M::ScalarBitLimit) => refusal(request, ResourceLimitKind::ScalarBits),
        E::Polynomial(M::WorkLimit) => refusal(request, ResourceLimitKind::WorkUnits),
        E::Domain(error) => definedness_failure(request, error),
        _ => Ok(unsupported(
            request,
            UnsupportedReason::ExpressionOutsideAlgorithm,
        )),
    }
}

fn admit_source(
    context: &EvaluationContext,
    source: &DomainExpression,
) -> Result<(), EvaluationError> {
    context.check_expression(source.expression())?;
    if source.conditions().len() > context.limits().max_rational_conditions {
        return Err(EvaluationError::Limit(
            ResourceLimitKind::RationalConditions,
        ));
    }
    for condition in source.conditions() {
        context.check_expression(condition.expression())?;
    }
    Ok(())
}

fn admit_request(
    context: &EvaluationContext,
    request: &ProblemRequest,
) -> Result<(), EvaluationError> {
    let limits = context.limits();
    if request.variables.len() > limits.max_expression_nodes {
        return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes));
    }
    match &request.statement {
        ProblemStatement::Absolute(_)
        | ProblemStatement::SpecialAngle { .. }
        | ProblemStatement::TrigZeros(_)
        | ProblemStatement::Ideal { .. } => native::admit(context, &request.statement),
        ProblemStatement::Equation(equation) => context.check_equation(equation),
        ProblemStatement::PolynomialZero(polynomial) => context.check_polynomial(polynomial),
        ProblemStatement::RationalEquation { left, right } => {
            context.check_rational_function(left)?;
            context.check_rational_function(right)
        }
        ProblemStatement::RationalEvaluation { function, at } => {
            context.check_rational_function(function)?;
            context.check_rational(at)
        }
        ProblemStatement::Expression(source) => admit_source(context, source),
        ProblemStatement::Limit { source, at, .. } => {
            admit_source(context, source)?;
            context.check_rational(at)
        }
        ProblemStatement::Comparison { left, right, .. }
        | ProblemStatement::SourceEquation { left, right } => {
            admit_source(context, left)?;
            admit_source(context, right)
        }
    }
}

fn validate_request(request: &ProblemRequest) -> Result<Option<ProblemResult>, ProblemError> {
    let matches_domain =
        |source: &DomainExpression| source.check_construction_domain(request.domain).is_ok();
    let compatible = match &request.statement {
        ProblemStatement::Expression(source) | ProblemStatement::Limit { source, .. } => {
            matches_domain(source)
        }
        ProblemStatement::SourceEquation { left, right }
        | ProblemStatement::Comparison { left, right, .. } => {
            matches_domain(left) && matches_domain(right)
        }
        ProblemStatement::Ideal { generators, target } => {
            generators.iter().chain(target.iter()).all(matches_domain)
        }
        _ => true,
    };
    if !compatible {
        return Ok(Some(unsupported(request, UnsupportedReason::Domain)));
    }
    if request.variables.is_empty()
        && !matches!(
            request.operation,
            ProblemOperation::Expand
                | ProblemOperation::EvaluateExact
                | ProblemOperation::EvaluateSpecialAngle
                | ProblemOperation::GroebnerBasis
                | ProblemOperation::IdealMembership
        )
    {
        return Err(ProblemError::MissingVariable);
    }
    if match request.operation {
        ProblemOperation::Expand
        | ProblemOperation::GroebnerBasis
        | ProblemOperation::IdealMembership => false,
        ProblemOperation::EvaluateExact | ProblemOperation::EvaluateSpecialAngle => {
            !request.variables.is_empty()
        }
        _ => request.variables.len() != 1,
    } {
        return Ok(Some(unsupported(
            request,
            UnsupportedReason::SeveralVariables,
        )));
    }
    let matched = matches!(
        (&request.operation, &request.statement),
        (
            ProblemOperation::SolveAbsolute,
            ProblemStatement::Absolute(_)
        ) | (
            ProblemOperation::EvaluateSpecialAngle,
            ProblemStatement::SpecialAngle { .. }
        ) | (ProblemOperation::TrigZeros, ProblemStatement::TrigZeros(_))
            | (
                ProblemOperation::GroebnerBasis,
                ProblemStatement::Ideal { target: None, .. }
            )
            | (
                ProblemOperation::IdealMembership,
                ProblemStatement::Ideal {
                    target: Some(_),
                    ..
                }
            )
            | (ProblemOperation::SolveLinear, ProblemStatement::Equation(_))
            | (
                ProblemOperation::SolveRational,
                ProblemStatement::RationalEquation { .. }
            )
            | (
                ProblemOperation::RationalRoots,
                ProblemStatement::PolynomialZero(_)
            )
            | (
                ProblemOperation::ComplexPolynomialRoots | ProblemOperation::PolynomialEquation,
                ProblemStatement::PolynomialZero(_)
            )
            | (
                ProblemOperation::Differentiate,
                ProblemStatement::Expression(_)
            )
            | (
                ProblemOperation::EvaluateRational,
                ProblemStatement::RationalEvaluation { .. }
            )
            | (
                ProblemOperation::Integrate
                    | ProblemOperation::Expand
                    | ProblemOperation::EvaluateExact,
                ProblemStatement::Expression(_)
            )
            | (ProblemOperation::Limit, ProblemStatement::Limit { .. })
            | (
                ProblemOperation::Inequality,
                ProblemStatement::Comparison { .. }
            )
            | (
                ProblemOperation::SolveAuto | ProblemOperation::SolveSourceEquation,
                ProblemStatement::SourceEquation { .. }
            )
    );
    if !matched {
        return Err(ProblemError::StatementMismatch);
    }
    let domain_supported = match request.operation {
        ProblemOperation::SolveLinear
        | ProblemOperation::SolveRational
        | ProblemOperation::RationalRoots
        | ProblemOperation::EvaluateRational => request.domain == ScalarDomain::Rational,
        ProblemOperation::ComplexPolynomialRoots => request.domain == ScalarDomain::Complex,
        ProblemOperation::PolynomialEquation => matches!(
            request.domain,
            ScalarDomain::Rational | ScalarDomain::Real | ScalarDomain::Complex
        ),
        ProblemOperation::Differentiate => {
            matches!(request.domain, ScalarDomain::Real | ScalarDomain::Complex)
        }
        ProblemOperation::Integrate => matches!(
            request.domain,
            ScalarDomain::Rational | ScalarDomain::Real | ScalarDomain::Complex
        ),
        ProblemOperation::Expand => matches!(
            request.domain,
            ScalarDomain::Rational | ScalarDomain::Real | ScalarDomain::Complex
        ),
        ProblemOperation::Limit | ProblemOperation::Inequality => {
            request.domain == ScalarDomain::Real
        }
        ProblemOperation::SolveAuto => matches!(
            request.domain,
            ScalarDomain::Rational | ScalarDomain::Complex
        ),
        ProblemOperation::SolveSourceEquation => matches!(
            request.domain,
            ScalarDomain::Rational | ScalarDomain::Real | ScalarDomain::Complex
        ),
        ProblemOperation::EvaluateExact
        | ProblemOperation::SolveAbsolute
        | ProblemOperation::EvaluateSpecialAngle
        | ProblemOperation::TrigZeros => request.domain == ScalarDomain::Real,
        ProblemOperation::GroebnerBasis | ProblemOperation::IdealMembership => {
            request.domain == ScalarDomain::Rational
        }
    };
    if !domain_supported {
        return Ok(Some(unsupported(request, UnsupportedReason::Domain)));
    }
    if let ProblemStatement::Absolute(equation) = &request.statement
        && (equation.domain != request.domain
            || request.variables.first() != Some(&equation.left.variable)
            || equation.left.variable != equation.right.variable)
    {
        return Err(ProblemError::StatementMismatch);
    }
    Ok(None)
}

fn checked_result(
    request: &ProblemRequest,
    outcome: ProblemOutcome,
    proof: CheckedBy,
    steps: Vec<StepReference>,
) -> ProblemResult {
    let mut published = result(request, outcome);
    published.proof = Some(proof);
    published.steps = steps;
    published
}

/// Validate an externally supplied algorithm result against the original
/// statement, ambient domain and independent verifier. Invalid certificates
/// cannot acquire a proof marker through this adapter.
pub fn adapt_verified_problem(
    context: &EvaluationContext,
    request: &ProblemRequest,
    evidence: ProblemEvidence<'_>,
) -> Result<ProblemResult, ProblemError> {
    if let Err(error) = admit_request(context, request) {
        return handle_evaluation_error(request, error);
    }
    if let Some(rejection) = validate_request(request)? {
        return Ok(rejection);
    }
    if native::is_native(request.operation) {
        return native::adapt(context, request, evidence);
    }
    let variable = request.variables.first();
    let limits = context.limits();
    let (outcome, proof, steps) = match (&request.operation, &request.statement, evidence) {
        (
            ProblemOperation::SolveLinear,
            ProblemStatement::Equation(equation),
            ProblemEvidence::Linear(candidate),
        ) => {
            context
                .check_equation(equation)
                .map_err(ProblemError::Evaluation)?;
            if candidate.original() != equation
                || candidate.steps().len() > limits.max_expression_nodes
            {
                return Err(ProblemError::InvalidEvidence);
            }
            context
                .check_polynomial(candidate.residual())
                .map_err(ProblemError::Evaluation)?;
            for step in candidate.steps() {
                context
                    .check_equation(step.before())
                    .map_err(ProblemError::Evaluation)?;
                context
                    .check_equation(step.after())
                    .map_err(ProblemError::Evaluation)?;
            }
            verify_linear_solution(candidate, variable.expect("validated variable"))
                .map_err(|_| ProblemError::InvalidEvidence)?;
            let outcome = match candidate.solution() {
                LinearSolution::Unique(value) => {
                    context
                        .check_rational(value)
                        .map_err(ProblemError::Evaluation)?;
                    ProblemOutcome::SolvedExact(ExactAnswer::RationalRoots(vec![value.clone()]))
                }
                LinearSolution::NoSolution => ProblemOutcome::NoSolutions,
                LinearSolution::AllRationals => ProblemOutcome::AllAdmissibleValues,
            };
            let steps = candidate
                .steps()
                .iter()
                .enumerate()
                .map(|(index, step)| StepReference {
                    index,
                    rule_id: step.rule().stable_id(),
                })
                .collect();
            (outcome, CheckedBy::LinearEquivalence, steps)
        }
        (
            ProblemOperation::RationalRoots,
            ProblemStatement::PolynomialZero(polynomial),
            ProblemEvidence::Quadratic(candidate),
        ) => {
            context
                .check_polynomial(polynomial)
                .map_err(ProblemError::Evaluation)?;
            if candidate.original() != polynomial || verify_quadratic_roots(candidate).is_err() {
                return Err(ProblemError::InvalidEvidence);
            }
            let outcome = match candidate.roots() {
                QuadraticRoots::AllRationals => ProblemOutcome::AllAdmissibleValues,
                QuadraticRoots::Finite(values) if values.is_empty() => ProblemOutcome::NoSolutions,
                QuadraticRoots::Finite(values) => {
                    for value in values {
                        context
                            .check_rational(value)
                            .map_err(ProblemError::Evaluation)?;
                    }
                    ProblemOutcome::SolvedExact(ExactAnswer::RationalRoots(values.clone()))
                }
            };
            (outcome, CheckedBy::QuadraticRootCriterion, Vec::new())
        }
        (
            ProblemOperation::SolveRational,
            ProblemStatement::RationalEquation { left, right },
            ProblemEvidence::RationalEquation(candidate),
        ) => {
            context
                .check_rational_function(left)
                .map_err(ProblemError::Evaluation)?;
            context
                .check_rational_function(right)
                .map_err(ProblemError::Evaluation)?;
            if candidate.left() != left
                || candidate.right() != right
                || verify_rational_equation(candidate).is_err()
            {
                return Err(ProblemError::InvalidEvidence);
            }
            context
                .check_rational_function(candidate.residual())
                .map_err(ProblemError::Evaluation)?;
            let outcome = match candidate.solution() {
                RationalEquationSolution::Unique(value) => {
                    context
                        .check_rational(value)
                        .map_err(ProblemError::Evaluation)?;
                    ProblemOutcome::SolvedExact(ExactAnswer::RationalRoots(vec![value.clone()]))
                }
                RationalEquationSolution::NoSolution => ProblemOutcome::NoSolutions,
                RationalEquationSolution::AllAdmissibleRationals => {
                    ProblemOutcome::AllAdmissibleValues
                }
            };
            (outcome, CheckedBy::RationalEquationDomain, Vec::new())
        }
        (
            ProblemOperation::ComplexPolynomialRoots,
            ProblemStatement::PolynomialZero(polynomial),
            ProblemEvidence::ComplexRoots(candidate),
        ) => {
            context
                .check_polynomial(polynomial)
                .map_err(ProblemError::Evaluation)?;
            if !verify_partial_polynomial_roots(context, polynomial, candidate)
                .map_err(ProblemError::PolynomialRoots)?
            {
                return Err(ProblemError::InvalidEvidence);
            }
            let outcome = match candidate.zero_set() {
                PolynomialZeroSet::AllComplexNumbers => ProblemOutcome::AllAdmissibleValues,
                PolynomialZeroSet::Finite {
                    roots,
                    unresolved_cofactor: Some(unresolved),
                    complete: false,
                } => {
                    context
                        .check_polynomial(unresolved)
                        .map_err(ProblemError::Evaluation)?;
                    ProblemOutcome::Partial {
                        known: ExactAnswer::ComplexRoots(roots.clone()),
                        unresolved: unresolved.clone(),
                    }
                }
                PolynomialZeroSet::Finite {
                    roots,
                    unresolved_cofactor: None,
                    complete: true,
                } if roots.is_empty() => ProblemOutcome::NoSolutions,
                PolynomialZeroSet::Finite {
                    roots,
                    unresolved_cofactor: None,
                    complete: true,
                } => ProblemOutcome::SolvedExact(ExactAnswer::ComplexRoots(roots.clone())),
                _ => return Err(ProblemError::InvalidEvidence),
            };
            (outcome, CheckedBy::PartialComplexFactorization, Vec::new())
        }
        (
            ProblemOperation::PolynomialEquation,
            ProblemStatement::PolynomialZero(polynomial),
            ProblemEvidence::PolynomialEquation(candidate),
        ) => {
            use crate::{
                CertifiedSolutionSet, PolynomialEquationError, verify_polynomial_equation,
            };
            match verify_polynomial_equation(
                context,
                polynomial,
                variable.expect("validated variable"),
                request.domain,
                candidate,
            ) {
                Ok(true) => {}
                Ok(false) => return Err(ProblemError::InvalidEvidence),
                Err(PolynomialEquationError::Evaluation(error)) => {
                    return handle_evaluation_error(request, error);
                }
                Err(_) => return Err(ProblemError::InvalidEvidence),
            }
            let outcome = match candidate.solutions() {
                CertifiedSolutionSet::None => ProblemOutcome::NoSolutions,
                CertifiedSolutionSet::Finite(roots) => {
                    ProblemOutcome::SolvedExact(ExactAnswer::ComplexRoots(roots.clone()))
                }
                CertifiedSolutionSet::AllAdmissible => ProblemOutcome::AllAdmissibleValues,
                CertifiedSolutionSet::Partial {
                    roots,
                    unresolved_cofactor,
                } => ProblemOutcome::Partial {
                    known: ExactAnswer::ComplexRoots(roots.clone()),
                    unresolved: unresolved_cofactor.clone(),
                },
            };
            (outcome, CheckedBy::SourceBoundPolynomial, Vec::new())
        }
        (
            ProblemOperation::Differentiate,
            ProblemStatement::Expression(source),
            ProblemEvidence::Derivative(candidate),
        ) => {
            context
                .check_expression(source.expression())
                .map_err(ProblemError::Evaluation)?;
            if source.conditions().len() > limits.max_rational_conditions {
                return Err(ProblemError::Evaluation(EvaluationError::Limit(
                    ResourceLimitKind::RationalConditions,
                )));
            }
            if candidate.original() != source.expression()
                || Some(candidate.variable()) != variable
                || candidate.domain() != Some(request.domain)
                || candidate.conditions() != source.conditions()
                || candidate.steps().len() > limits.max_expression_nodes
                || verify_symbolic_derivative(candidate).is_err()
            {
                return Err(ProblemError::InvalidEvidence);
            }
            context
                .check_expression(candidate.derivative())
                .map_err(ProblemError::Evaluation)?;
            let conditions = Assumptions::new();
            if source
                .conditions()
                .iter()
                .any(|condition| conditions.assess(condition) == ConditionStatus::Refuted)
            {
                return Err(ProblemError::InvalidEvidence);
            }
            let conditional = source
                .conditions()
                .iter()
                .any(|condition| !matches!(conditions.assess(condition), ConditionStatus::Proven));
            let answer = ExactAnswer::Expression(candidate.derivative().clone());
            let outcome = if conditional {
                ProblemOutcome::SolvedConditional(answer)
            } else {
                ProblemOutcome::SolvedExact(answer)
            };
            let steps = candidate
                .steps()
                .iter()
                .enumerate()
                .map(|(index, step)| StepReference {
                    index,
                    rule_id: step.rule().stable_id(),
                })
                .collect();
            (outcome, CheckedBy::SymbolicDerivativeRules, steps)
        }
        (
            ProblemOperation::EvaluateRational,
            ProblemStatement::RationalEvaluation { function, at },
            ProblemEvidence::RationalEvaluation(value),
        ) => {
            let checked = context
                .evaluate_rational_function(function, at)
                .map_err(ProblemError::Evaluation)?;
            if value != &checked {
                return Err(ProblemError::InvalidEvidence);
            }
            (
                ProblemOutcome::SolvedExact(ExactAnswer::Rational(value.clone())),
                CheckedBy::RationalEvaluation,
                Vec::new(),
            )
        }
        (
            ProblemOperation::Integrate,
            ProblemStatement::Expression(source),
            ProblemEvidence::Integration(candidate),
        ) => {
            if source_is_refuted(source) {
                return Err(ProblemError::InvalidEvidence);
            }
            if let Err(error) = context.verify_dispatched_antiderivative(
                source,
                variable.expect("validated variable"),
                request.domain,
                candidate,
            ) {
                return integration_failure(request, error);
            }
            let answer = ExactAnswer::Expression(candidate.primitive().expression().clone());
            return Ok(with_conditions(
                checked_result(
                    request,
                    conditional_answer(answer, candidate.conditions()),
                    CheckedBy::IntegrationTerms,
                    Vec::new(),
                ),
                candidate.conditions(),
            ));
        }
        (
            ProblemOperation::Expand,
            ProblemStatement::Expression(source),
            ProblemEvidence::Expansion(candidate),
        ) => {
            if source_is_refuted(source) {
                return Err(ProblemError::InvalidEvidence);
            }
            if candidate.variables() != request.variables.as_slice() || candidate.source() != source
            {
                return Err(ProblemError::InvalidEvidence);
            }
            match verify_polynomial_expansion(
                source,
                candidate,
                &request.variables,
                expansion_limits(context),
            ) {
                Ok(true) => {}
                Ok(false) => return Err(ProblemError::InvalidEvidence),
                Err(PolynomialExpansionError::Conversion(error)) => {
                    return expansion_failure(request, error);
                }
            }
            if let Err(error) = admit_source(context, candidate.expanded()) {
                return handle_evaluation_error(request, error);
            }
            let answer = ExactAnswer::Expression(candidate.expanded().expression().clone());
            (
                conditional_answer(answer, source.conditions()),
                CheckedBy::PolynomialIdentity,
                Vec::new(),
            )
        }
        (
            ProblemOperation::Limit,
            ProblemStatement::Limit {
                source,
                at,
                direction,
            },
            ProblemEvidence::Limit(candidate),
        ) => {
            if source_is_refuted(source) {
                return Err(ProblemError::InvalidEvidence);
            }
            let function =
                match context.rational_expression(source, variable.expect("validated variable")) {
                    Ok(value) => value,
                    Err(error) => return conversion_failure(request, error),
                };
            match verify_rational_limit(context, &function, at, *direction, candidate) {
                Ok(()) => {}
                Err(LimitVerificationError::Evaluation(error)) => {
                    return handle_evaluation_error(request, error);
                }
                Err(_) => return Err(ProblemError::InvalidEvidence),
            }
            return Ok(with_exclusions(
                checked_result(
                    request,
                    ProblemOutcome::SolvedExact(ExactAnswer::Limit(candidate.value().clone())),
                    CheckedBy::RationalLimitFactorization,
                    Vec::new(),
                ),
                function.excluded_zero_sets(),
            ));
        }
        (
            ProblemOperation::Inequality,
            ProblemStatement::Comparison {
                left,
                right,
                operator,
            },
            ProblemEvidence::Inequality(candidate),
        ) => {
            if source_is_refuted(left) || source_is_refuted(right) {
                return Err(ProblemError::InvalidEvidence);
            }
            match context.verify_expression_inequality(
                left,
                right,
                variable.expect("validated variable"),
                *operator,
                candidate,
            ) {
                Ok(true) => {}
                Ok(false) => return Err(ProblemError::InvalidEvidence),
                Err(error) => return inequality_failure(request, error),
            }
            let outcome = if candidate.intervals().is_empty() {
                ProblemOutcome::NoSolutions
            } else {
                ProblemOutcome::SolvedExact(ExactAnswer::Intervals(candidate.intervals().to_vec()))
            };
            return Ok(with_exclusions(
                checked_result(
                    request,
                    outcome,
                    CheckedBy::RationalInequalitySigns,
                    Vec::new(),
                ),
                candidate.excluded_zero_sets(),
            ));
        }
        _ => return Err(ProblemError::StatementMismatch),
    };
    Ok(checked_result(request, outcome, proof, steps))
}

/// Dispatches only existing verified algorithms. Unsupported or exhausted work
/// returns an explicit outcome, never a guessed answer or proof trace.
pub fn dispatch_problem(
    context: &EvaluationContext,
    request: &ProblemRequest,
) -> Result<ProblemResult, ProblemError> {
    if let Err(error) = admit_request(context, request) {
        return handle_evaluation_error(request, error);
    }
    if let Some(rejection) = validate_request(request)? {
        return Ok(rejection);
    }
    if native::is_native(request.operation) {
        return native::dispatch(context, request);
    }
    let variable = request.variables.first();
    match (&request.operation, &request.statement) {
        (ProblemOperation::SolveLinear, ProblemStatement::Equation(equation)) => {
            match context.solve_linear(equation, variable.expect("validated variable")) {
                Ok(proof) => {
                    adapt_verified_problem(context, request, ProblemEvidence::Linear(&proof))
                }
                Err(error) => handle_evaluation_error(request, error),
            }
        }
        (ProblemOperation::RationalRoots, ProblemStatement::PolynomialZero(polynomial)) => {
            match context.solve_quadratic(polynomial) {
                Ok(proof) => {
                    adapt_verified_problem(context, request, ProblemEvidence::Quadratic(&proof))
                }
                Err(error) => handle_evaluation_error(request, error),
            }
        }
        (ProblemOperation::SolveRational, ProblemStatement::RationalEquation { left, right }) => {
            match context.solve_rational_equation(left, right) {
                Ok(proof) => adapt_verified_problem(
                    context,
                    request,
                    ProblemEvidence::RationalEquation(&proof),
                ),
                Err(error) => handle_evaluation_error(request, error),
            }
        }
        (
            ProblemOperation::ComplexPolynomialRoots,
            ProblemStatement::PolynomialZero(polynomial),
        ) => match context.partial_polynomial_roots(polynomial) {
            Ok(proof) => {
                adapt_verified_problem(context, request, ProblemEvidence::ComplexRoots(&proof))
            }
            Err(PartialPolynomialRootError::Evaluation(error)) => {
                handle_evaluation_error(request, error)
            }
            Err(error) => Err(ProblemError::PolynomialRoots(error)),
        },
        (ProblemOperation::PolynomialEquation, ProblemStatement::PolynomialZero(polynomial)) => {
            use crate::{PolynomialEquationError, solve_polynomial_equation};
            match solve_polynomial_equation(
                context,
                polynomial,
                variable.expect("validated variable"),
                request.domain,
            ) {
                Ok(proof) => adapt_verified_problem(
                    context,
                    request,
                    ProblemEvidence::PolynomialEquation(&proof),
                ),
                Err(PolynomialEquationError::Evaluation(error)) => {
                    handle_evaluation_error(request, error)
                }
                Err(_) => Err(ProblemError::InvalidEvidence),
            }
        }
        (ProblemOperation::Differentiate, ProblemStatement::Expression(source)) => {
            let assumptions = Assumptions::new();
            if source
                .conditions()
                .iter()
                .any(|condition| assumptions.assess(condition) == ConditionStatus::Refuted)
            {
                return Ok(result(
                    request,
                    ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
                ));
            }
            match context.differentiate_domain_expression(
                source,
                variable.expect("validated variable"),
                request.domain,
            ) {
                Ok(proof) => {
                    adapt_verified_problem(context, request, ProblemEvidence::Derivative(&proof))
                }
                Err(error) => handle_evaluation_error(request, error),
            }
        }
        (
            ProblemOperation::EvaluateRational,
            ProblemStatement::RationalEvaluation { function, at },
        ) => match context.evaluate_rational_function(function, at) {
            Ok(value) => adapt_verified_problem(
                context,
                request,
                ProblemEvidence::RationalEvaluation(&value),
            ),
            Err(error) => handle_evaluation_error(request, error),
        },
        (ProblemOperation::Integrate, ProblemStatement::Expression(source)) => {
            if source_is_refuted(source) {
                return Ok(result(
                    request,
                    ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
                ));
            }
            match context.integrate_expression_dispatch(
                source,
                variable.expect("validated variable"),
                request.domain,
            ) {
                Ok(proof) => {
                    adapt_verified_problem(context, request, ProblemEvidence::Integration(&proof))
                }
                Err(error) => integration_failure(request, error),
            }
        }
        (ProblemOperation::Expand, ProblemStatement::Expression(source)) => {
            if source_is_refuted(source) {
                return Ok(result(
                    request,
                    ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
                ));
            }
            match expand_polynomial(source, &request.variables, expansion_limits(context)) {
                PolynomialExpansionOutcome::Exact(proof) => {
                    adapt_verified_problem(context, request, ProblemEvidence::Expansion(&proof))
                }
                PolynomialExpansionOutcome::Unsupported(error)
                | PolynomialExpansionOutcome::ResourceLimited(error) => {
                    expansion_failure(request, error)
                }
            }
        }
        (
            ProblemOperation::Limit,
            ProblemStatement::Limit {
                source,
                at,
                direction,
            },
        ) => {
            if source_is_refuted(source) {
                return Ok(result(
                    request,
                    ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
                ));
            }
            match context.rational_expression_limit(
                source,
                variable.expect("validated variable"),
                at,
                *direction,
            ) {
                Ok(proof) => {
                    adapt_verified_problem(context, request, ProblemEvidence::Limit(&proof))
                }
                Err(LimitError::Evaluation(error)) => handle_evaluation_error(request, error),
                Err(LimitError::Conversion(error)) => conversion_failure(request, error),
                Err(LimitError::Verification(LimitVerificationError::Evaluation(error))) => {
                    handle_evaluation_error(request, error)
                }
                Err(_) => Err(ProblemError::InvalidEvidence),
            }
        }
        (
            ProblemOperation::Inequality,
            ProblemStatement::Comparison {
                left,
                right,
                operator,
            },
        ) => {
            if source_is_refuted(left) || source_is_refuted(right) {
                return Ok(result(
                    request,
                    ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
                ));
            }
            match context.solve_expression_inequality(
                left,
                right,
                variable.expect("validated variable"),
                *operator,
            ) {
                Ok(proof) => {
                    adapt_verified_problem(context, request, ProblemEvidence::Inequality(&proof))
                }
                Err(error) => inequality_failure(request, error),
            }
        }
        (ProblemOperation::SolveAuto, ProblemStatement::SourceEquation { left, right }) => {
            solve_auto(
                context,
                request,
                left,
                right,
                variable.expect("validated variable"),
            )
        }
        (ProblemOperation::EvaluateExact, ProblemStatement::Expression(source)) => {
            evaluate_exact(context, request, source)
        }
        _ => Err(ProblemError::StatementMismatch),
    }
}

fn solve_auto(
    context: &EvaluationContext,
    request: &ProblemRequest,
    left: &DomainExpression,
    right: &DomainExpression,
    variable: &Symbol,
) -> Result<ProblemResult, ProblemError> {
    if source_is_refuted(left) || source_is_refuted(right) {
        return Ok(result(
            request,
            ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
        ));
    }
    let (original_left, original_right) = (left, right);
    let left = match context.rational_expression(left, variable) {
        Ok(value) => value,
        Err(error) => return conversion_failure(request, error),
    };
    let right = match context.rational_expression(right, variable) {
        Ok(value) => value,
        Err(error) => return conversion_failure(request, error),
    };
    if request.domain == ScalarDomain::Complex
        && left.excluded_zero_sets().is_empty()
        && right.excluded_zero_sets().is_empty()
    {
        let residual = match context.subtract_rational_functions(&left, &right) {
            Ok(value) => value,
            Err(error) => return handle_evaluation_error(request, error),
        };
        if residual.excluded_zero_sets().is_empty() && residual.denominator().degree() == Some(0) {
            if left.denominator().degree() != Some(0)
                || right.denominator().degree() != Some(0)
                || left.numerator().subtract(right.numerator()) != *residual.numerator()
            {
                return Err(ProblemError::InvalidEvidence);
            }
            let polynomial = ProblemRequest {
                operation: ProblemOperation::ComplexPolynomialRoots,
                statement: ProblemStatement::PolynomialZero(residual.numerator().clone()),
                variables: request.variables.clone(),
                domain: request.domain,
            };
            let mut checked = dispatch_problem(context, &polynomial)?;
            checked.request = request.clone();
            checked.restrictions = restrictions(&request.statement);
            return Ok(checked);
        }
    }
    let checked = match context.solve_rational_quadratic_equation(&left, &right) {
        Ok(value) => value,
        Err(error) => return handle_evaluation_error(request, error),
    };
    if checked.left() != &left
        || checked.right() != &right
        || verify_rational_quadratic_equation(&checked).is_err()
    {
        return Err(ProblemError::InvalidEvidence);
    }
    // Check the rational solution set against both original operands, not only
    // the reduced numerator. Verify in the source domain before projecting
    // onto the legacy rational-only fallback scope.
    use crate::{
        CertifiedSolutionSet, SolutionCertificateError, certify_solution_set, verify_solution_set,
    };
    let certificate = match certify_solution_set(
        context,
        original_left,
        original_right,
        variable,
        request.domain,
        &[],
    ) {
        Ok(value) => value,
        Err(SolutionCertificateError::Evaluation(error)) => {
            return handle_evaluation_error(request, error);
        }
        Err(SolutionCertificateError::Expression(error)) => {
            return conversion_failure(request, error);
        }
        Err(_) => return Err(ProblemError::InvalidEvidence),
    };
    match verify_solution_set(
        context,
        original_left,
        original_right,
        variable,
        request.domain,
        &certificate,
    ) {
        Ok(true) => {}
        Ok(false) => return Err(ProblemError::InvalidEvidence),
        Err(SolutionCertificateError::Evaluation(error)) => {
            return handle_evaluation_error(request, error);
        }
        Err(SolutionCertificateError::Expression(error)) => {
            return conversion_failure(request, error);
        }
        Err(_) => return Err(ProblemError::InvalidEvidence),
    }
    let agrees = match (&certificate.solutions, checked.roots()) {
        (CertifiedSolutionSet::None, RationalQuadraticRoots::Finite(values)) => values.is_empty(),
        (CertifiedSolutionSet::Finite(roots), RationalQuadraticRoots::Finite(values)) => {
            let rational_roots: Vec<_> = roots
                .iter()
                .filter_map(|root| match root.root() {
                    crate::ExactQuadraticRoot::Rational(value) => Some(value),
                    _ => None,
                })
                .collect();
            rational_roots.len() == values.len()
                && rational_roots.iter().all(|value| values.contains(value))
        }
        (CertifiedSolutionSet::AllAdmissible, RationalQuadraticRoots::AllAdmissibleRationals) => {
            true
        }
        _ => false,
    };
    if !agrees {
        return Err(ProblemError::InvalidEvidence);
    }
    let outcome = match checked.roots() {
        RationalQuadraticRoots::Finite(values) if request.domain == ScalarDomain::Complex => {
            ProblemOutcome::PartialScope {
                known: ExactAnswer::RationalRoots(values.clone()),
                reason: UnsupportedReason::Domain,
            }
        }
        RationalQuadraticRoots::AllAdmissibleRationals
            if request.domain == ScalarDomain::Complex =>
        {
            ProblemOutcome::PartialScope {
                known: ExactAnswer::AllAdmissibleValues,
                reason: UnsupportedReason::Domain,
            }
        }
        RationalQuadraticRoots::Finite(values) if values.is_empty() => ProblemOutcome::NoSolutions,
        RationalQuadraticRoots::Finite(values) => {
            ProblemOutcome::SolvedExact(ExactAnswer::RationalRoots(values.clone()))
        }
        RationalQuadraticRoots::AllAdmissibleRationals => ProblemOutcome::AllAdmissibleValues,
    };
    let mut published = with_exclusions(
        checked_result(
            request,
            outcome,
            CheckedBy::RationalQuadraticEquation,
            Vec::new(),
        ),
        checked.domain_conditions(),
    );
    published.result_domain = ScalarDomain::Rational;
    Ok(published)
}

fn analytic_failure(
    request: &ProblemRequest,
    error: AnalyticConstantError,
) -> Result<ProblemResult, ProblemError> {
    match error {
        AnalyticConstantError::Limit(kind) => refusal(
            request,
            match kind {
                AnalyticConstantLimit::Nodes | AnalyticConstantLimit::NameBytes => {
                    ResourceLimitKind::ExpressionNodes
                }
                AnalyticConstantLimit::Depth => ResourceLimitKind::ExpressionDepth,
                AnalyticConstantLimit::Conditions => ResourceLimitKind::RationalConditions,
                AnalyticConstantLimit::ScalarBits => ResourceLimitKind::ScalarBits,
                AnalyticConstantLimit::Work | AnalyticConstantLimit::Configuration => {
                    ResourceLimitKind::WorkUnits
                }
            },
        ),
        AnalyticConstantError::Unsupported(_) => Ok(unsupported(
            request,
            UnsupportedReason::ExpressionOutsideAlgorithm,
        )),
        AnalyticConstantError::Definedness(error) => definedness_failure(request, error),
        AnalyticConstantError::ContradictedCondition(condition) => {
            let mut published = result(
                request,
                ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition),
            );
            let restriction = ProblemRestriction::Condition(condition);
            if !published.restrictions.contains(&restriction) {
                published.restrictions.push(restriction);
            }
            Ok(published)
        }
        AnalyticConstantError::InvalidRulePath => Err(ProblemError::InvalidEvidence),
    }
}

fn evaluate_exact(
    context: &EvaluationContext,
    request: &ProblemRequest,
    source: &DomainExpression,
) -> Result<ProblemResult, ProblemError> {
    const MAX_REWRITES: usize = 16;
    let configured = context.limits();
    let defaults = AnalyticConstantLimits::default();
    let limits = AnalyticConstantLimits {
        max_nodes: defaults.max_nodes.min(configured.max_expression_nodes),
        max_depth: defaults.max_depth.min(configured.max_expression_depth),
        max_conditions: defaults
            .max_conditions
            .min(configured.max_rational_conditions),
        max_scalar_bits: defaults
            .max_scalar_bits
            .min(configured.max_scalar_bits)
            .min(configured.max_intermediate_scalar_bits),
        max_work: defaults.max_work.min(
            usize::try_from(configured.max_work_units / (2 * MAX_REWRITES as u64 + 1))
                .unwrap_or(usize::MAX),
        ),
    };
    let mut current = source.clone();
    let mut steps = Vec::new();
    let mut retained = source.conditions().to_vec();
    let mut exhausted = true;
    for index in 0..MAX_REWRITES {
        match rewrite_analytic_constant_once(&current, request.domain, limits) {
            Ok(AnalyticConstantOutcome::Exact(proof)) => {
                match verify_analytic_constant_rewrite(&current, request.domain, &proof, limits) {
                    Ok(true) => {}
                    Ok(false) => return Err(ProblemError::InvalidEvidence),
                    Err(error) => return analytic_failure(request, error),
                }
                if let Err(error) = admit_source(context, &proof.result) {
                    return handle_evaluation_error(request, error);
                }
                steps.push(StepReference {
                    index,
                    rule_id: proof.rule.stable_id(),
                });
                current = proof.result;
                retained = current.conditions().to_vec();
            }
            Ok(AnalyticConstantOutcome::NoRule {
                retained_conditions,
            }) => {
                retained = retained_conditions;
                exhausted = false;
                break;
            }
            Ok(AnalyticConstantOutcome::Unsupported(_)) => {
                return Ok(unsupported(
                    request,
                    UnsupportedReason::ExpressionOutsideAlgorithm,
                ));
            }
            Err(error) => return analytic_failure(request, error),
        }
    }
    let value = match current.expression().view() {
        ExprView::Integer(value) => Some(Rational::from(value.clone())),
        ExprView::Rational(value) => Some(value.clone()),
        _ => None,
    };
    let outcome = if let Some(value) = value {
        conditional_answer(ExactAnswer::Rational(value), &retained)
    } else if steps.is_empty() {
        return Ok(unsupported(
            request,
            UnsupportedReason::ExpressionOutsideAlgorithm,
        ));
    } else {
        ProblemOutcome::PartialScope {
            known: ExactAnswer::Expression(current.expression().clone()),
            reason: if exhausted {
                UnsupportedReason::RewriteStepLimit
            } else {
                UnsupportedReason::ExpressionOutsideAlgorithm
            },
        }
    };
    let proof = if steps.is_empty() {
        CheckedBy::RationalLiteral
    } else {
        CheckedBy::AnalyticConstantRules
    };
    let mut published = checked_result(request, outcome, proof, steps);
    for condition in retained {
        let restriction = ProblemRestriction::Condition(condition);
        if !published.restrictions.contains(&restriction) {
            published.restrictions.push(restriction);
        }
    }
    Ok(published)
}
