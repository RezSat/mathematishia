//! Source-checked projections for the opt-in school mathematics strategies.

use super::*;
use crate::absolute_equations::{
    PiecewiseAffine, PiecewiseBranch, PiecewiseEquation, PiecewiseEquationError, PiecewiseInterval,
};
use crate::analytic_semantics::{
    DomainPreconditionError, DomainPreconditionReport, PrincipalSquareRootValue,
};
use crate::calculus_rule_checks::RationalCalculusError;
use crate::elementary_integration::polynomial_analytic::{
    PolynomialSubstitutionPrimitive, PolynomialTrigonometricPrimitive,
};
use crate::limits::{AnalyticLimitError, InfinityEnd, LimitLeadingTerm, StandardLimitRule};
use crate::polynomial_equation_dispatch::{
    PolynomialEquationError, PolynomialLiftEvidence, SchoolPolynomialResult,
};
use crate::trigonometry::{PeriodicFamily, PeriodicTrigEquation, TrigError, TrigLimits};
use crate::{CertifiedSolutionSet, PolynomialConversionError};

#[derive(Debug)]
pub enum SchoolError {
    Evaluation(EvaluationError),
    Problem(ProblemError),
    Wire(ProblemWireError),
    Unsupported(String),
    Verification(String),
    Domain(String),
    Resource(String),
}

impl fmt::Display for SchoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => error.fmt(f),
            Self::Problem(error) => error.fmt(f),
            Self::Wire(error) => error.fmt(f),
            Self::Unsupported(message)
            | Self::Verification(message)
            | Self::Domain(message)
            | Self::Resource(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for SchoolError {}

impl From<ProblemWireError> for SchoolError {
    fn from(error: ProblemWireError) -> Self {
        Self::Wire(error)
    }
}

impl From<ProblemError> for SchoolError {
    fn from(error: ProblemError) -> Self {
        Self::Problem(error)
    }
}

impl From<EvaluationError> for SchoolError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

fn verified(ok: bool) -> Result<(), SchoolError> {
    if ok {
        Ok(())
    } else {
        Err(SchoolError::Verification(
            "school mathematics certificate failed verification".into(),
        ))
    }
}

fn bound(
    context: &EvaluationContext,
    source: &DomainExpression,
    domain: ScalarDomain,
) -> Result<(), SchoolError> {
    admit_source(context, source)?;
    if source.construction_domain() != Some(domain) || source.has_external_history() {
        return Err(SchoolError::Unsupported(
            "explicitly bound original source required".into(),
        ));
    }
    if source_is_refuted(source) {
        return Err(SchoolError::Domain(
            "original source has a refuted condition".into(),
        ));
    }
    Ok(())
}

fn envelope(
    status: &str,
    method: &str,
    request: impl FnOnce(&mut Encoder) -> WireResult,
    answer: impl FnOnce(&mut Encoder) -> WireResult,
    evidence: impl FnOnce(&mut Encoder) -> WireResult,
) -> Result<String, SchoolError> {
    let mut writer = Encoder::default();
    object!(writer,
        "schema_version" => |w| w.count(VERSION),
        "kind" => |w| w.string("checked_school_result"),
        "status" => |w| w.string(status),
        "method" => |w| w.string(method),
        "request" => request,
        "answer" => answer,
        "evidence" => evidence,
    )?;
    Ok(writer.text)
}

fn school_response(
    request: &ProblemRequest,
    status: &str,
    method: &str,
    answer: impl FnOnce(&mut Encoder) -> WireResult,
    evidence: impl FnOnce(&mut Encoder) -> WireResult,
) -> Result<String, SchoolError> {
    envelope(status, method, |w| w.request(request), answer, evidence)
}

fn legacy(context: &EvaluationContext, request: &ProblemRequest) -> Result<String, SchoolError> {
    Ok(dispatch_problem(context, request)?.to_json()?)
}

fn conditional(source: &DomainExpression) -> &'static str {
    if source.conditions().is_empty() {
        "complete"
    } else {
        "conditional"
    }
}

