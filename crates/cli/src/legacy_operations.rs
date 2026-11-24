use std::fmt::Write as _;

use mathema::{Equation, ScalarDomain, Symbol};
use mathema_frontend::{DomainStatement, parse_domain_statement_in_domain};
use mathematishia_kernel::{
    EvaluationContext, ExpressionLinearSystemSet, LinearSystemExprError, MatrixError,
    MultivariateDivisionError, MultivariateError, MultivariateExprError, MultivariateLimits,
    MultivariatePolynomial, RootIntervalOutcome, RootRefinementError,
    RootRefinementVerificationError, divide_multivariate, solve_linear_system_expr,
    verify_linear_system_expr, verify_multivariate_division, verify_root_refinement,
};

use super::real_analysis::{
    check_unrestricted, evaluation_error, exact_real_scalar, parse_error, strict_variable,
    unrestricted_real_expression,
};
use super::{CliError, CliErrorKind, format_vector, invalid, problem_error};

fn variables(ctx: &EvaluationContext, names: &str) -> Result<Vec<Symbol>, CliError> {
    let mut result = Vec::new();
    for name in names.split(',') {
        if result.len() == 16 {
            return Err(problem_error(
                CliErrorKind::Resource,
                "variable limit exceeded",
            ));
        }
        let symbol = strict_variable(ctx, name)?;
        if result.contains(&symbol) {
            return Err(invalid("variables must be distinct"));
        }
        result.push(symbol);
    }
    Ok(result)
}

