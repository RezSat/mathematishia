use mathema::{ScalarDomain, Symbol};
use mathema_frontend::{
    RealComparisonOperator, parse_domain_expression_in_domain, parse_real_comparison,
};
use mathematishia_kernel::{
    ComparisonOperator, EvaluationContext, EvaluationError, ExactQuadraticRoot, InequalityError,
    LimitDirection, LimitError, LimitFactor, LimitValue, LimitVerificationError,
    RationalExpressionError, RationalFunctionError, RealBound, verify_rational_limit,
};
use pyo3::prelude::*;

use super::{
    MathSyntaxError, PyAlgebraicValue, PyPolynomialData, PyRationalFunctionData,
    ResourceLimitError, UndefinedAtPointError, UnsupportedMathematicsError, VerificationFailure,
    map_domain_parse, map_evaluation_error, map_rational_expression, poly_data, rational_data,
    selected_scalar, selected_variable,
};

fn evaluation_error(error: EvaluationError) -> PyErr {
    match error {
        EvaluationError::RationalFunction(
            RationalFunctionError::ZeroDenominator
            | RationalFunctionError::ZeroDivisor
            | RationalFunctionError::EmptyDomain
            | RationalFunctionError::UndefinedAtPoint,
        ) => UndefinedAtPointError::new_err(error.to_string()),
        _ => map_evaluation_error(error),
    }
}

fn conversion_error(error: RationalExpressionError) -> PyErr {
    match error {
        RationalExpressionError::Evaluation(reason) => evaluation_error(reason),
        RationalExpressionError::ExponentLimitExceeded => {
            ResourceLimitError::new_err(error.to_string())
        }
        RationalExpressionError::RationalFunction(
            RationalFunctionError::ZeroDenominator
            | RationalFunctionError::ZeroDivisor
            | RationalFunctionError::EmptyDomain
            | RationalFunctionError::UndefinedAtPoint,
        ) => UndefinedAtPointError::new_err(error.to_string()),
        _ => map_rational_expression(error),
    }
}

fn limit_verification_error(error: LimitVerificationError) -> PyErr {
    match error {
        LimitVerificationError::Evaluation(reason) => evaluation_error(reason),
        LimitVerificationError::InvalidCertificate => {
            VerificationFailure::new_err(error.to_string())
        }
    }
}

fn limit_error(error: LimitError) -> PyErr {
    match error {
        LimitError::Evaluation(reason) => evaluation_error(reason),
        LimitError::Conversion(reason) => conversion_error(reason),
        LimitError::Verification(reason) => limit_verification_error(reason),
        LimitError::InconsistentFactorization => VerificationFailure::new_err(error.to_string()),
    }
}

fn inequality_error(error: InequalityError) -> PyErr {
    match error {
        InequalityError::Evaluation(reason) => evaluation_error(reason),
        InequalityError::Conversion(reason) => conversion_error(reason),
        InequalityError::UnsupportedDegree => {
            UnsupportedMathematicsError::new_err(error.to_string())
        }
        InequalityError::IncorrectCertificate => VerificationFailure::new_err(error.to_string()),
    }
}

type PyFactor = (usize, PyPolynomialData);
type PyLimit = (
    String,
    Option<String>,
    String,
    PyRationalFunctionData,
    Option<PyFactor>,
    PyFactor,
    Vec<String>,
    bool,
);

fn factor_data(factor: &LimitFactor, symbol: &Symbol) -> PyFactor {
    (factor.order(), poly_data(factor.cofactor(), symbol))
}