fn polynomial_error(error: PolynomialEquationError) -> SchoolError {
    match error {
        PolynomialEquationError::Evaluation(error) => error.into(),
        PolynomialEquationError::Certificate(crate::SolutionCertificateError::Evaluation(
            error,
        )) => error.into(),
        PolynomialEquationError::UnsupportedDomain
        | PolynomialEquationError::Certificate(
            crate::SolutionCertificateError::UnsupportedDomain,
        ) => SchoolError::Unsupported(error.to_string()),
        _ => SchoolError::Verification(error.to_string()),
    }
}

fn calculus_error(error: RationalCalculusError) -> SchoolError {
    match error {
        RationalCalculusError::Evaluation(error) => error.into(),
        RationalCalculusError::NameLimit => SchoolError::Resource(error.to_string()),
        RationalCalculusError::NewExclusion | RationalCalculusError::DomainMismatch => {
            SchoolError::Domain(error.to_string())
        }
        RationalCalculusError::IncorrectCertificate | RationalCalculusError::IncorrectIdentity => {
            SchoolError::Verification(error.to_string())
        }
        _ => SchoolError::Unsupported(error.to_string()),
    }
}

fn integration_error(error: ElementaryIntegrationError) -> SchoolError {
    use ElementaryIntegrationError as E;
    match error {
        E::Evaluation(error) => error.into(),
        E::UnsupportedArgument(PolynomialConversionError::DegreeLimitExceeded) => {
            SchoolError::Resource(error.to_string())
        }
        E::Definedness(
            DefinednessError::NodeLimitExceeded | DefinednessError::ConditionLimitExceeded,
        ) => SchoolError::Resource(error.to_string()),
        E::UnsupportedForm
        | E::UnsupportedFunction
        | E::UnsupportedArgument(_)
        | E::NonAffineArgument
        | E::ZeroSlope
        | E::UnsupportedScalarDomain
        | E::UnsupportedNormalization => SchoolError::Unsupported(error.to_string()),
        E::MissingDomainCondition | E::Definedness(_) => SchoolError::Domain(error.to_string()),
        _ => SchoolError::Verification(error.to_string()),
    }
}

fn solution_status(value: &CertifiedSolutionSet) -> &'static str {
    match value {
        CertifiedSolutionSet::None => "no_solutions",
        CertifiedSolutionSet::AllAdmissible => "all_admissible",
        CertifiedSolutionSet::Finite(_) => "complete",
        CertifiedSolutionSet::Partial { .. } => "partial",
    }
}

fn solution_set(w: &mut Encoder, value: &CertifiedSolutionSet) -> WireResult {
    match value {
        CertifiedSolutionSet::None => {
            w.array::<PolynomialRootMultiplicity>(&[], Encoder::root_multiplicity)
        }
        CertifiedSolutionSet::Finite(roots) => w.array(roots, Encoder::root_multiplicity),
        CertifiedSolutionSet::Partial {
            roots,
            unresolved_cofactor,
        } => object!(w,
            "known_roots" => |w| w.array(roots, Encoder::root_multiplicity),
            "unresolved_cofactor" => |w| w.polynomial(unresolved_cofactor),
        ),
        CertifiedSolutionSet::AllAdmissible => w.raw("null"),
    }
}

