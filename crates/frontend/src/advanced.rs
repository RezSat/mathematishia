//! Text admission for bounded checked algebra and proof operations.

use std::{collections::BTreeSet, fmt};

use mathema::{Condition, DomainExpression, Equation, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    AbsoluteAffine, AbsoluteEquation, AbsoluteEquationError, AbsoluteEquationResult,
    AbsoluteExpression, EvaluationContext, GroebnerCertificate, GroebnerError, IdealMembership,
    LinearSolution, MultivariateError, MultivariateExprError, MultivariateLimits,
    MultivariatePolynomial, PiMultiple, RationalCalculusCertificate, RationalCalculusError,
    RationalCalculusRelation, SpecialAngleCertificate, TrigError, TrigFunction, TrigLimits,
    VerifiedCondition, certify_rational_calculus, evaluate_special_angle,
    groebner_basis_from_domain_expressions, ideal_membership_from_domain_expressions,
    partial_trig_zeros, solve_absolute_equation,
    step_records::{
        StepClaim, StepConclusion, StepCondition, StepEvidence, StepLimits, StepRecordError,
        StepRecords, record_steps, verify_step_records,
    },
    verify_absolute_equation, verify_groebner_basis_from_domain_expressions,
    verify_ideal_membership_from_domain_expressions, verify_rational_calculus,
    verify_special_angle,
};

use crate::problem::{self, RequestError};
use crate::{DomainStatement, parse_domain_statement_in_domain};

#[derive(Debug)]
pub enum AdvancedError {
    Input(RequestError),
    Resource(String),
    Unsupported(String),
    Domain(String),
    Verification(String),
}

