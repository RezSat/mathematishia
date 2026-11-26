use super::{
    MathSyntaxError, ResourceLimitError, UndefinedAtPointError, UnsupportedMathematicsError,
    VerificationFailure, checked_domain, explicit_variable, map_domain_parse, map_evaluation_error,
    matrix_error, selected_unrestricted_polynomial, unrestricted_scalar,
};
use mathema::{Equation, Rational, Symbol};
use mathema_frontend::{DomainStatement, parse_domain_statement};
use mathematishia_kernel::{
    EvaluationContext, ExpressionLinearSystemSet, LinearSystemExprError, MultivariateDivisionError,
    MultivariateError, MultivariateExprError, MultivariateLimits, MultivariatePolynomial,
    RootIntervalOutcome, RootRefinementError, RootRefinementVerificationError, divide_multivariate,
    solve_linear_system_expr, verify_multivariate_division, verify_root_refinement,
};
use pyo3::prelude::*;

fn system_error(error: LinearSystemExprError) -> PyErr {
    match error {
        LinearSystemExprError::Evaluation(reason) => map_evaluation_error(reason),
        LinearSystemExprError::Matrix(reason) => matrix_error(reason),
        LinearSystemExprError::DimensionLimit => ResourceLimitError::new_err(error.to_string()),
        LinearSystemExprError::VerificationFailed => {
            VerificationFailure::new_err(error.to_string())
        }
        _ => UnsupportedMathematicsError::new_err(error.to_string()),
    }
}

fn unrestricted_equation(source: &str, ctx: &EvaluationContext) -> PyResult<Equation> {
    let DomainStatement::Equation { left, right } =
        parse_domain_statement(source).map_err(map_domain_parse)?
    else {
        return Err(MathSyntaxError::new_err(
            "expected an equation containing '='",
        ));
    };
    if !left.conditions().is_empty() || !right.conditions().is_empty() {
        return Err(UnsupportedMathematicsError::new_err(
            "affine systems cannot discard source-domain restrictions",
        ));
    }
    ctx.check_expression(left.expression())
        .map_err(map_evaluation_error)?;
    ctx.check_expression(right.expression())
        .map_err(map_evaluation_error)?;
    Ok(Equation::new(
        left.expression().clone(),
        right.expression().clone(),
    ))
}

type SystemData = (String, Vec<String>, Vec<Vec<String>>);

#[pyfunction]
fn solve_linear_system(equations: Vec<String>, variables: Vec<String>) -> PyResult<SystemData> {
    if equations.len() > 16 || variables.len() > 16 {
        return Err(ResourceLimitError::new_err(
            "system exceeds the 16 by 16 limit",
        ));
    }
    let ctx = EvaluationContext::default();
    let variables = variables
        .iter()
        .map(|name| explicit_variable(name))
        .collect::<PyResult<Vec<_>>>()?;
    let equations = equations
        .iter()
        .map(|s| unrestricted_equation(s, &ctx))
        .collect::<PyResult<Vec<_>>>()?;
    let solution = solve_linear_system_expr(&ctx, &equations, &variables).map_err(system_error)?;
    let strings = |values: &[Rational]| values.iter().map(ToString::to_string).collect::<Vec<_>>();
    Ok(match solution.set() {
        ExpressionLinearSystemSet::Empty => ("inconsistent".into(), vec![], vec![]),
        ExpressionLinearSystemSet::Unique(values) => ("unique".into(), strings(values), vec![]),
        ExpressionLinearSystemSet::Affine { particular, basis } => (
            "infinite".into(),
            strings(particular),
            basis.iter().map(|v| strings(v)).collect(),
        ),
    })
}

fn polynomial_error(error: MultivariateError) -> PyErr {
    match error {
        MultivariateError::VariableLimit
        | MultivariateError::TotalDegreeLimit
        | MultivariateError::TermLimit
        | MultivariateError::InputTermLimit
        | MultivariateError::ScalarBitLimit
        | MultivariateError::WorkLimit => ResourceLimitError::new_err(error.to_string()),
        _ => UnsupportedMathematicsError::new_err(error.to_string()),
    }
}

fn bridge_error(error: MultivariateExprError) -> PyErr {
    match error {
        MultivariateExprError::Polynomial(reason) => polynomial_error(reason),
        MultivariateExprError::ExponentLimit
        | MultivariateExprError::ExpressionDepthLimit
        | MultivariateExprError::ExpressionNodeLimit
        | MultivariateExprError::ConditionLimit => ResourceLimitError::new_err(error.to_string()),
        _ => UnsupportedMathematicsError::new_err(error.to_string()),
    }
}