fn pattern_json(
    context: &EvaluationContext,
    request: &ProblemRequest,
    left: &DomainExpression,
    right: &DomainExpression,
    variable: &Symbol,
) -> Result<Option<String>, SchoolError> {
    if !left.conditions().is_empty() || !right.conditions().is_empty() {
        return Ok(None);
    }
    let proof =
        match context.solve_school_polynomial_equation(left, right, variable, request.domain) {
            Ok(Some(proof)) => proof,
            Ok(None)
            | Err(PolynomialEquationError::Evaluation(EvaluationError::Polynomial(
                PolynomialConversionError::OtherSymbol
                | PolynomialConversionError::UnsupportedFunction
                | PolynomialConversionError::UnsupportedPower,
            ))) => return Ok(None),
            Err(error) => return Err(polynomial_error(error)),
        };
    verified(
        context
            .verify_school_polynomial_equation(left, right, variable, request.domain, &proof)
            .map_err(polynomial_error)?,
    )?;
    Ok(Some(school_response(
        request,
        solution_status(proof.solutions()),
        proof.method_id(),
        |w| solution_set(w, proof.solutions()),
        |w| match &proof {
            SchoolPolynomialResult::Substitution(value) => object!(w,
                "original" => |w| w.polynomial(&value.original),
                "substitution_power" => |w| w.count(value.power),
                "outer_polynomial" => |w| w.polynomial(value.outer.original()),
                "lifts" => |w| w.array(&value.lifts, |w, lift| object!(w,
                    "factor" => |w| w.polynomial(&lift.factor),
                    "multiplicity" => |w| w.count(lift.multiplicity),
                    "kind" => |w| w.string(match &lift.evidence {
                        PolynomialLiftEvidence::Excluded(_) => "excluded",
                        PolynomialLiftEvidence::Quadratic(_) => "quadratic",
                        PolynomialLiftEvidence::FactorSearch(_) => "factor_search",
                    }),
                )),
            ),
            SchoolPolynomialResult::Factored(value) => object!(w,
                "original" => |w| w.polynomial(&value.original),
                "unit" => |w| w.rational(&value.unit),
                "factors" => |w| w.array(&value.factors, Encoder::polynomial),
            ),
        },
    )?))
}

fn rational_derivative_json(
    context: &EvaluationContext,
    request: &ProblemRequest,
    source: &DomainExpression,
    variable: &Symbol,
) -> Result<Option<String>, SchoolError> {
    let trace = match context.differentiate_rational_domain_expression(source, variable) {
        Ok(trace) => trace,
        Err(
            RationalCalculusError::UnsupportedSymbol
            | RationalCalculusError::UnsupportedFunction
            | RationalCalculusError::UnsupportedExponent
            | RationalCalculusError::UnsupportedCondition,
        ) => return Ok(None),
        Err(error) => return Err(calculus_error(error)),
    };
    context
        .verify_rational_domain_derivative(source, variable, &trace)
        .map_err(calculus_error)?;
    Ok(Some(school_response(
        request,
        conditional(source),
        "calculus.rational_quotient_rule",
        |w| w.domain_expression(&trace[1].candidate),
        |w| {
            w.array(&trace, |w, step| object!(w,
            "rule" => |w| w.string(match step.relation {
                crate::RationalCalculusRelation::Derivative => "calculus.quotient_rule",
                crate::RationalCalculusRelation::Identity => "algebra.rational_simplification",
            }),
            "before" => |w| w.domain_expression(&step.source),
            "after" => |w| w.domain_expression(&step.candidate),
            "rational_value" => |w| w.rational_function(&step.value),
        ))
        },
    )?))
}

fn parts_json(
    request: &ProblemRequest,
    proof: &PolynomialTrigonometricPrimitive,
) -> Result<String, SchoolError> {
    school_response(
        request,
        conditional(proof.source()),
        "calculus.polynomial_trigonometric_parts",
        |w| w.domain_expression(proof.primitive()),
        |w| {
            object!(w,
                "slope" => |w| w.rational(proof.slope()),
                "intercept" => |w| w.rational(proof.intercept()),
                "sine_polynomial" => |w| w.polynomial(proof.sine_polynomial()),
                "cosine_polynomial" => |w| w.polynomial(proof.cosine_polynomial()),
                "reductions" => |w| w.array(proof.steps(), |w, step| object!(w,
                    "rule" => |w| w.string(step.rule().stable_id()),
                    "multiplier" => |w| w.polynomial(step.multiplier()),
                    "boundary" => |w| w.polynomial(step.boundary()),
                    "remainder" => |w| w.polynomial(step.remainder()),
                )),
            )
        },
    )
}

fn substitution_json(
    request: &ProblemRequest,
    proof: &PolynomialSubstitutionPrimitive,
) -> Result<String, SchoolError> {
    school_response(
        request,
        conditional(proof.source()),
        "calculus.polynomial_substitution",
        |w| w.domain_expression(proof.primitive()),
        |w| {
            object!(w,
                "rule" => |w| w.string(proof.rule().stable_id()),
                "inner" => |w| w.polynomial(proof.inner()),
                "inner_derivative" => |w| w.polynomial(proof.inner_derivative()),
                "scale" => |w| w.rational(proof.scale()),
            )
        },
    )
}

