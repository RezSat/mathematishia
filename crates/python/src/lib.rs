//! Thin, typed Python bindings to the verified Rust mathematical kernel.
//!
//! Python performs representation and presentation only. All algebraic work
//! is delegated to the checked core, never reimplemented using Python floats.

use mathema::{Equation, Expr, ExprView, Rational, Symbol};
use mathema_frontend::{Statement, parse_expression, parse_statement};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, LinearSolution, QuadraticRoots, RationalFunction,
    RationalFunctionError, RationalPolynomial, ResourceLimitKind, VerifiedCondition,
};
use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

create_exception!(_native, MathSyntaxError, PyValueError);
create_exception!(_native, UnsupportedMathematicsError, PyValueError);
create_exception!(_native, ResourceLimitError, PyRuntimeError);
create_exception!(_native, VerificationFailure, PyRuntimeError);
create_exception!(_native, UndefinedAtPointError, PyValueError);

fn map_evaluation_error(error: EvaluationError) -> PyErr {
    match error {
        EvaluationError::Limit(kind) => {
            ResourceLimitError::new_err(format!("{kind:?} limit exceeded"))
        }
        EvaluationError::Verification(reason) => VerificationFailure::new_err(reason.to_string()),
        EvaluationError::SquareFreeVerification(reason) => {
            VerificationFailure::new_err(reason.to_string())
        }
        EvaluationError::DerivativeVerification(reason) => {
            VerificationFailure::new_err(reason.to_string())
        }
        EvaluationError::QuadraticVerification(reason) => {
            VerificationFailure::new_err(reason.to_string())
        }
        EvaluationError::RationalEquationVerification(reason) => {
            VerificationFailure::new_err(reason.to_string())
        }
        EvaluationError::RationalFunction(RationalFunctionError::UndefinedAtPoint) => {
            UndefinedAtPointError::new_err(error.to_string())
        }
        EvaluationError::Polynomial(_)
        | EvaluationError::Linear(_)
        | EvaluationError::SquareFree(_)
        | EvaluationError::SymbolicDifferentiation(_)
        | EvaluationError::RationalFunction(_)
        | EvaluationError::Quadratic(_)
        | EvaluationError::RationalEquation(_)
        | EvaluationError::DivisionByZero => {
            UnsupportedMathematicsError::new_err(error.to_string())
        }
    }
}

fn checked_expression(source: &str) -> PyResult<Expr> {
    let expr =
        parse_expression(source).map_err(|error| MathSyntaxError::new_err(error.to_string()))?;
    EvaluationContext::default()
        .check_expression(&expr)
        .map_err(map_evaluation_error)?;
    Ok(expr)
}

fn selected_variable(name: &str) -> PyResult<Symbol> {
    let parsed = checked_expression(name)?;
    if let ExprView::Symbol(symbol) = parsed.view() {
        Ok(symbol.clone())
    } else {
        Err(MathSyntaxError::new_err("variable must be one identifier"))
    }
}

fn selected_equation(source: &str) -> PyResult<Equation> {
    match parse_statement(source).map_err(|error| MathSyntaxError::new_err(error.to_string()))? {
        Statement::Equation(eq) => Ok(eq),
        Statement::Expression(_) => Err(MathSyntaxError::new_err(
            "expected an equation containing '='",
        )),
    }
}

fn selected_scalar(source: &str) -> PyResult<Rational> {
    let parsed = checked_expression(source)?;
    match parsed.view() {
        ExprView::Integer(value) => Ok(Rational::from(value.clone())),
        ExprView::Rational(value) => Ok(value.clone()),
        _ => Err(UnsupportedMathematicsError::new_err(
            "expected an exact rational literal or constant",
        )),
    }
}

fn selected_polynomial(source: &str, symbol: &Symbol) -> PyResult<RationalPolynomial> {
    EvaluationContext::default()
        .polynomial(&checked_expression(source)?, symbol)
        .map_err(map_evaluation_error)
}