fn variable_text(variables: &[Symbol]) -> String {
    variables
        .iter()
        .map(Symbol::name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn system_error(error: LinearSystemExprError) -> CliError {
    let kind = match error {
        LinearSystemExprError::Evaluation(reason)
        | LinearSystemExprError::Matrix(MatrixError::Evaluation(reason)) => {
            return evaluation_error(reason);
        }
        LinearSystemExprError::DimensionLimit
        | LinearSystemExprError::Matrix(MatrixError::DimensionLimit) => CliErrorKind::Resource,
        LinearSystemExprError::VerificationFailed => CliErrorKind::Verification,
        LinearSystemExprError::EmptyEquations
        | LinearSystemExprError::EmptyVariables
        | LinearSystemExprError::DuplicateVariable
        | LinearSystemExprError::Matrix(_) => CliErrorKind::InvalidInput,
        _ => CliErrorKind::Unsupported,
    };
    problem_error(kind, error.to_string())
}

pub(super) fn linear_system(names: &str, inputs: &[&str]) -> Result<String, CliError> {
    if inputs.is_empty() {
        return Err(invalid("at least one equation is required"));
    }
    if inputs.len() > 16 {
        return Err(problem_error(
            CliErrorKind::Resource,
            "equation limit exceeded",
        ));
    }
    let ctx = EvaluationContext::default();
    let variables = variables(&ctx, names)?;
    let mut equations = Vec::with_capacity(inputs.len());
    for input in inputs {
        let statement =
            parse_domain_statement_in_domain(input, ScalarDomain::Real).map_err(parse_error)?;
        let DomainStatement::Equation { left, right } = statement else {
            return Err(invalid("expected an equation containing '='"));
        };
        check_unrestricted(&ctx, &left)?;
        check_unrestricted(&ctx, &right)?;
        equations.push(Equation::new(
            left.expression().clone(),
            right.expression().clone(),
        ));
    }
    let answer = solve_linear_system_expr(&ctx, &equations, &variables).map_err(system_error)?;
    if !verify_linear_system_expr(&ctx, &equations, &variables, &answer).map_err(system_error)? {
        return Err(problem_error(
            CliErrorKind::Verification,
            "linear system verification failed",
        ));
    }
    let mut output = format!(
        "status: exact\ndomain: Q\nvariables: {}",
        variable_text(answer.variables())
    );
    match answer.set() {
        ExpressionLinearSystemSet::Empty => output.push_str("\nsolution: empty"),
        ExpressionLinearSystemSet::Unique(values) => {
            write!(
                output,
                "\nsolution: unique\nvalues: {}",
                format_vector(values)
            )
            .expect("writing to String is infallible");
        }
        ExpressionLinearSystemSet::Affine { particular, basis } => {
            write!(
                output,
                "\nsolution: affine\nparticular: {}",
                format_vector(particular)
            )
            .expect("writing to String is infallible");
            for direction in basis {
                write!(output, "\nbasis: {}", format_vector(direction))
                    .expect("writing to String is infallible");
            }
        }
    }
    output.push_str("\nconditions: none\nverified: true");
    Ok(output)
}

fn multivariate_error(error: MultivariateError) -> CliError {
    let kind = match error {
        MultivariateError::VariableLimit
        | MultivariateError::TotalDegreeLimit
        | MultivariateError::TermLimit
        | MultivariateError::InputTermLimit
        | MultivariateError::ScalarBitLimit
        | MultivariateError::WorkLimit => CliErrorKind::Resource,
        _ => CliErrorKind::InvalidInput,
    };
    problem_error(kind, error.to_string())
}

fn bridge_error(error: MultivariateExprError) -> CliError {
    let kind = match error {
        MultivariateExprError::Polynomial(reason) => return multivariate_error(reason),
        MultivariateExprError::ExponentLimit
        | MultivariateExprError::ExpressionDepthLimit
        | MultivariateExprError::ExpressionNodeLimit
        | MultivariateExprError::ConditionLimit => CliErrorKind::Resource,
        _ => CliErrorKind::Unsupported,
    };
    problem_error(kind, error.to_string())
}

fn division_error(error: MultivariateDivisionError) -> CliError {
    let kind = match error {
        MultivariateDivisionError::Polynomial(reason) => return multivariate_error(reason),
        MultivariateDivisionError::TooManyDivisors | MultivariateDivisionError::ResourceLimit => {
            CliErrorKind::Resource
        }
        MultivariateDivisionError::ZeroDivisor => CliErrorKind::Domain,
        MultivariateDivisionError::NoDivisors => CliErrorKind::InvalidInput,
        MultivariateDivisionError::NondecreasingLeadingTerm
        | MultivariateDivisionError::InvalidCertificate => CliErrorKind::Verification,
    };
    problem_error(kind, error.to_string())
}

pub(super) fn multivariate_division(
    names: &str,
    dividend: &str,
    divisors: &[&str],
) -> Result<String, CliError> {
    if divisors.is_empty() {
        return Err(invalid("at least one divisor is required"));
    }
    if divisors.len() > 32 {
        return Err(problem_error(
            CliErrorKind::Resource,
            "divisor limit exceeded",
        ));
    }
    let ctx = EvaluationContext::default();
    let variables = variables(&ctx, names)?;
    let limits = MultivariateLimits::default();
    let convert = |input: &str| {
        let source = unrestricted_real_expression(&ctx, input)?;
        MultivariatePolynomial::try_from_expr(source.expression(), &variables, limits)
            .map_err(bridge_error)
    };
    let dividend = convert(dividend)?;
    let divisors = divisors
        .iter()
        .map(|input| convert(input))
        .collect::<Result<Vec<_>, _>>()?;
    let answer = divide_multivariate(&dividend, &divisors, limits).map_err(division_error)?;
    if !verify_multivariate_division(&dividend, &divisors, &answer, limits)
        .map_err(division_error)?
    {
        return Err(problem_error(
            CliErrorKind::Verification,
            "multivariate division verification failed",
        ));
    }
    let mut output = format!(
        "status: exact\ndomain: Q[variables]\nvariables: {}\nmonomial_order: lexicographic",
        variable_text(dividend.variables())
    );
    for (index, quotient) in answer.quotients().iter().enumerate() {
        write!(
            output,
            "\nquotient {}: {}",
            index + 1,
            quotient.to_expr(limits).map_err(bridge_error)?
        )
        .expect("writing to String is infallible");
    }
    write!(
        output,
        "\nremainder: {}\nconditions: none\nverified: true",
        answer.remainder().to_expr(limits).map_err(bridge_error)?
    )
    .expect("writing to String is infallible");
    Ok(output)
}

fn bracket_error(error: RootRefinementError) -> CliError {
    match error {
        RootRefinementError::Evaluation(reason) => evaluation_error(reason),
        RootRefinementError::ZeroPolynomial => {
            problem_error(CliErrorKind::Unsupported, error.to_string())
        }
        _ => invalid(error.to_string()),
    }
}

fn bracket_verification_error(error: RootRefinementVerificationError) -> CliError {
    match error {
        RootRefinementVerificationError::Evaluation(reason) => evaluation_error(reason),
        _ => problem_error(CliErrorKind::Verification, error.to_string()),
    }
}

pub(super) fn root_bracket(
    input: &str,
    name: &str,
    lower: &str,
    upper: &str,
    width: &str,
    max: &str,
) -> Result<String, CliError> {
    let ctx = EvaluationContext::default();
    let symbol = strict_variable(&ctx, name)?;
    let source = unrestricted_real_expression(&ctx, input)?;
    let polynomial = ctx
        .polynomial(source.expression(), &symbol)
        .map_err(evaluation_error)?;
    let lower = exact_real_scalar(&ctx, lower)?;
    let upper = exact_real_scalar(&ctx, upper)?;
    let width = exact_real_scalar(&ctx, width)?;
    let max = max
        .parse::<usize>()
        .map_err(|_| invalid("expected a nonnegative bisection count"))?;
    let answer = ctx
        .refine_real_root_bracket(&polynomial, &lower, &upper, &width, max)
        .map_err(bracket_error)?;
    verify_root_refinement(&ctx, &polynomial, &lower, &upper, &width, max, &answer)
        .map_err(bracket_verification_error)?;
    let mut output = match answer.outcome() {
        RootIntervalOutcome::ExactRoot(root) => {
            format!("status: exact\ndomain: R\nroot: {root}\nclaim: one rational root")
        }
        RootIntervalOutcome::SignChange { lower, upper } => format!(
            "status: partial\ndomain: R\ninterval: ({lower}, {upper})\nclaim: at least one root; uniqueness not established"
        ),
        RootIntervalOutcome::Unresolved { lower, upper } => format!(
            "status: partial\ndomain: R\ninterval: ({lower}, {upper})\nclaim: unresolved; neither existence nor absence established"
        ),
    };
    write!(
        output,
        "\nbisections: {}\nprecision_met: {}\nconditions: none\nverified: true",
        answer.bisections(),
        answer.precision_met()
    )
    .expect("writing to String is infallible");
    Ok(output)
}