fn integral_json(
    context: &EvaluationContext,
    request: &ProblemRequest,
    source: &DomainExpression,
    variable: &Symbol,
) -> Result<Option<String>, SchoolError> {
    match context.integrate_polynomial_substitution(source, variable, request.domain) {
        Ok(proof) => {
            context
                .verify_polynomial_substitution_primitive(source, variable, request.domain, &proof)
                .map_err(integration_error)?;
            return Ok(Some(substitution_json(request, &proof)?));
        }
        Err(error) => match integration_error(error) {
            SchoolError::Unsupported(_) => {}
            other => return Err(other),
        },
    }
    match context.integrate_polynomial_trigonometric(source, variable, request.domain) {
        Ok(proof) => {
            context
                .verify_polynomial_trigonometric_primitive(source, variable, request.domain, &proof)
                .map_err(integration_error)?;
            Ok(Some(parts_json(request, &proof)?))
        }
        Err(error) => match integration_error(error) {
            SchoolError::Unsupported(_) => Ok(None),
            other => Err(other),
        },
    }
}

fn inequality_error(error: InequalityError) -> SchoolError {
    match error {
        InequalityError::Evaluation(error)
        | InequalityError::Conversion(RationalExpressionError::Evaluation(error)) => error.into(),
        InequalityError::UnsupportedDegree => SchoolError::Unsupported(error.to_string()),
        InequalityError::Conversion(_) => SchoolError::Unsupported(error.to_string()),
        InequalityError::IncorrectCertificate => SchoolError::Verification(error.to_string()),
    }
}

fn analytic_error(error: AnalyticLimitError) -> SchoolError {
    match error {
        AnalyticLimitError::Evaluation(error) => error.into(),
        AnalyticLimitError::Unsupported => {
            SchoolError::Unsupported("analytic limit pattern is unsupported".into())
        }
        AnalyticLimitError::InvalidCertificate => {
            SchoolError::Verification("invalid analytic limit certificate".into())
        }
    }
}