fn poly_data(poly: &RationalPolynomial, symbol: &Symbol) -> (String, Vec<String>) {
    (
        poly.to_expr(symbol).to_string(),
        poly.coefficients()
            .iter()
            .map(ToString::to_string)
            .collect(),
    )
}

fn show_equation(eq: &Equation) -> String {
    format!("{} = {}", eq.left(), eq.right())
}

#[pyfunction(name = "expression")]
fn py_expression(source: &str) -> PyResult<String> {
    Ok(checked_expression(source)?.to_string())
}

#[pyfunction(name = "expand", signature = (source, max_steps=8))]
fn py_expand(source: &str, max_steps: usize) -> PyResult<PyExpansion> {
    let default = EvaluationContext::default();
    if max_steps > default.limits().max_distribution_steps {
        return Err(map_evaluation_error(EvaluationError::Limit(
            ResourceLimitKind::DistributionSteps,
        )));
    }
    let expr = checked_expression(source)?;
    let result = default
        .with_distribution_steps(max_steps)
        .expand(&expr)
        .map_err(map_evaluation_error)?;
    let steps = result
        .steps()
        .iter()
        .map(|step| {
            (
                step.before().to_string(),
                step.after().to_string(),
                step.rule().stable_id().to_owned(),
            )
        })
        .collect();
    Ok((result.result().to_string(), result.is_complete(), steps))
}

type PyExpansionStep = (String, String, String);
type PyExpansion = (String, bool, Vec<PyExpansionStep>);
type PyPolynomialData = (String, Vec<String>);
type PyDivision = (PyPolynomialData, PyPolynomialData);

type PyEquationStep = (String, String, String, Option<String>);

#[pyfunction(name = "solve")]
fn py_solve(
    source: &str,
    variable: &str,
) -> PyResult<(String, Option<String>, Vec<PyEquationStep>)> {
    let eq = selected_equation(source)?;
    let symbol = selected_variable(variable)?;
    let result = EvaluationContext::default()
        .solve_linear(&eq, &symbol)
        .map_err(map_evaluation_error)?;
    let (status, value) = match result.solution() {
        LinearSolution::Unique(root) => ("unique".to_owned(), Some(root.to_string())),
        LinearSolution::NoSolution => ("none".to_owned(), None),
        LinearSolution::AllRationals => ("all".to_owned(), None),
    };
    let steps = result
        .steps()
        .iter()
        .map(|step| {
            let condition = step.verified_condition().map(|condition| match condition {
                VerifiedCondition::Nonzero(value) => format!("{value} != 0"),
            });
            (
                show_equation(step.before()),
                show_equation(step.after()),
                step.rule().stable_id().to_owned(),
                condition,
            )
        })
        .collect();
    Ok((status, value, steps))
}

#[pyfunction(name = "polynomial")]
fn py_polynomial(source: &str, variable: &str) -> PyResult<(String, Vec<String>)> {
    let symbol = selected_variable(variable)?;
    let poly = selected_polynomial(source, &symbol)?;
    Ok(poly_data(&poly, &symbol))
}

#[pyfunction(name = "evaluate")]
fn py_evaluate(source: &str, variable: &str, at: &str) -> PyResult<String> {
    let symbol = selected_variable(variable)?;
    let poly = selected_polynomial(source, &symbol)?;
    let input = selected_scalar(at)?;
    let result = EvaluationContext::default()
        .evaluate_polynomial(&poly, &input)
        .map_err(map_evaluation_error)?;
    Ok(result.to_string())
}

#[pyfunction(name = "divide")]
fn py_divide(dividend: &str, divisor: &str, variable: &str) -> PyResult<PyDivision> {
    let symbol = selected_variable(variable)?;
    let first = selected_polynomial(dividend, &symbol)?;
    let second = selected_polynomial(divisor, &symbol)?;
    let (quotient, remainder) = EvaluationContext::default()
        .divide(&first, &second)
        .map_err(map_evaluation_error)?;
    Ok((
        poly_data(&quotient, &symbol),
        poly_data(&remainder, &symbol),
    ))
}

