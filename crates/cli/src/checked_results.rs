use std::fmt::Write as _;

use mathema::ScalarDomain;
use mathema_frontend::problem::{self, RequestError};
use mathematishia_kernel::{
    EvaluationContext, ProblemError, RealRootIsolationError, verify_real_root_isolation,
};

use super::real_analysis::{evaluation_error, parse_error};
use super::{CliError, CliErrorKind, invalid, problem_error};

pub(super) fn request_error(error: RequestError) -> CliError {
    match error {
        RequestError::Advanced(error) => super::advanced_operations::error(*error),
        RequestError::Parse(error) => parse_error(error),
        RequestError::Invalid(message) => invalid(message),
        RequestError::Unsupported(message) => problem_error(CliErrorKind::Unsupported, message),
        RequestError::Evaluation(error)
        | RequestError::Problem(ProblemError::Evaluation(error)) => evaluation_error(error),
        RequestError::Wire(error) => problem_error(CliErrorKind::Resource, error.to_string()),
        RequestError::Problem(error) => {
            problem_error(CliErrorKind::Verification, error.to_string())
        }
    }
}

pub(super) fn json(args: &[&str]) -> Result<String, CliError> {
    problem::evaluate_json(&EvaluationContext::default(), args).map_err(request_error)
}

fn isolation_error(error: RealRootIsolationError) -> CliError {
    match error {
        RealRootIsolationError::Evaluation(error) => evaluation_error(error),
        RealRootIsolationError::NonpositiveTargetWidth => invalid(error.to_string()),
        RealRootIsolationError::InvalidEvidence => {
            problem_error(CliErrorKind::Verification, error.to_string())
        }
        _ => problem_error(CliErrorKind::Unsupported, error.to_string()),
    }
}

pub(super) fn isolate(input: &str, name: &str, width: &str) -> Result<String, CliError> {
    let context = EvaluationContext::default();
    let source = problem::source(input, ScalarDomain::Real).map_err(request_error)?;
    let symbol = problem::variable(&context, name).map_err(request_error)?;
    let width = problem::scalar(&context, width).map_err(request_error)?;
    let result = context
        .isolate_real_roots(&source, &symbol, &width)
        .map_err(isolation_error)?;
    verify_real_root_isolation(&context, &source, &symbol, &width, &result)
        .map_err(isolation_error)?;
    let mut output = format!(
        "status: exact\ndomain: R\nroots: {}\ntarget_width: {width}",
        result.roots().len()
    );
    for root in result.roots() {
        write!(
            output,
            "\ninterval: ({}, {})\nmidpoint: {}\nerror_bound: {}",
            root.lower(),
            root.upper(),
            root.midpoint(),
            root.error_bound()
        )
        .expect("writing to String is infallible");
    }
    output.push_str("\ncomplete: true\nconditions: none\nverified: true");
    Ok(output)
}