/// Opt-in native strategies. Existing problem/result and text contracts remain unchanged.
pub fn checked_school_json(
    context: &EvaluationContext,
    request: &ProblemRequest,
) -> Result<String, SchoolError> {
    admit_request(context, request)?;
    if let Some(rejection) = validate_request(request)? {
        return Ok(rejection.to_json()?);
    }
    let Some(variable) = request.variables.first() else {
        return legacy(context, request);
    };
    match (&request.operation, &request.statement) {
        (
            ProblemOperation::SolveSourceEquation,
            ProblemStatement::SourceEquation { left, right },
        ) => {
            bound(context, left, request.domain)?;
            bound(context, right, request.domain)?;
            if let Some(json) = pattern_json(context, request, left, right, variable)? {
                return Ok(json);
            }
        }
        (ProblemOperation::Differentiate, ProblemStatement::Expression(source))
            if request.domain == ScalarDomain::Real =>
        {
            bound(context, source, request.domain)?;
            if let Some(json) = rational_derivative_json(context, request, source, variable)? {
                return Ok(json);
            }
        }
        (ProblemOperation::Integrate, ProblemStatement::Expression(source)) => {
            bound(context, source, request.domain)?;
            if let Some(json) = integral_json(context, request, source, variable)? {
                return Ok(json);
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
            bound(context, left, request.domain)?;
            bound(context, right, request.domain)?;
            let proof = context
                .solve_factored_expression_inequality(left, right, variable, *operator)
                .map_err(inequality_error)?;
            verified(
                context
                    .verify_expression_inequality(left, right, variable, *operator, &proof)
                    .map_err(inequality_error)?,
            )?;
            return school_response(
                request,
                if proof.intervals().is_empty() {
                    "no_solutions"
                } else {
                    "complete"
                },
                "inequality.factored_sign_chart",
                |w| w.array(proof.intervals(), Encoder::interval),
                |w| {
                    object!(w,
                        "residual" => |w| w.rational_function(proof.function()),
                        "critical_points" => |w| w.array(proof.critical_points(), |w, point| object!(w,
                            "root" => |w| w.root(point.root()),
                            "numerator_multiplicity" => |w| w.count(usize::from(point.numerator_multiplicity())),
                            "denominator_multiplicity" => |w| w.count(usize::from(point.denominator_multiplicity())),
                            "excluded" => |w| w.boolean(point.is_excluded()),
                        )),
                        "segment_signs" => |w| w.array(proof.segment_signs(), |w, sign| w.raw(&sign.to_string())),
                    )
                },
            );
        }
        (
            ProblemOperation::Limit,
            ProblemStatement::Limit {
                source,
                at,
                direction,
            },
        ) if at.numerator().is_zero() => {
            bound(context, source, request.domain)?;
            match context.analytic_limit_at_zero(source, variable, *direction) {
                Ok(proof) => {
                    proof
                        .verify(context, source, variable, *direction)
                        .map_err(analytic_error)?;
                    return school_response(
                        request,
                        "complete",
                        "calculus.standard_limit",
                        |w| w.limit_value(&proof.value),
                        |w| {
                            object!(w,
                                "rule" => |w| w.string(match proof.rule {
                                    StandardLimitRule::Sine => "sine",
                                    StandardLimitRule::OneMinusCosine => "one_minus_cosine",
                                    StandardLimitRule::ExponentialMinusOne => "exponential_minus_one",
                                    StandardLimitRule::NaturalLogarithm => "natural_logarithm",
                                }),
                                "argument_scale" => |w| w.rational(&proof.argument_scale),
                                "outer_scale" => |w| w.rational(&proof.outer_scale),
                                "denominator_order" => |w| w.count(proof.denominator_order),
                            )
                        },
                    );
                }
                Err(AnalyticLimitError::Unsupported) => {}
                Err(error) => return Err(analytic_error(error)),
            }
        }
        _ => {}
    }
    legacy(context, request)
}

fn semantic_error(error: DomainPreconditionError) -> SchoolError {
    match error {
        DomainPreconditionError::DepthLimit
        | DomainPreconditionError::ScalarLimit
        | DomainPreconditionError::NameLimit
        | DomainPreconditionError::Definedness(
            DefinednessError::NodeLimitExceeded | DefinednessError::ConditionLimitExceeded,
        ) => SchoolError::Resource(error.to_string()),
        _ => SchoolError::Unsupported(error.to_string()),
    }
}

pub fn checked_sqrt_json(
    context: &EvaluationContext,
    source: &DomainExpression,
) -> Result<String, SchoolError> {
    admit_source(context, source)?;
    let assumptions = Assumptions::new();
    let proof = DomainPreconditionReport::principal_square_root(source, &assumptions)
        .map_err(semantic_error)?;
    verified(
        DomainPreconditionReport::verify_principal_square_root(source, &assumptions, &proof)
            .map_err(semantic_error)?,
    )?;
    let status = match &proof.value {
        PrincipalSquareRootValue::Undefined => "undefined",
        PrincipalSquareRootValue::Unsupported => "unsupported",
        _ => conditional(source),
    };
    envelope(
        status,
        "radical.principal_real_square_root",
        |w| w.domain_expression(source),
        |w| match &proof.value {
            PrincipalSquareRootValue::Exact(root) => w.root(root),
            PrincipalSquareRootValue::Symbolic(value) => w.domain_expression(value),
            _ => w.raw("null"),
        },
        |w| w.raw("null"),
    )
}

fn periodic_limits(context: &EvaluationContext) -> Result<TrigLimits, SchoolError> {
    if context.limits().max_work_units < 32 {
        return Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits).into());
    }
    Ok(TrigLimits {
        max_input_bits: TrigLimits::default()
            .max_input_bits
            .min(context.limits().max_scalar_bits)
            .min(
                context
                    .limits()
                    .max_intermediate_scalar_bits
                    .saturating_sub(16)
                    / 2,
            ),
    })
}

fn trig_error(error: TrigError) -> SchoolError {
    match error {
        TrigError::InvalidEvidence => SchoolError::Verification(error.to_string()),
        TrigError::ScalarLimit | TrigError::InvalidLimits => {
            SchoolError::Resource(error.to_string())
        }
        _ => SchoolError::Unsupported(error.to_string()),
    }
}

