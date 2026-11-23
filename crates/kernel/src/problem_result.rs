//! Typed, domain-explicit results for existing checked mathematical operations.
//!
//! This module does not parse source text or infer missing source restrictions.
//! Inputs are already-normalized mathematical statements; conditions inherited
//! from construction must be carried by `DomainExpression` or `RationalFunction`.

use std::error::Error;
use std::fmt;

use mathema::{Condition, DomainExpression, Equation, Expr, Rational, ScalarDomain, Symbol};

use crate::{
    Assumptions, ConditionStatus, EvaluationContext, EvaluationError, LinearSolution,
    LinearSolveResult, PartialPolynomialRootError, PartialPolynomialRootResult,
    PolynomialRootMultiplicity, PolynomialZeroSet, QuadraticResult, QuadraticRoots,
    RationalEquationResult, RationalEquationSolution, RationalFunction, RationalFunctionError,
    RationalPolynomial, ResourceLimitKind, SymbolicDerivative, SymbolicDifferentiationError,
    verify_linear_solution, verify_partial_polynomial_roots, verify_quadratic_roots,
    verify_rational_equation, verify_symbolic_derivative,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProblemOperation {
    SolveLinear,
    SolveRational,
    RationalRoots,
    ComplexPolynomialRoots,
    Differentiate,
    EvaluateRational,
    Integrate,
}

impl ProblemOperation {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::SolveLinear => "equation.linear",
            Self::SolveRational => "equation.rational_linear",
            Self::RationalRoots => "polynomial.rational_roots_degree_two",
            Self::ComplexPolynomialRoots => "polynomial.complex_roots_partial",
            Self::Differentiate => "calculus.differentiate",
            Self::EvaluateRational => "rational.evaluate",
            Self::Integrate => "calculus.integrate",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProblemStatement {
    Equation(Equation),
    PolynomialZero(RationalPolynomial),
    RationalEquation {
        left: RationalFunction,
        right: RationalFunction,
    },
    RationalEvaluation {
        function: RationalFunction,
        at: Rational,
    },
    Expression(DomainExpression),
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason {
    Domain,
    SeveralVariables,
    AlgorithmNotAvailable,
    ExpressionOutsideAlgorithm,
    DegreeOutsideAlgorithm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndefinedReason {
    ExcludedPoint,
    ContradictedDomainCondition,
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
    SymbolicDerivativeRules,
    RationalEvaluation,
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
    Derivative(&'a SymbolicDerivative),
    RationalEvaluation(&'a Rational),
}

fn restrictions(statement: &ProblemStatement) -> Vec<ProblemRestriction> {
    match statement {
        ProblemStatement::Expression(source) => source
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
        ProblemStatement::Equation(_) | ProblemStatement::PolynomialZero(_) => Vec::new(),
    }
}

fn result(request: &ProblemRequest, outcome: ProblemOutcome) -> ProblemResult {
    ProblemResult {
        request: request.clone(),
        result_domain: request.domain,
        outcome,
        restrictions: restrictions(&request.statement),
        proof: None,
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
        other => return Err(ProblemError::Evaluation(other)),
    };
    Ok(result(request, outcome))
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
        ProblemStatement::Expression(source) => {
            context.check_expression(source.expression())?;
            if source.conditions().len() > limits.max_rational_conditions {
                return Err(EvaluationError::Limit(
                    ResourceLimitKind::RationalConditions,
                ));
            }
            for condition in source.conditions() {
                context.check_expression(condition.expression())?;
            }
            Ok(())
        }
    }
}

fn validate_request(request: &ProblemRequest) -> Result<Option<ProblemResult>, ProblemError> {
    if request.variables.is_empty() {
        return Err(ProblemError::MissingVariable);
    }
    if request.variables.len() != 1 {
        return Ok(Some(unsupported(
            request,
            UnsupportedReason::SeveralVariables,
        )));
    }
    let matched = matches!(
        (&request.operation, &request.statement),
        (ProblemOperation::SolveLinear, ProblemStatement::Equation(_))
            | (
                ProblemOperation::SolveRational,
                ProblemStatement::RationalEquation { .. }
            )
            | (
                ProblemOperation::RationalRoots,
                ProblemStatement::PolynomialZero(_)
            )
            | (
                ProblemOperation::ComplexPolynomialRoots,
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
            | (ProblemOperation::Integrate, ProblemStatement::Expression(_))
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
        ProblemOperation::Differentiate => {
            matches!(request.domain, ScalarDomain::Real | ScalarDomain::Complex)
        }
        ProblemOperation::Integrate => true,
    };
    if !domain_supported {
        return Ok(Some(unsupported(request, UnsupportedReason::Domain)));
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
    let variable = &request.variables[0];
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
            verify_linear_solution(candidate, variable)
                .map_err(|_| ProblemError::InvalidEvidence)?;
            for step in candidate.steps() {
                context
                    .check_equation(step.before())
                    .map_err(ProblemError::Evaluation)?;
                context
                    .check_equation(step.after())
                    .map_err(ProblemError::Evaluation)?;
            }
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
                || candidate.variable() != variable
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
    let variable = &request.variables[0];
    match (&request.operation, &request.statement) {
        (ProblemOperation::SolveLinear, ProblemStatement::Equation(equation)) => {
            match context.solve_linear(equation, variable) {
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
            match context.differentiate_domain_expression(source, variable, request.domain) {
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
            match context.check_expression(source.expression()) {
                Ok(()) => Ok(unsupported(
                    request,
                    UnsupportedReason::AlgorithmNotAvailable,
                )),
                Err(error) => handle_evaluation_error(request, error),
            }
        }
        _ => Err(ProblemError::StatementMismatch),
    }
}
