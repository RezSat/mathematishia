use std::fmt::Write as _;

use mathema::Rational;
use mathema_frontend::advanced::{self, AdvancedError};
use mathematishia_kernel::{
    EvaluationContext, ExactQuadraticRoot, IdealMembership, SpecialAngleOutcome,
};

use super::checked_results::request_error;
use super::{CliError, CliErrorKind, problem_error};

fn error(error: AdvancedError) -> CliError {
    match error {
        AdvancedError::Input(error) => request_error(error),
        AdvancedError::Resource(message) => problem_error(CliErrorKind::Resource, message),
        AdvancedError::Unsupported(message) => problem_error(CliErrorKind::Unsupported, message),
        AdvancedError::Domain(message) => problem_error(CliErrorKind::Domain, message),
        AdvancedError::Verification(message) => problem_error(CliErrorKind::Verification, message),
    }
}

pub(super) fn basis(names: &str, generators: &[&str]) -> Result<String, CliError> {
    let context = EvaluationContext::default();
    let names = names.split(',').collect::<Vec<_>>();
    let result = advanced::basis(&context, &names, generators).map_err(error)?;
    let mut output = format!(
        "status: exact\ndomain: Q[variables]\nbasis_count: {}",
        result.basis.len()
    );
    for polynomial in result.basis {
        write!(output, "\nbasis: {polynomial}").unwrap();
    }
    output.push_str("\nverified: true");
    Ok(output)
}

pub(super) fn membership(
    names: &str,
    generators: &[&str],
    target: &str,
) -> Result<String, CliError> {
    let names = names.split(',').collect::<Vec<_>>();
    let result = advanced::membership(&EvaluationContext::default(), &names, generators, target)
        .map_err(error)?;
    let (member, reduction) = match result {
        IdealMembership::Member(reduction) => (true, reduction),
        IdealMembership::NonMember(reduction) => (false, reduction),
    };
    Ok(format!(
        "status: exact\nmember: {member}\nremainder: {}\nverified: true",
        reduction.remainder
    ))
}

pub(super) fn trig(function: &str, coefficient: &str) -> Result<String, CliError> {
    let result = advanced::special_angle(&EvaluationContext::default(), function, coefficient)
        .map_err(error)?;
    Ok(match result.outcome {
        SpecialAngleOutcome::UndefinedTangentPole => {
            "status: undefined\nreason: tangent pole\nverified: true".into()
        }
        SpecialAngleOutcome::Exact(ExactQuadraticRoot::Rational(value)) => {
            format!("status: exact\nvalue: {value}\nverified: true")
        }
        SpecialAngleOutcome::Exact(ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch,
        }) => format!(
            "status: exact\nminimal_polynomial: {}\nbranch: {}\nverified: true",
            minimal_polynomial.to_expr(&mathema::Symbol::new("t").unwrap()),
            branch.stable_id()
        ),
    })
}

pub(super) fn absolute(source: &str, name: &str) -> Result<String, CliError> {
    let result = advanced::absolute(&EvaluationContext::default(), source, name).map_err(error)?;
    let mut output = format!(
        "status: exact\ndomain: R\ncomponents: {}",
        result.solutions.len()
    );
    let bound = |value: Option<Rational>, infinity: &str| {
        value.map_or_else(|| infinity.to_owned(), |value| value.to_string())
    };
    for interval in result.solutions {
        let left = if interval.lower.is_some() { '[' } else { '(' };
        let right = if interval.upper.is_some() { ']' } else { ')' };
        write!(
            output,
            "\ninterval: {left}{}, {}{right}",
            bound(interval.lower, "-infinity"),
            bound(interval.upper, "infinity")
        )
        .unwrap();
    }
    output.push_str("\ncomplete: true\nverified: true");
    Ok(output)
}

pub(super) fn calculus(
    source: &str,
    candidate: &str,
    name: &str,
    relation: &str,
) -> Result<String, CliError> {
    let certificate = advanced::calculus(
        &EvaluationContext::default(),
        source,
        candidate,
        name,
        relation,
    )
    .map_err(error)?;
    let mut output = format!("status: exact\nrelation: {relation}\nverified: true");
    for condition in certificate.value.excluded_zero_sets() {
        write!(
            output,
            "\ncondition: {} != 0",
            condition.to_expr(&certificate.variable)
        )
        .unwrap();
    }
    Ok(output)
}

pub(super) fn steps(kind: &str, source: &str, name: &str) -> Result<String, CliError> {
    let result =
        advanced::derivation(&EvaluationContext::default(), kind, source, name).map_err(error)?;
    let mut output = format!("steps: {}", result.steps.len());
    for step in result.steps {
        write!(
            output,
            "\n{}. {}: {:?}; premises: {:?}; conditions: {:?}",
            step.index,
            step.rule.stable_id(),
            step.claim,
            step.premises,
            step.conditions
        )
        .unwrap();
    }
    for condition in result.source_conditions {
        write!(output, "\nsource condition: {condition:?}").unwrap();
    }
    write!(
        output,
        "\nconclusion: {:?}\nverified: true",
        result.conclusion
    )
    .unwrap();
    Ok(output)
}