fn periodic_family(w: &mut Encoder, family: &PeriodicFamily) -> WireResult {
    object!(w,
        "offset_pi" => |w| w.rational(family.offset.coefficient()),
        "period_pi" => |w| w.rational(family.period.coefficient()),
        "parameter_domain" => |w| w.string("Z"),
    )
}

pub fn checked_periodic_json(
    context: &EvaluationContext,
    equation: &PeriodicTrigEquation,
    variable: &Symbol,
) -> Result<String, SchoolError> {
    let limits = periodic_limits(context)?;
    let proof = equation.solve(limits).map_err(trig_error)?;
    verified(equation.verify(&proof, limits).map_err(trig_error)?)?;
    envelope(
        if proof.families.is_empty() {
            "no_solutions"
        } else {
            "complete"
        },
        "trig.periodic_families",
        |w| {
            object!(w,
                "function" => |w| w.string(trig_id(equation.function)),
                "variable" => |w| w.string(variable.name()),
                "domain" => |w| w.string(domain_id(equation.domain)),
                "slope" => |w| w.rational(&equation.slope),
                "shift_pi" => |w| w.rational(equation.shift.coefficient()),
                "target" => |w| w.root(&equation.target),
            )
        },
        |w| w.array(&proof.families, periodic_family),
        |w| {
            object!(w,
                "principal" => |w| match &proof.principal { Some(value) => w.special_angle(value), None => w.raw("null") },
                "argument_period_pi" => |w| match &proof.argument_period { Some(value) => w.rational(value.coefficient()), None => w.raw("null") },
                "excluded_poles" => |w| match &proof.excluded_poles { Some(value) => periodic_family(w, value), None => w.raw("null") },
            )
        },
    )
}

pub fn checked_trig_identity_json(
    context: &EvaluationContext,
    coefficients: &[Rational; 4],
) -> Result<String, SchoolError> {
    let limits = periodic_limits(context)?;
    let proof = TrigFunction::prove_squared_identity(coefficients, ScalarDomain::Real, limits)
        .map_err(trig_error)?;
    verified(
        proof
            .verify(coefficients, ScalarDomain::Real, limits)
            .map_err(trig_error)?,
    )?;
    envelope(
        "complete",
        "trig.squared_identity",
        |w| w.array(coefficients, Encoder::rational),
        |w| w.boolean(true),
        |w| {
            object!(w,
                "circle_multiplier" => |w| w.rational(&proof.circle_multiplier),
                "addition_multiplier" => |w| w.rational(&proof.addition_multiplier),
            )
        },
    )
}

fn limit_error(error: LimitError) -> SchoolError {
    match error {
        LimitError::Evaluation(error)
        | LimitError::Conversion(RationalExpressionError::Evaluation(error))
        | LimitError::Verification(LimitVerificationError::Evaluation(error)) => error.into(),
        LimitError::Conversion(_) => SchoolError::Unsupported(error.to_string()),
        _ => SchoolError::Verification(error.to_string()),
    }
}

fn leading(w: &mut Encoder, term: &LimitLeadingTerm) -> WireResult {
    object!(w,
        "degree" => |w| w.count(term.degree),
        "coefficient" => |w| w.rational(&term.coefficient),
        "tail_bound" => |w| w.rational(&term.tail_bound),
    )
}

pub fn checked_infinity_json(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    end: InfinityEnd,
) -> Result<String, SchoolError> {
    bound(context, source, ScalarDomain::Real)?;
    let function = context
        .rational_expression(source, variable)
        .map_err(|error| limit_error(LimitError::Conversion(error)))?;
    let proof = context
        .rational_infinity_limit(&function, end)
        .map_err(limit_error)?;
    proof
        .verify(context, &function, end)
        .map_err(|error| limit_error(LimitError::Verification(error)))?;
    envelope(
        "complete",
        "calculus.rational_infinity_limit",
        |w| {
            object!(w,
                "source" => |w| w.domain_expression(source),
                "variable" => |w| w.string(variable.name()),
                "end" => |w| w.string(match end { InfinityEnd::Positive => "+infinity", InfinityEnd::Negative => "-infinity" }),
            )
        },
        |w| w.limit_value(&proof.value),
        |w| {
            object!(w,
                "rational_function" => |w| w.rational_function(&function),
                "numerator" => |w| match &proof.numerator { Some(value) => leading(w, value), None => w.raw("null") },
                "denominator" => |w| leading(w, &proof.denominator),
                "exclusions" => |w| w.array(&proof.exclusions, leading),
            )
        },
    )
}

