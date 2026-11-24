use std::fmt::Write as _;

use mathema::{DefinednessError, ScalarDomain, Symbol};
use mathema_frontend::{
    DomainParseError, ParseErrorKind, RealComparisonOperator, parse_domain_expression_in_domain,
    parse_real_comparison,
};
use mathematishia_kernel::{
    ComparisonOperator, EvaluationContext, EvaluationError, ExactQuadraticRoot, InequalityError,
    LimitDirection, LimitError, LimitValue, LimitVerificationError, RationalExpressionError,
    RationalFunction, RationalFunctionError, RealBound, RealInterval, verify_inequality_result,
    verify_rational_limit,
};

use super::{CliError, CliErrorKind, invalid, problem_error, scalar, variable};

fn evaluation_error(error: EvaluationError) -> CliError {
    let kind = match error {
        EvaluationError::Limit(_) => CliErrorKind::Resource,
        EvaluationError::RationalFunction(
            RationalFunctionError::ZeroDenominator
            | RationalFunctionError::ZeroDivisor
            | RationalFunctionError::EmptyDomain
            | RationalFunctionError::UndefinedAtPoint,
        ) => CliErrorKind::Domain,
        EvaluationError::AlgebraicVerification(_) | EvaluationError::Verification(_) => {
            CliErrorKind::Verification
        }
        _ => CliErrorKind::Unsupported,
    };
    problem_error(kind, error.to_string())
}

fn parse_error(error: DomainParseError) -> CliError {
    let kind = match &error {
        DomainParseError::Syntax(reason) => match reason.kind() {
            ParseErrorKind::InputLimitExceeded
            | ParseErrorKind::TokenLimitExceeded
            | ParseErrorKind::NestingLimitExceeded
            | ParseErrorKind::DecimalDigitLimitExceeded => CliErrorKind::Resource,
            _ => CliErrorKind::InvalidInput,
        },
        DomainParseError::Domain(DefinednessError::ZeroDenominator) => CliErrorKind::Domain,
        DomainParseError::Domain(
            DefinednessError::NodeLimitExceeded | DefinednessError::ConditionLimitExceeded,
        ) => CliErrorKind::Resource,
        DomainParseError::Domain(_) => CliErrorKind::Unsupported,
    };
    problem_error(kind, error.to_string())
}

fn conversion_error(error: RationalExpressionError) -> CliError {
    match error {
        RationalExpressionError::Evaluation(reason) => evaluation_error(reason),
        RationalExpressionError::ExponentLimitExceeded => {
            problem_error(CliErrorKind::Resource, error.to_string())
        }
        RationalExpressionError::RationalFunction(
            RationalFunctionError::ZeroDenominator
            | RationalFunctionError::ZeroDivisor
            | RationalFunctionError::EmptyDomain
            | RationalFunctionError::UndefinedAtPoint,
        ) => problem_error(CliErrorKind::Domain, error.to_string()),
        _ => problem_error(CliErrorKind::Unsupported, error.to_string()),
    }
}

fn limit_verification_error(error: LimitVerificationError) -> CliError {
    match error {
        LimitVerificationError::Evaluation(reason) => evaluation_error(reason),
        LimitVerificationError::InvalidCertificate => {
            problem_error(CliErrorKind::Verification, error.to_string())
        }
    }
}

fn limit_error(error: LimitError) -> CliError {
    match error {
        LimitError::Evaluation(reason) => evaluation_error(reason),
        LimitError::Conversion(reason) => conversion_error(reason),
        LimitError::Verification(reason) => limit_verification_error(reason),
        LimitError::InconsistentFactorization => {
            problem_error(CliErrorKind::Verification, error.to_string())
        }
    }
}

fn inequality_error(error: InequalityError) -> CliError {
    match error {
        InequalityError::Evaluation(reason) => evaluation_error(reason),
        InequalityError::Conversion(reason) => conversion_error(reason),
        InequalityError::UnsupportedDegree => {
            problem_error(CliErrorKind::Unsupported, error.to_string())
        }
        InequalityError::IncorrectCertificate => {
            problem_error(CliErrorKind::Verification, error.to_string())
        }
    }
}