#[pyfunction(name = "gcd")]
fn py_gcd(left: &str, right: &str, variable: &str) -> PyResult<(String, Vec<String>)> {
    let symbol = selected_variable(variable)?;
    let first = selected_polynomial(left, &symbol)?;
    let second = selected_polynomial(right, &symbol)?;
    let result = EvaluationContext::default()
        .gcd(&first, &second)
        .map_err(map_evaluation_error)?;
    Ok(poly_data(&result, &symbol))
}

#[pyfunction(name = "differentiate")]
fn py_differentiate(source: &str, variable: &str) -> PyResult<PyPolynomialData> {
    let symbol = selected_variable(variable)?;
    let polynomial = selected_polynomial(source, &symbol)?;
    let derivative = EvaluationContext::default()
        .differentiate(&polynomial)
        .map_err(map_evaluation_error)?;
    Ok(poly_data(&derivative, &symbol))
}

type PySquareFreeFactor = (PyPolynomialData, usize);
type PySquareFreeResult = (String, Vec<PySquareFreeFactor>);

#[pyfunction(name = "square_free")]
fn py_square_free(source: &str, variable: &str) -> PyResult<PySquareFreeResult> {
    let symbol = selected_variable(variable)?;
    let polynomial = selected_polynomial(source, &symbol)?;
    let result = EvaluationContext::default()
        .square_free_decomposition(&polynomial)
        .map_err(map_evaluation_error)?;
    let factors = result
        .factors()
        .iter()
        .map(|entry| (poly_data(entry.polynomial(), &symbol), entry.multiplicity()))
        .collect();
    Ok((result.unit().to_string(), factors))
}

type PySymbolicStep = (String, String, String, String);
type PySymbolicDerivative = (String, Vec<PySymbolicStep>);

#[pyfunction(name = "differentiate_expression")]
fn py_differentiate_expression(source: &str, variable: &str) -> PyResult<PySymbolicDerivative> {
    let selected = selected_variable(variable)?;
    let original = checked_expression(source)?;
    let result = EvaluationContext::default()
        .differentiate_expression(&original, &selected)
        .map_err(map_evaluation_error)?;
    let steps = result
        .steps()
        .iter()
        .map(|step| {
            (
                step.before().to_string(),
                step.derivative().to_string(),
                step.rule().stable_id().to_owned(),
                "derivative_of".to_owned(),
            )
        })
        .collect();
    Ok((result.derivative().to_string(), steps))
}

type PyRationalFunctionData = (PyPolynomialData, PyPolynomialData, Vec<PyPolynomialData>);

fn rational_data(value: &RationalFunction, variable: &Symbol) -> PyRationalFunctionData {
    (
        poly_data(value.numerator(), variable),
        poly_data(value.denominator(), variable),
        value
            .excluded_zero_sets()
            .iter()
            .map(|condition| poly_data(condition, variable))
            .collect(),
    )
}

fn selected_rational(
    numerator: &str,
    denominator: &str,
    variable: &Symbol,
) -> PyResult<RationalFunction> {
    let n = selected_polynomial(numerator, variable)?;
    let d = selected_polynomial(denominator, variable)?;
    EvaluationContext::default()
        .rational_function(&n, &d)
        .map_err(map_evaluation_error)
}

#[pyfunction(name = "rational_function")]
fn py_rational_function(
    numerator: &str,
    denominator: &str,
    variable: &str,
) -> PyResult<PyRationalFunctionData> {
    let symbol = selected_variable(variable)?;
    Ok(rational_data(
        &selected_rational(numerator, denominator, &symbol)?,
        &symbol,
    ))
}