impl fmt::Display for AdvancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(error) => error.fmt(f),
            Self::Resource(message)
            | Self::Unsupported(message)
            | Self::Domain(message)
            | Self::Verification(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for AdvancedError {}

impl From<RequestError> for AdvancedError {
    fn from(error: RequestError) -> Self {
        Self::Input(error)
    }
}

fn evaluation(error: mathematishia_kernel::EvaluationError) -> AdvancedError {
    AdvancedError::Input(RequestError::Evaluation(error))
}

fn invalid(message: &'static str) -> AdvancedError {
    RequestError::Invalid(message).into()
}

fn rejected(message: &'static str) -> AdvancedError {
    AdvancedError::Unsupported(message.into())
}

fn verification(message: &'static str) -> AdvancedError {
    AdvancedError::Verification(message.into())
}

fn ring(context: &EvaluationContext, names: &[&str]) -> Result<Vec<Symbol>, AdvancedError> {
    if names.len() > 3 {
        return Err(AdvancedError::Resource(
            "at most three Groebner variables".into(),
        ));
    }
    let symbols = names
        .iter()
        .map(|name| problem::variable(context, name))
        .collect::<Result<Vec<_>, _>>()?;
    if symbols.iter().collect::<BTreeSet<_>>().len() != symbols.len() {
        return Err(invalid("variables must be distinct"));
    }
    Ok(symbols)
}

fn groebner_error(error: GroebnerError) -> AdvancedError {
    use GroebnerError as G;
    use MultivariateError as M;
    use MultivariateExprError as E;
    let message = error.to_string();
    match error {
        G::GeneratorLimit
        | G::BasisLimit
        | G::VariableNameLimit
        | G::Polynomial(
            M::VariableLimit
            | M::TotalDegreeLimit
            | M::TermLimit
            | M::InputTermLimit
            | M::ScalarBitLimit
            | M::WorkLimit,
        )
        | G::Expression(
            E::ExponentLimit | E::ExpressionDepthLimit | E::ExpressionNodeLimit | E::ConditionLimit,
        ) => AdvancedError::Resource(message),
        G::RestrictedDomain { .. } => AdvancedError::Domain(message),
        G::InvalidCertificate => AdvancedError::Verification(message),
        G::Expression(E::Polynomial(error)) => groebner_error(G::Polynomial(error)),
        _ => AdvancedError::Unsupported(message),
    }
}

fn ideal_input(
    context: &EvaluationContext,
    names: &[&str],
    generators: &[&str],
) -> Result<(Vec<Symbol>, Vec<DomainExpression>), AdvancedError> {
    if generators.len() > 8 {
        return Err(AdvancedError::Resource(
            "at most eight Groebner generators".into(),
        ));
    }
    Ok((
        ring(context, names)?,
        generators
            .iter()
            .map(|source| problem::source(source, ScalarDomain::Real))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

pub fn basis(
    context: &EvaluationContext,
    names: &[&str],
    generators: &[&str],
) -> Result<GroebnerCertificate, AdvancedError> {
    let (variables, sources) = ideal_input(context, names, generators)?;
    let limits = MultivariateLimits::default();
    let result = groebner_basis_from_domain_expressions(&variables, &sources, limits)
        .map_err(groebner_error)?;
    if !verify_groebner_basis_from_domain_expressions(&variables, &sources, &result, limits)
        .map_err(groebner_error)?
    {
        return Err(verification("Groebner certificate failed verification"));
    }
    Ok(result)
}

pub fn membership(
    context: &EvaluationContext,
    names: &[&str],
    generators: &[&str],
    target: &str,
) -> Result<IdealMembership, AdvancedError> {
    let (variables, sources) = ideal_input(context, names, generators)?;
    let limits = MultivariateLimits::default();
    let target = problem::source(target, ScalarDomain::Real)?;
    let basis = groebner_basis_from_domain_expressions(&variables, &sources, limits)
        .map_err(groebner_error)?;
    let result =
        ideal_membership_from_domain_expressions(&variables, &sources, &basis, &target, limits)
            .map_err(groebner_error)?;
    if !verify_ideal_membership_from_domain_expressions(
        &variables, &sources, &basis, &target, &result, limits,
    )
    .map_err(groebner_error)?
    {
        return Err(verification(
            "ideal membership evidence failed verification",
        ));
    }
    Ok(result)
}

pub fn special_angle(
    context: &EvaluationContext,
    function: &str,
    coefficient: &str,
) -> Result<SpecialAngleCertificate, AdvancedError> {
    let function = match function {
        "sin" => TrigFunction::Sin,
        "cos" => TrigFunction::Cos,
        "tan" => TrigFunction::Tan,
        _ => return Err(invalid("expected sin, cos or tan")),
    };
    let angle = PiMultiple::new(problem::scalar(context, coefficient)?);
    let limits = TrigLimits::default();
    let map = |error: TrigError| match error {
        TrigError::ScalarLimit | TrigError::InvalidLimits => {
            AdvancedError::Resource(error.to_string())
        }
        TrigError::InvalidEvidence => AdvancedError::Verification(error.to_string()),
        _ => AdvancedError::Unsupported(error.to_string()),
    };
    let result =
        evaluate_special_angle(&angle, function, ScalarDomain::Real, limits).map_err(map)?;
    if !verify_special_angle(&angle, function, ScalarDomain::Real, &result, limits).map_err(map)? {
        return Err(verification("special angle evidence failed verification"));
    }
    Ok(result)
}

fn affine(
    context: &EvaluationContext,
    input: &str,
    variable: &Symbol,
) -> Result<AbsoluteAffine, AdvancedError> {
    let parsed = problem::source(input, ScalarDomain::Real)?;
    if !parsed.conditions().is_empty() {
        return Err(AdvancedError::Domain(
            "absolute-value chains cannot discard source restrictions".into(),
        ));
    }
    let polynomial = context
        .polynomial(parsed.expression(), variable)
        .map_err(evaluation)?;
    if polynomial.degree().is_some_and(|degree| degree > 1) {
        return Err(rejected("absolute-value layers must be affine"));
    }
    let coefficient = |index: usize| {
        polynomial
            .coefficients()
            .get(index)
            .cloned()
            .unwrap_or_else(|| Rational::from(Integer::zero()))
    };
    Ok(AbsoluteAffine {
        slope: coefficient(1),
        constant: coefficient(0),
    })
}

const PLACEHOLDER: &str = "mathemaAbsLayer";

// Each side may have one nested chain. Replace the outermost abs call with
// a fresh identifier and admit the surrounding affine map independently.
fn absolute_chain(
    context: &EvaluationContext,
    source: &str,
    variable: &Symbol,
    depth: usize,
) -> Result<AbsoluteExpression, AdvancedError> {
    if depth > 8 {
        return Err(AdvancedError::Resource(
            "at most eight absolute-value layers".into(),
        ));
    }
    let Some(start) = source.find("abs(") else {
        return Ok(AbsoluteExpression {
            variable: variable.clone(),
            base: affine(context, source, variable)?,
            layers: Vec::new(),
        });
    };
    if start != 0
        && source[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
    {
        return Err(rejected("expected an explicit abs call"));
    }
    let begin = start + 4;
    let mut nesting = 1usize;
    let mut end = None;
    for (offset, c) in source[begin..].char_indices() {
        match c {
            '(' => nesting += 1,
            ')' => {
                nesting -= 1;
                if nesting == 0 {
                    end = Some(begin + offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| invalid("unclosed abs call"))?;
    let outer = format!("{}({PLACEHOLDER}){}", &source[..start], &source[end + 1..]);
    let placeholder = problem::variable(context, PLACEHOLDER)?;
    let layer = affine(context, &outer, &placeholder)?;
    let mut result = absolute_chain(context, &source[begin..end], variable, depth + 1)?;
    result.layers.push(layer);
    Ok(result)
}

pub(crate) fn absolute_request(
    context: &EvaluationContext,
    source: &str,
    name: &str,
) -> Result<AbsoluteEquation, AdvancedError> {
    if source.len() > 32768 {
        return Err(AdvancedError::Resource(
            "absolute equation input limit exceeded".into(),
        ));
    }
    if source.contains(PLACEHOLDER) || name.trim() == PLACEHOLDER {
        return Err(invalid("reserved absolute-value layer identifier"));
    }
    let (left, right) = source
        .split_once('=')
        .ok_or_else(|| invalid("expected an equation containing '='"))?;
    if right.contains('=') {
        return Err(invalid("expected one equality"));
    }
    let variable = problem::variable(context, name)?;
    Ok(AbsoluteEquation {
        domain: ScalarDomain::Real,
        left: absolute_chain(context, left, &variable, 0)?,
        right: absolute_chain(context, right, &variable, 0)?,
    })
}

pub fn absolute(
    context: &EvaluationContext,
    source: &str,
    name: &str,
) -> Result<AbsoluteEquationResult, AdvancedError> {
    let equation = absolute_request(context, source, name)?;
    let map = |error: AbsoluteEquationError| match error {
        AbsoluteEquationError::Evaluation(error) => evaluation(error),
        AbsoluteEquationError::CaseLimit => AdvancedError::Resource(error.to_string()),
        AbsoluteEquationError::VerificationFailed => AdvancedError::Verification(error.to_string()),
        _ => AdvancedError::Unsupported(error.to_string()),
    };
    let result = solve_absolute_equation(context, &equation).map_err(map)?;
    if !verify_absolute_equation(context, &equation, &result).map_err(map)? {
        return Err(verification(
            "absolute equation evidence failed verification",
        ));
    }
    Ok(result)
}

pub fn calculus(
    context: &EvaluationContext,
    source: &str,
    candidate: &str,
    name: &str,
    relation: &str,
) -> Result<RationalCalculusCertificate, AdvancedError> {
    let relation = match relation {
        "identity" => RationalCalculusRelation::Identity,
        "derivative" => RationalCalculusRelation::Derivative,
        _ => return Err(invalid("expected identity or derivative")),
    };
    let source = problem::source(source, ScalarDomain::Real)?;
    let candidate = problem::source(candidate, ScalarDomain::Real)?;
    let variable = problem::variable(context, name)?;
    let map = |error: RationalCalculusError| match error {
        RationalCalculusError::Evaluation(error) => evaluation(error),
        RationalCalculusError::NameLimit => AdvancedError::Resource(error.to_string()),
        RationalCalculusError::NewExclusion | RationalCalculusError::DomainMismatch => {
            AdvancedError::Domain(error.to_string())
        }
        RationalCalculusError::IncorrectIdentity | RationalCalculusError::IncorrectCertificate => {
            AdvancedError::Verification(error.to_string())
        }
        _ => AdvancedError::Unsupported(error.to_string()),
    };
    let result = certify_rational_calculus(context, &source, &candidate, &variable, relation)
        .map_err(map)?;
    verify_rational_calculus(context, &source, &candidate, &variable, relation, &result)
        .map_err(map)?;
    Ok(result)
}

fn step_error(error: StepRecordError) -> AdvancedError {
    match error {
        StepRecordError::Evaluation(error) => evaluation(error),
        StepRecordError::ResourceLimit => AdvancedError::Resource(error.to_string()),
        StepRecordError::InvalidDomain => AdvancedError::Domain(error.to_string()),
        StepRecordError::InvalidEvidence
        | StepRecordError::InvalidRecord
        | StepRecordError::SourceMismatch => AdvancedError::Verification(error.to_string()),
        _ => AdvancedError::Unsupported(error.to_string()),
    }
}

fn wire_step_error(error: mathematishia_kernel::ProblemWireError) -> AdvancedError {
    match error {
        mathematishia_kernel::ProblemWireError::StepVerification(error) => step_error(error),
        _ => AdvancedError::Resource(error.to_string()),
    }
}

fn checked_records(evidence: StepEvidence<'_>) -> Result<StepRecords, AdvancedError> {
    let limits = StepLimits::default();
    let records = record_steps(evidence, limits).map_err(step_error)?;
    verify_step_records(evidence, &records, limits).map_err(step_error)?;
    Ok(records)
}

pub fn derivation(
    context: &EvaluationContext,
    kind: &str,
    source: &str,
    name: &str,
) -> Result<StepRecords, AdvancedError> {
    with_derivation(context, kind, source, name, checked_records)
}

pub fn derivation_json(
    context: &EvaluationContext,
    kind: &str,
    source: &str,
    name: &str,
) -> Result<String, AdvancedError> {
    with_derivation(context, kind, source, name, |evidence| {
        let limits = StepLimits::default();
        let records = record_steps(evidence, limits).map_err(step_error)?;
        records
            .to_checked_json(evidence, limits)
            .map_err(wire_step_error)
    })
}

fn with_derivation<T>(
    context: &EvaluationContext,
    kind: &str,
    source: &str,
    name: &str,
    emit: impl FnOnce(StepEvidence<'_>) -> Result<T, AdvancedError>,
) -> Result<T, AdvancedError> {
    let variable = problem::variable(context, name)?;
    match kind {
        "expand" => {
            let domain = ScalarDomain::Real;
            let source = problem::source(source, domain)?;
            let result = context.expand(source.expression()).map_err(evaluation)?;
            emit(StepEvidence::Distribution {
                source: &source,
                domain,
                result: &result,
            })
        }
        "linear" => {
            let statement = parse_domain_statement_in_domain(source, ScalarDomain::Rational)
                .map_err(RequestError::Parse)?;
            let DomainStatement::Equation { left, right } = statement else {
                return Err(invalid("expected an equation"));
            };
            if !left.conditions().is_empty() || !right.conditions().is_empty() {
                return Err(rejected(
                    "linear derivation cannot discard source restrictions",
                ));
            }
            let equation = Equation::new(left.expression().clone(), right.expression().clone());
            let result = context
                .solve_linear(&equation, &variable)
                .map_err(evaluation)?;
            emit(StepEvidence::Linear {
                left: &left,
                right: &right,
                variable: &variable,
                result: &result,
            })
        }
        "derivative" => {
            let domain = ScalarDomain::Real;
            let source = problem::source(source, domain)?;
            let result = context
                .differentiate_domain_expression(&source, &variable, domain)
                .map_err(evaluation)?;
            emit(StepEvidence::Derivative {
                source: &source,
                domain,
                variable: &variable,
                result: &result,
            })
        }
        _ => Err(invalid("expected linear, derivative or expand")),
    }
}

pub fn polynomial_text(poly: &MultivariatePolynomial) -> Result<String, AdvancedError> {
    poly.to_expr(MultivariateLimits::default())
        .map(|value| value.to_string())
        .map_err(|error| groebner_error(GroebnerError::Expression(error)))
}

pub fn trig_zeros(function: &str) -> Result<Vec<Rational>, AdvancedError> {
    let function = match function {
        "sin" => TrigFunction::Sin,
        "cos" => TrigFunction::Cos,
        _ => return Err(rejected("partial zero examples support sine and cosine")),
    };
    let result = partial_trig_zeros(function, ScalarDomain::Real, TrigLimits::default())
        .map_err(|error| AdvancedError::Unsupported(error.to_string()))?;
    Ok(result
        .known_zeros()
        .iter()
        .map(|proof| proof.angle.coefficient().clone())
        .collect())
}

pub fn condition_text(condition: &Condition) -> String {
    match condition {
        Condition::NonZero(expr) => format!("{expr} != 0"),
        Condition::Positive(expr) => format!("{expr} > 0"),
        Condition::Real(expr) => format!("{expr} in R"),
        Condition::Integer(expr) => format!("{expr} in Z"),
        Condition::InDomain { expression, domain } => format!("{expression} in {domain:?}"),
    }
}

pub fn step_condition_text(condition: &StepCondition) -> String {
    match condition {
        StepCondition::Source(index) => format!("source condition {index}"),
        StepCondition::Proven(VerifiedCondition::Nonzero(value)) => format!("proved {value} != 0"),
    }
}

pub fn claim_text(claim: &StepClaim) -> String {
    match claim {
        StepClaim::Equal { before, after } => format!("{before} = {after}"),
        StepClaim::Equivalent {
            before,
            after,
            variable,
        } => format!(
            "{} = {} <=> {} = {} (in {variable})",
            before.left(),
            before.right(),
            after.left(),
            after.right()
        ),
        StepClaim::DerivativeOf {
            before,
            after,
            variable,
        } => format!("d/d{variable} ({before}) = {after}"),
        StepClaim::PolynomialDivision { .. } | StepClaim::MultivariateDivision { .. } => {
            "final polynomial division recomposition".into()
        }
    }
}

pub fn conclusion_text(conclusion: &StepConclusion) -> String {
    match conclusion {
        StepConclusion::Distribution {
            result, complete, ..
        } => format!("distribution result: {result}; distributive traversal complete: {complete}"),
        StepConclusion::Linear {
            variable, solution, ..
        } => match solution {
            LinearSolution::Unique(value) => format!("{variable} = {value}"),
            LinearSolution::NoSolution => "no rational solutions".into(),
            LinearSolution::AllRationals => "all rational values".into(),
        },
        StepConclusion::Derivative { result, .. } => format!("derivative: {result}"),
        StepConclusion::FinalDivision => {
            "final recomposition and reduced remainder verified".into()
        }
    }
}

pub fn division_trace_json(
    context: &EvaluationContext,
    names: &[&str],
    dividend: &str,
    divisors: &[&str],
) -> Result<String, AdvancedError> {
    use mathematishia_kernel::step_records::division_trace::record_ordered_division;
    if names.len() > 8 || divisors.len() > 8 {
        return Err(AdvancedError::Resource(
            "division trace input limit exceeded".into(),
        ));
    }
    if divisors.is_empty() {
        return Err(invalid("expected at least one divisor"));
    }
    let variables = names
        .iter()
        .map(|name| problem::variable(context, name))
        .collect::<Result<Vec<_>, _>>()?;
    if variables.iter().collect::<BTreeSet<_>>().len() != variables.len() {
        return Err(invalid("variables must be distinct"));
    }
    let limits = MultivariateLimits {
        max_variables: 8,
        max_terms: 64,
        max_input_terms: 4096,
        max_total_degree: 32,
        max_scalar_bits: 256,
        max_work_units: 250_000,
    };
    let convert = |input: &str| {
        let source = problem::source(input, ScalarDomain::Rational)?;
        if !source.conditions().is_empty() {
            return Err(AdvancedError::Domain(
                "division trace requires unrestricted polynomials".into(),
            ));
        }
        MultivariatePolynomial::try_from_expr(source.expression(), &variables, limits)
            .map_err(|error| groebner_error(GroebnerError::Expression(error)))
    };
    let dividend = convert(dividend)?;
    let divisors = divisors
        .iter()
        .map(|source| convert(source))
        .collect::<Result<Vec<_>, _>>()?;
    let limits = StepLimits::default();
    let trace = record_ordered_division(&dividend, &divisors, limits).map_err(step_error)?;
    trace
        .to_checked_json(&dividend, &divisors, limits)
        .map_err(wire_step_error)
}