fn append_exclusions(output: &mut String, source: &RationalFunction, symbol: &Symbol) {
    if source.excluded_zero_sets().is_empty() {
        output.push_str("\nconditions: none");
    } else {
        for condition in source.excluded_zero_sets() {
            write!(output, "\ncondition: {} != 0", condition.to_expr(symbol))
                .expect("writing to String is infallible");
        }
    }
}

pub(super) fn limit(input: &str, name: &str, point: &str, side: &str) -> Result<String, CliError> {
    let symbol = variable(name)?;
    let direction = match side {
        "left" => LimitDirection::Left,
        "right" => LimitDirection::Right,
        "two-sided" => LimitDirection::TwoSided,
        _ => return Err(invalid("direction must be left, right or two-sided")),
    };
    if matches!(point, "infinity" | "-infinity" | "inf" | "-inf" | "∞" | "-∞") {
        return Err(problem_error(
            CliErrorKind::Unsupported,
            "limits at infinity are not supported",
        ));
    }
    let at = scalar(point)?;
    let ctx = EvaluationContext::default();
    let source = parse_domain_expression_in_domain(input, ScalarDomain::Real).map_err(parse_error)?;
    let result = ctx
        .rational_expression_limit(&source, &symbol, &at, direction)
        .map_err(limit_error)?;
    verify_rational_limit(&ctx, result.source(), &at, direction, &result)
        .map_err(limit_verification_error)?;
    let value = match result.value() {
        LimitValue::Finite(value) => value.to_string(),
        LimitValue::PositiveInfinity => "infinity".to_owned(),
        LimitValue::NegativeInfinity => "-infinity".to_owned(),
        LimitValue::DoesNotExist => "does not exist".to_owned(),
    };
    let mut output = format!(
        "status: exact\ndomain: R\nlimit: {value}\nat: {at}\ndirection: {side}\nexcluded_at_point: {}",
        result.excluded_at_point()
    );
    append_exclusions(&mut output, result.source(), &symbol);
    output.push_str("\nverified: true");
    Ok(output)
}

pub(super) fn is_comparison(input: &str) -> bool {
    input.chars().any(|c| matches!(c, '<' | '>' | '≤' | '≥'))
}

fn bound_text(bound: &RealBound, symbol: &Symbol) -> String {
    match bound {
        RealBound::NegativeInfinity => "-infinity".to_owned(),
        RealBound::PositiveInfinity => "infinity".to_owned(),
        RealBound::Finite(ExactQuadraticRoot::Rational(value)) => value.to_string(),
        RealBound::Finite(ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch,
        }) => format!(
            "{} of minimal polynomial {}",
            branch.stable_id(),
            minimal_polynomial.to_expr(symbol)
        ),
    }
}

fn interval_text(interval: &RealInterval, symbol: &Symbol) -> String {
    format!(
        "{}{}, {}{}",
        if interval.lower_closed { "[" } else { "(" },
        bound_text(&interval.lower, symbol),
        bound_text(&interval.upper, symbol),
        if interval.upper_closed { "]" } else { ")" }
    )
}

pub(super) fn inequality(input: &str, name: &str) -> Result<String, CliError> {
    let symbol = variable(name)?;
    let parsed = parse_real_comparison(input).map_err(parse_error)?;
    let comparison = match parsed.operator {
        RealComparisonOperator::Less => ComparisonOperator::Less,
        RealComparisonOperator::LessOrEqual => ComparisonOperator::LessOrEqual,
        RealComparisonOperator::Greater => ComparisonOperator::Greater,
        RealComparisonOperator::GreaterOrEqual => ComparisonOperator::GreaterOrEqual,
    };
    let ctx = EvaluationContext::default();
    let result = ctx
        .solve_expression_inequality(&parsed.left, &parsed.right, &symbol, comparison)
        .map_err(inequality_error)?;
    if !verify_inequality_result(&ctx, &result).map_err(inequality_error)? {
        return Err(problem_error(
            CliErrorKind::Verification,
            "inequality certificate failed verification",
        ));
    }
    let mut output = format!("status: exact\ndomain: R\nintervals: {}", result.intervals().len());
    for interval in result.intervals() {
        write!(output, "\ninterval: {}", interval_text(interval, &symbol))
            .expect("writing to String is infallible");
    }
    append_exclusions(&mut output, result.function(), &symbol);
    output.push_str("\nverified: true");
    Ok(output)
}
