use mathema_frontend::advanced::{self, AdvancedError};
use mathematishia_kernel::{
    EvaluationContext, ExactQuadraticRoot, IdealMembership, SpecialAngleOutcome,
};
use pyo3::prelude::*;

use super::checked_results::request_error;
use super::poly_data;
use super::{
    ResourceLimitError, UndefinedAtPointError, UnsupportedMathematicsError, VerificationFailure,
};

fn error(error: AdvancedError) -> PyErr {
    match error {
        AdvancedError::Input(error) => request_error(error),
        AdvancedError::Resource(message) => ResourceLimitError::new_err(message),
        AdvancedError::Unsupported(message) => UnsupportedMathematicsError::new_err(message),
        AdvancedError::Domain(message) => UndefinedAtPointError::new_err(message),
        AdvancedError::Verification(message) => VerificationFailure::new_err(message),
    }
}

type BasisData = (Vec<String>, Vec<String>);

#[pyfunction]
fn groebner(variables: Vec<String>, generators: Vec<String>) -> PyResult<BasisData> {
    let variables = variables.iter().map(String::as_str).collect::<Vec<_>>();
    let generators = generators.iter().map(String::as_str).collect::<Vec<_>>();
    let result =
        advanced::basis(&EvaluationContext::default(), &variables, &generators).map_err(error)?;
    Ok((
        result.variables.iter().map(ToString::to_string).collect(),
        result
            .basis
            .iter()
            .map(advanced::polynomial_text)
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?,
    ))
}

#[pyfunction]
fn ideal_membership(
    variables: Vec<String>,
    generators: Vec<String>,
    target: &str,
) -> PyResult<(bool, String)> {
    let variables = variables.iter().map(String::as_str).collect::<Vec<_>>();
    let generators = generators.iter().map(String::as_str).collect::<Vec<_>>();
    let result = advanced::membership(
        &EvaluationContext::default(),
        &variables,
        &generators,
        target,
    )
    .map_err(error)?;
    Ok(match result {
        IdealMembership::Member(reduction) => (
            true,
            advanced::polynomial_text(&reduction.remainder).map_err(error)?,
        ),
        IdealMembership::NonMember(reduction) => (
            false,
            advanced::polynomial_text(&reduction.remainder).map_err(error)?,
        ),
    })
}

type PolynomialData = (String, Vec<String>);
type AngleData = (String, Option<String>, Option<PolynomialData>, Option<String>);

#[pyfunction]
fn special_angle(function: &str, coefficient: &str) -> PyResult<AngleData> {
    let result = advanced::special_angle(&EvaluationContext::default(), function, coefficient)
        .map_err(error)?;
    Ok(match result.outcome {
        SpecialAngleOutcome::UndefinedTangentPole => ("undefined".into(), None, None, None),
        SpecialAngleOutcome::Exact(ExactQuadraticRoot::Rational(value)) => {
            ("rational".into(), Some(value.to_string()), None, None)
        }
        SpecialAngleOutcome::Exact(ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch,
        }) => (
            "real_quadratic".into(),
            None,
            Some(poly_data(
                &minimal_polynomial,
                &mathema::Symbol::new("t").unwrap(),
            )),
            Some(branch.stable_id().into()),
        ),
    })
}

type IntervalData = (Option<String>, Option<String>);

#[pyfunction]
fn solve_absolute(source: &str, variable: &str) -> PyResult<Vec<IntervalData>> {
    let result =
        advanced::absolute(&EvaluationContext::default(), source, variable).map_err(error)?;
    Ok(result
        .solutions
        .into_iter()
        .map(|interval| {
            (
                interval.lower.map(|v| v.to_string()),
                interval.upper.map(|v| v.to_string()),
            )
        })
        .collect())
}

#[pyfunction]
fn check_calculus(
    source: &str,
    candidate: &str,
    variable: &str,
    relation: &str,
) -> PyResult<Vec<PolynomialData>> {
    let result = advanced::calculus(
        &EvaluationContext::default(),
        source,
        candidate,
        variable,
        relation,
    )
    .map_err(error)?;
    Ok(result
        .value
        .excluded_zero_sets()
        .iter()
        .map(|poly| poly_data(poly, &result.variable))
        .collect())
}

type RecordData = (usize, String, String, Vec<usize>, Vec<String>);
type DerivationData = (Vec<String>, Vec<RecordData>, String);

#[pyfunction]
fn derivation_records(kind: &str, source: &str, variable: &str) -> PyResult<DerivationData> {
    let result = advanced::derivation(&EvaluationContext::default(), kind, source, variable)
        .map_err(error)?;
    Ok((
        result
            .source_conditions
            .iter()
            .map(advanced::condition_text)
            .collect(),
        result
            .steps
            .into_iter()
            .map(|step| {
                (
                    step.index,
                    step.rule.stable_id().into(),
                    advanced::claim_text(&step.claim),
                    step.premises,
                    step.conditions
                        .iter()
                        .map(advanced::step_condition_text)
                        .collect(),
                )
            })
            .collect(),
        advanced::conclusion_text(&result.conclusion),
    ))
}

#[pyfunction]
fn trig_zero_examples(function: &str) -> PyResult<Vec<String>> {
    Ok(advanced::trig_zeros(function)
        .map_err(error)?
        .iter()
        .map(ToString::to_string)
        .collect())
}

pub(super) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(trig_zero_examples, m)?)?;
    m.add_function(wrap_pyfunction!(groebner, m)?)?;
    m.add_function(wrap_pyfunction!(ideal_membership, m)?)?;
    m.add_function(wrap_pyfunction!(special_angle, m)?)?;
    m.add_function(wrap_pyfunction!(solve_absolute, m)?)?;
    m.add_function(wrap_pyfunction!(check_calculus, m)?)?;
    m.add_function(wrap_pyfunction!(derivation_records, m)?)?;
    Ok(())
}