fn piecewise_interval(w: &mut Encoder, interval: &PiecewiseInterval) -> WireResult {
    object!(w,
        "lower" => |w| match &interval.lower { Some(value) => w.rational(value), None => w.raw("null") },
        "upper" => |w| match &interval.upper { Some(value) => w.rational(value), None => w.raw("null") },
        "lower_closed" => |w| w.boolean(interval.lower_closed),
        "upper_closed" => |w| w.boolean(interval.upper_closed),
    )
}

fn piecewise_form(w: &mut Encoder, form: &PiecewiseAffine) -> WireResult {
    match form {
        PiecewiseAffine::AbsoluteSum { base, terms } => object!(w,
            "kind" => |w| w.string("absolute_sum"),
            "base" => |w| w.absolute_affine(base),
            "terms" => |w| w.array(terms, |w, term| object!(w,
                "weight" => |w| w.rational(&term.weight),
                "argument" => |w| w.absolute_affine(&term.argument),
            )),
        ),
        PiecewiseAffine::Explicit(cases) => object!(w,
            "kind" => |w| w.string("explicit"),
            "cases" => |w| w.array(cases, |w, case| object!(w,
                "guard" => |w| piecewise_interval(w, &case.guard),
                "form" => |w| w.absolute_affine(&case.form),
            )),
        ),
    }
}

fn piecewise_branch(w: &mut Encoder, branch: &PiecewiseBranch) -> WireResult {
    match branch {
        PiecewiseBranch::Signs(signs) => w.array(signs, |w, sign| {
            w.string(match sign {
                AbsoluteSign::Nonnegative => "nonnegative",
                AbsoluteSign::Nonpositive => "nonpositive",
            })
        }),
        PiecewiseBranch::Explicit(index) => w.count(*index),
    }
}

fn piecewise_error(error: PiecewiseEquationError) -> SchoolError {
    match error {
        PiecewiseEquationError::Arithmetic(crate::AbsoluteEquationError::Evaluation(error)) => {
            error.into()
        }
        PiecewiseEquationError::CaseLimit => SchoolError::Resource(error.to_string()),
        PiecewiseEquationError::VerificationFailed => SchoolError::Verification(error.to_string()),
        _ => SchoolError::Unsupported(error.to_string()),
    }
}

pub fn checked_piecewise_json(
    context: &EvaluationContext,
    equation: &PiecewiseEquation,
) -> Result<String, SchoolError> {
    let proof = context
        .solve_piecewise_equation(equation)
        .map_err(piecewise_error)?;
    verified(
        context
            .verify_piecewise_equation(equation, &proof)
            .map_err(piecewise_error)?,
    )?;
    envelope(
        if proof.solutions.is_empty() {
            "no_solutions"
        } else {
            "complete"
        },
        proof.method_id(),
        |w| {
            object!(w,
                "variable" => |w| w.string(equation.variable.name()),
                "domain" => |w| w.string(domain_id(equation.domain)),
                "left" => |w| piecewise_form(w, &equation.left),
                "right" => |w| piecewise_form(w, &equation.right),
            )
        },
        |w| w.array(&proof.solutions, piecewise_interval),
        |w| {
            w.array(&proof.cases, |w, case| object!(w,
            "interval" => |w| piecewise_interval(w, &case.interval),
            "left" => |w| piecewise_branch(w, &case.left),
            "right" => |w| piecewise_branch(w, &case.right),
            "residual" => |w| w.absolute_affine(&case.residual),
            "solution" => |w| match &case.solution { Some(value) => piecewise_interval(w, value), None => w.raw("null") },
        ))
        },
    )
}