#[pyfunction(name = "rational_evaluate")]
fn py_rational_evaluate(
    numerator: &str,
    denominator: &str,
    variable: &str,
    at: &str,
) -> PyResult<String> {
    let symbol = selected_variable(variable)?;
    let function = selected_rational(numerator, denominator, &symbol)?;
    let point = selected_scalar(at)?;
    EvaluationContext::default()
        .evaluate_rational_function(&function, &point)
        .map(|value| value.to_string())
        .map_err(map_evaluation_error)
}

#[pyfunction(name = "rational_differentiate")]
fn py_rational_differentiate(
    numerator: &str,
    denominator: &str,
    variable: &str,
) -> PyResult<PyRationalFunctionData> {
    let symbol = selected_variable(variable)?;
    let function = selected_rational(numerator, denominator, &symbol)?;
    let derivative = EvaluationContext::default()
        .differentiate_rational_function(&function)
        .map_err(map_evaluation_error)?;
    Ok(rational_data(&derivative, &symbol))
}

#[pyfunction(name = "rational_operation")]
fn py_rational_operation(
    operation: &str,
    left_numerator: &str,
    left_denominator: &str,
    right_numerator: &str,
    right_denominator: &str,
    variable: &str,
) -> PyResult<PyRationalFunctionData> {
    let symbol = selected_variable(variable)?;
    let left = selected_rational(left_numerator, left_denominator, &symbol)?;
    let right = selected_rational(right_numerator, right_denominator, &symbol)?;
    let context = EvaluationContext::default();
    let result = match operation {
        "add" => context.add_rational_functions(&left, &right),
        "sub" => context.subtract_rational_functions(&left, &right),
        "mul" => context.multiply_rational_functions(&left, &right),
        "div" => context.divide_rational_functions(&left, &right),
        _ => {
            return Err(UnsupportedMathematicsError::new_err(
                "expected add, sub, mul or div",
            ));
        }
    }
    .map_err(map_evaluation_error)?;
    Ok(rational_data(&result, &symbol))
}

#[pyfunction(name = "solve_quadratic")]
fn py_solve_quadratic(source: &str, variable: &str) -> PyResult<(String, Vec<String>)> {
    let symbol = selected_variable(variable)?;
    let polynomial = selected_polynomial(source, &symbol)?;
    let result = EvaluationContext::default()
        .solve_quadratic(&polynomial)
        .map_err(map_evaluation_error)?;
    let (kind, roots) = match result.roots() {
        QuadraticRoots::AllRationals => ("all_rationals".to_owned(), Vec::new()),
        QuadraticRoots::Finite(roots) => (
            "finite".to_owned(),
            roots.iter().map(ToString::to_string).collect(),
        ),
    };
    Ok((kind, roots))
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_expression, m)?)?;
    m.add_function(wrap_pyfunction!(py_expand, m)?)?;
    m.add_function(wrap_pyfunction!(py_solve, m)?)?;
    m.add_function(wrap_pyfunction!(py_polynomial, m)?)?;
    m.add_function(wrap_pyfunction!(py_evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(py_divide, m)?)?;
    m.add_function(wrap_pyfunction!(py_gcd, m)?)?;
    m.add_function(wrap_pyfunction!(py_differentiate, m)?)?;
    m.add_function(wrap_pyfunction!(py_square_free, m)?)?;
    m.add_function(wrap_pyfunction!(py_differentiate_expression, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_function, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_differentiate, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_operation, m)?)?;
    m.add_function(wrap_pyfunction!(py_solve_quadratic, m)?)?;
    m.add("MathSyntaxError", m.py().get_type::<MathSyntaxError>())?;
    m.add(
        "UndefinedAtPointError",
        m.py().get_type::<UndefinedAtPointError>(),
    )?;
    m.add(
        "UnsupportedMathematicsError",
        m.py().get_type::<UnsupportedMathematicsError>(),
    )?;
    m.add(
        "ResourceLimitError",
        m.py().get_type::<ResourceLimitError>(),
    )?;
    m.add(
        "VerificationFailure",
        m.py().get_type::<VerificationFailure>(),
    )?;
    Ok(())
}
