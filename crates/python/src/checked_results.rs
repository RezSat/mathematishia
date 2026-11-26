use mathema::ScalarDomain;
use mathema_frontend::problem::{self, RequestError};
use mathematishia_kernel::{
    EvaluationContext, ProblemError, RealRootIsolationError, verify_real_root_isolation,
};
use pyo3::prelude::*;

use super::{
    MathSyntaxError, ResourceLimitError, UnsupportedMathematicsError, VerificationFailure,
    map_domain_parse, map_evaluation_error,
};

fn request_error(error: RequestError) -> PyErr {
    match error {
        RequestError::Parse(error) => map_domain_parse(error),
        RequestError::Invalid(message) => MathSyntaxError::new_err(message),
        RequestError::Unsupported(message) => UnsupportedMathematicsError::new_err(message),
        RequestError::Evaluation(error)
        | RequestError::Problem(ProblemError::Evaluation(error)) => map_evaluation_error(error),
        RequestError::Wire(error) => ResourceLimitError::new_err(error.to_string()),
        RequestError::Problem(error) => VerificationFailure::new_err(error.to_string()),
    }
}

#[pyfunction]
fn problem_json(args: Vec<String>) -> PyResult<String> {
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    problem::evaluate_json(&EvaluationContext::default(), &args).map_err(request_error)
}

fn isolation_error(error: RealRootIsolationError) -> PyErr {
    match error {
        RealRootIsolationError::Evaluation(error) => map_evaluation_error(error),
        RealRootIsolationError::NonpositiveTargetWidth => {
            MathSyntaxError::new_err(error.to_string())
        }
        RealRootIsolationError::InvalidEvidence => VerificationFailure::new_err(error.to_string()),
        _ => UnsupportedMathematicsError::new_err(error.to_string()),
    }
}

type RootData = (String, String, String, String);

#[pyfunction]
fn isolate_real_roots(source: &str, variable: &str, target_width: &str) -> PyResult<Vec<RootData>> {
    let context = EvaluationContext::default();
    let source = problem::source(source, ScalarDomain::Real).map_err(request_error)?;
    let symbol = problem::variable(&context, variable).map_err(request_error)?;
    let width = problem::scalar(&context, target_width).map_err(request_error)?;
    let result = context
        .isolate_real_roots(&source, &symbol, &width)
        .map_err(isolation_error)?;
    verify_real_root_isolation(&context, &source, &symbol, &width, &result)
        .map_err(isolation_error)?;
    Ok(result
        .roots()
        .iter()
        .map(|root| {
            (
                root.lower().to_string(),
                root.upper().to_string(),
                root.midpoint().to_string(),
                root.error_bound().to_string(),
            )
        })
        .collect())
}

pub(super) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(problem_json, m)?)?;
    m.add_function(wrap_pyfunction!(isolate_real_roots, m)?)?;
    Ok(())
}