#[pyfunction(name = "limit", signature = (source, variable, at, direction="two-sided"))]
fn py_limit(source: &str, variable: &str, at: &str, direction: &str) -> PyResult<PyLimit> {
    let symbol = selected_variable(variable)?;
    let side = match direction {
        "left" => LimitDirection::Left,
        "right" => LimitDirection::Right,
        "two-sided" => LimitDirection::TwoSided,
        _ => {
            return Err(MathSyntaxError::new_err(
                "direction must be left, right or two-sided",
            ));
        }
    };
    if matches!(at, "infinity" | "-infinity" | "inf" | "-inf" | "∞" | "-∞") {
        return Err(UnsupportedMathematicsError::new_err(
            "limits at infinity are not supported",
        ));
    }
    let at = selected_scalar(at)?;
    let ctx = EvaluationContext::default();
    let parsed =
        parse_domain_expression_in_domain(source, ScalarDomain::Real).map_err(map_domain_parse)?;
    let function = ctx
        .rational_expression(&parsed, &symbol)
        .map_err(conversion_error)?;
    let result = ctx
        .rational_limit(&function, &at, side)
        .map_err(limit_error)?;
    verify_rational_limit(&ctx, &function, &at, side, &result).map_err(limit_verification_error)?;
    let (kind, value) = match result.value() {
        LimitValue::Finite(value) => ("finite", Some(value.to_string())),
        LimitValue::PositiveInfinity => ("positive_infinity", None),
        LimitValue::NegativeInfinity => ("negative_infinity", None),
        LimitValue::DoesNotExist => ("does_not_exist", None),
    };
    Ok((
        kind.to_owned(),
        value,
        at.to_string(),
        rational_data(result.source(), &symbol),
        result
            .numerator_factor()
            .map(|factor| factor_data(factor, &symbol)),
        factor_data(result.denominator_factor(), &symbol),
        result
            .exclusion_values()
            .iter()
            .map(ToString::to_string)
            .collect(),
        result.excluded_at_point(),
    ))
}

fn root_data(root: &ExactQuadraticRoot, symbol: &Symbol) -> PyAlgebraicValue {
    match root {
        ExactQuadraticRoot::Rational(value) => {
            ("rational".to_owned(), Some(value.to_string()), None, None)
        }
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch,
        } => (
            "real_quadratic".to_owned(),
            None,
            Some(poly_data(minimal_polynomial, symbol)),
            Some(branch.stable_id().to_owned()),
        ),
    }
}

type PyBound = (String, Option<PyAlgebraicValue>);
type PyInterval = (PyBound, PyBound, bool, bool);
type PyCriticalPoint = (PyAlgebraicValue, u8, u8, bool);
type PyInequality = (
    String,
    PyRationalFunctionData,
    Vec<PyInterval>,
    Vec<PyCriticalPoint>,
);

fn bound_data(bound: &RealBound, symbol: &Symbol) -> PyBound {
    match bound {
        RealBound::NegativeInfinity => ("negative_infinity".to_owned(), None),
        RealBound::PositiveInfinity => ("positive_infinity".to_owned(), None),
        RealBound::Finite(root) => ("finite".to_owned(), Some(root_data(root, symbol))),
    }
}

#[pyfunction(name = "solve_inequality", signature = (source, variable="x"))]
fn py_inequality(source: &str, variable: &str) -> PyResult<PyInequality> {
    let symbol = selected_variable(variable)?;
    let parsed = parse_real_comparison(source).map_err(map_domain_parse)?;
    let (comparison, operator) = match parsed.operator {
        RealComparisonOperator::Less => (ComparisonOperator::Less, "<"),
        RealComparisonOperator::LessOrEqual => (ComparisonOperator::LessOrEqual, "<="),
        RealComparisonOperator::Greater => (ComparisonOperator::Greater, ">"),
        RealComparisonOperator::GreaterOrEqual => (ComparisonOperator::GreaterOrEqual, ">="),
    };
    let ctx = EvaluationContext::default();
    let result = ctx
        .solve_expression_inequality(&parsed.left, &parsed.right, &symbol, comparison)
        .map_err(inequality_error)?;
    if !ctx
        .verify_expression_inequality(&parsed.left, &parsed.right, &symbol, comparison, &result)
        .map_err(inequality_error)?
    {
        return Err(VerificationFailure::new_err(
            "inequality certificate failed verification",
        ));
    }
    Ok((
        operator.to_owned(),
        rational_data(result.function(), &symbol),
        result
            .intervals()
            .iter()
            .map(|interval| {
                (
                    bound_data(&interval.lower, &symbol),
                    bound_data(&interval.upper, &symbol),
                    interval.lower_closed,
                    interval.upper_closed,
                )
            })
            .collect(),
        result
            .critical_points()
            .iter()
            .map(|point| {
                (
                    root_data(point.root(), &symbol),
                    point.numerator_multiplicity(),
                    point.denominator_multiplicity(),
                    point.is_excluded(),
                )
            })
            .collect(),
    ))
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(py_limit, module)?)?;
    module.add_function(wrap_pyfunction!(py_inequality, module)?)?;
    Ok(())
}