fn division_error(error: MultivariateDivisionError) -> PyErr {
    match error {
        MultivariateDivisionError::Polynomial(reason) => polynomial_error(reason),
        MultivariateDivisionError::TooManyDivisors | MultivariateDivisionError::ResourceLimit => {
            ResourceLimitError::new_err(error.to_string())
        }
        MultivariateDivisionError::ZeroDivisor => UndefinedAtPointError::new_err(error.to_string()),
        MultivariateDivisionError::InvalidCertificate
        | MultivariateDivisionError::NondecreasingLeadingTerm => {
            VerificationFailure::new_err(error.to_string())
        }
        _ => UnsupportedMathematicsError::new_err(error.to_string()),
    }
}

fn ring_polynomial(
    source: &str,
    variables: &[Symbol],
    limits: MultivariateLimits,
) -> PyResult<MultivariatePolynomial> {
    let value = checked_domain(source)?;
    if !value.conditions().is_empty() {
        return Err(UnsupportedMathematicsError::new_err(
            "polynomial division cannot discard source-domain restrictions",
        ));
    }
    MultivariatePolynomial::try_from_expr(value.expression(), variables, limits)
        .map_err(bridge_error)
}

type Terms = Vec<(Vec<u32>, String)>;
type DivisionData = (Vec<String>, Vec<Terms>, Terms, usize);

#[pyfunction(name = "divide_multivariate")]
fn py_divide_multivariate(
    dividend: &str,
    divisors: Vec<String>,
    variables: Vec<String>,
) -> PyResult<DivisionData> {
    if variables.len() > MultivariateLimits::HARD_LIMITS.max_variables {
        return Err(ResourceLimitError::new_err(
            "ambient ring exceeds the variable limit",
        ));
    }
    if divisors.len() > 32 {
        return Err(ResourceLimitError::new_err(
            "multivariate divisor count limit exceeded",
        ));
    }
    let variables = variables
        .iter()
        .map(|name| explicit_variable(name))
        .collect::<PyResult<Vec<_>>>()?;
    let limits = MultivariateLimits::default();
    let dividend = ring_polynomial(dividend, &variables, limits)?;
    let divisors = divisors
        .iter()
        .map(|s| ring_polynomial(s, &variables, limits))
        .collect::<PyResult<Vec<_>>>()?;
    let result = divide_multivariate(&dividend, &divisors, limits).map_err(division_error)?;
    if !verify_multivariate_division(&dividend, &divisors, &result, limits)
        .map_err(division_error)?
    {
        return Err(VerificationFailure::new_err(
            "multivariate division failed verification",
        ));
    }
    let terms = |poly: &MultivariatePolynomial| {
        poly.terms()
            .iter()
            .map(|(powers, coefficient)| (powers.clone(), coefficient.to_string()))
            .collect::<Terms>()
    };
    Ok((
        dividend
            .variables()
            .iter()
            .map(ToString::to_string)
            .collect(),
        result.quotients().iter().map(terms).collect(),
        terms(result.remainder()),
        result.steps(),
    ))
}

type BracketData = (
    String,
    Option<String>,
    Option<(String, String)>,
    usize,
    bool,
);

#[pyfunction]
fn refine_real_root_bracket(
    source: &str,
    variable: &str,
    lower: &str,
    upper: &str,
    target_width: &str,
    max_bisections: usize,
) -> PyResult<BracketData> {
    let symbol = explicit_variable(variable)?;
    let polynomial = selected_unrestricted_polynomial(source, &symbol)?;
    let lower = unrestricted_scalar(lower)?;
    let upper = unrestricted_scalar(upper)?;
    let width = unrestricted_scalar(target_width)?;
    let ctx = EvaluationContext::default();
    let result = ctx
        .refine_real_root_bracket(&polynomial, &lower, &upper, &width, max_bisections)
        .map_err(|error| match error {
            RootRefinementError::Evaluation(reason) => map_evaluation_error(reason),
            _ => UnsupportedMathematicsError::new_err(error.to_string()),
        })?;
    verify_root_refinement(
        &ctx,
        &polynomial,
        &lower,
        &upper,
        &width,
        max_bisections,
        &result,
    )
    .map_err(|error| match error {
        RootRefinementVerificationError::Evaluation(reason) => map_evaluation_error(reason),
        _ => VerificationFailure::new_err(error.to_string()),
    })?;
    let (kind, root, interval) = match result.outcome() {
        RootIntervalOutcome::ExactRoot(root) => ("exact", Some(root.to_string()), None),
        RootIntervalOutcome::SignChange { lower, upper } => (
            "sign_change",
            None,
            Some((lower.to_string(), upper.to_string())),
        ),
        RootIntervalOutcome::Unresolved { lower, upper } => (
            "unresolved",
            None,
            Some((lower.to_string(), upper.to_string())),
        ),
    };
    Ok((
        kind.into(),
        root,
        interval,
        result.bisections(),
        result.precision_met(),
    ))
}

pub(super) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(solve_linear_system, m)?)?;
    m.add_function(wrap_pyfunction!(py_divide_multivariate, m)?)?;
    m.add_function(wrap_pyfunction!(refine_real_root_bracket, m)?)?;
    Ok(())
}
