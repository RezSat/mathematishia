//! Thin, typed Python bindings to the verified Rust mathematical kernel.
//!
//! Python performs representation and presentation only. All algebraic work
//! is delegated to the checked core, never reimplemented using Python floats.

use mathema::{Condition, DomainExpression, Equation, Expr, ExprView, Rational, Symbol};
use mathema_frontend::{
    DomainParseError, DomainStatement, Statement, parse_domain_expression, parse_domain_statement,
    parse_expression, parse_statement,
};
use mathematishia_kernel::{
    AlgebraicRootEvidence, AlgebraicRootSet, Assumptions, DefinednessError, DomainOperationError,
    DomainRewriteOutcome, EvaluationContext, EvaluationError, ExactQuadraticRoot, LinearSolution,
    ProofNodeKind, QuadraticBranch, QuadraticElement, QuadraticField, QuadraticFieldError, QuadraticRoots, RationalExpressionError, RationalFunction,
    RationalFunctionError, RationalPolynomial, RationalQuadraticRoots, ResourceLimitKind,
    VerifiedCondition, rewrite_domain_once, substitute_domain,
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
        EvaluationError::AlgebraicVerification(reason) => {
            VerificationFailure::new_err(reason.to_string())
        }
        EvaluationError::ExtendedGcdVerification(reason) => {
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

fn map_domain_parse(error: DomainParseError) -> PyErr {
    match error {
        DomainParseError::Syntax(error) => MathSyntaxError::new_err(error.to_string()),
        DomainParseError::Domain(
            DefinednessError::UnsupportedExponent | DefinednessError::ZeroDenominator,
        ) => UnsupportedMathematicsError::new_err(error.to_string()),
        DomainParseError::Domain(_) => ResourceLimitError::new_err(error.to_string()),
    }
}

fn map_domain_operation(error: DomainOperationError) -> PyErr {
    match error {
        DomainOperationError::Definedness(DefinednessError::UnsupportedExponent) => {
            UnsupportedMathematicsError::new_err(error.to_string())
        }
        DomainOperationError::Definedness(_) | DomainOperationError::ProofConstruction(_) => {
            ResourceLimitError::new_err(error.to_string())
        }
        DomainOperationError::ProofVerification(_) => {
            VerificationFailure::new_err(error.to_string())
        }
    }
}

fn checked_domain(source: &str) -> PyResult<DomainExpression> {
    let value = parse_domain_expression(source).map_err(map_domain_parse)?;
    let ctx = EvaluationContext::default();
    ctx.check_expression(value.expression())
        .map_err(map_evaluation_error)?;
    for condition in value.conditions() {
        ctx.check_expression(condition.expression())
            .map_err(map_evaluation_error)?;
    }
    Ok(value)
}

fn condition_text(condition: &Condition) -> String {
    match condition {
        Condition::NonZero(expr) => format!("{expr} != 0"),
        Condition::Positive(expr) => format!("{expr} > 0"),
        Condition::Real(expr) => format!("{expr} in R"),
        Condition::Integer(expr) => format!("{expr} in Z"),
        Condition::InDomain { expression, domain } => format!("{expression} in {domain:?}"),
    }
}

fn domain_conditions(value: &DomainExpression) -> Vec<String> {
    value.conditions().iter().map(condition_text).collect()
}

type PyDomainValue = (String, Vec<String>);
type PyDomainEquation = (PyDomainValue, PyDomainValue);
type PyDomainProofNode = (usize, String, String, Vec<usize>, Option<String>);
type PyDomainRewrite = (
    String,
    String,
    Vec<String>,
    Vec<String>,
    Option<String>,
    Vec<PyDomainProofNode>,
);

#[pyfunction(name = "domain_expression")]
fn py_domain_expression(source: &str) -> PyResult<(String, Vec<String>)> {
    let value = checked_domain(source)?;
    Ok((value.expression().to_string(), domain_conditions(&value)))
}

#[pyfunction(name = "domain_equation")]
fn py_domain_equation(source: &str) -> PyResult<PyDomainEquation> {
    let statement = parse_domain_statement(source).map_err(map_domain_parse)?;
    let DomainStatement::Equation { left, right } = statement else {
        return Err(MathSyntaxError::new_err(
            "expected an equation containing '='",
        ));
    };
    let ctx = EvaluationContext::default();
    for side in [&left, &right] {
        ctx.check_expression(side.expression())
            .map_err(map_evaluation_error)?;
        for condition in side.conditions() {
            ctx.check_expression(condition.expression())
                .map_err(map_evaluation_error)?;
        }
    }
    Ok((
        (left.expression().to_string(), domain_conditions(&left)),
        (right.expression().to_string(), domain_conditions(&right)),
    ))
}

#[pyfunction(name = "substitute_domain")]
fn py_substitute_domain(
    source: &str,
    variable: &str,
    replacement: &str,
) -> PyResult<(String, Vec<String>)> {
    let source = checked_domain(source)?;
    let symbol = selected_variable(variable)?;
    let replacement = checked_domain(replacement)?;
    let value =
        substitute_domain(&source, &symbol, &replacement, 2_048, 256).map_err(
            |error| match error {
                DefinednessError::UnsupportedExponent => {
                    UnsupportedMathematicsError::new_err(error.to_string())
                }
                _ => ResourceLimitError::new_err(error.to_string()),
            },
        )?;
    Ok((value.expression().to_string(), domain_conditions(&value)))
}

#[pyfunction(name = "rewrite_domain", signature = (source, nonzero=None))]
fn py_rewrite_domain(source: &str, nonzero: Option<Vec<String>>) -> PyResult<PyDomainRewrite> {
    let source = checked_domain(source)?;
    let mut assumptions = Assumptions::new();
    let premises = nonzero.unwrap_or_default();
    if premises.len() > 32 {
        return Err(ResourceLimitError::new_err(
            "at most 32 nonzero premises are supported",
        ));
    }
    for premise in premises {
        let value = checked_domain(&premise)?;
        if !value.conditions().is_empty() {
            return Err(UnsupportedMathematicsError::new_err(
                "premise expressions must not contain unresolved domain restrictions",
            ));
        }
        assumptions
            .add(Condition::NonZero(value.expression().clone()))
            .map_err(|error| UnsupportedMathematicsError::new_err(error.to_string()))?;
    }

    let outcome = rewrite_domain_once(&source, &assumptions).map_err(map_domain_operation)?;
    match outcome {
        DomainRewriteOutcome::Applied(rewrite) => {
            let mut nodes = Vec::new();
            for node in rewrite.proof().nodes() {
                let (kind, value) = match node.kind() {
                    ProofNodeKind::Input(expr) => ("input", expr.to_string()),
                    ProofNodeKind::Condition { condition, status } => (
                        "condition",
                        format!("{} [{status:?}]", condition_text(condition)),
                    ),
                    ProofNodeKind::Distribution { step, .. } => {
                        ("distribution", step.after().to_string())
                    }
                    ProofNodeKind::GuardedCancellation { step, .. } => {
                        ("guarded_cancellation", step.after().to_string())
                    }
                    ProofNodeKind::CompositeCancellation { step, .. } => {
                        ("composite_cancellation", step.after().to_string())
                    }
                };
                nodes.push((
                    node.id().index(),
                    kind.to_owned(),
                    value,
                    node.kind()
                        .dependencies()
                        .iter()
                        .map(|id| id.index())
                        .collect(),
                    node.kind().rule_id().map(str::to_owned),
                ));
            }
            Ok((
                "applied".to_owned(),
                rewrite.result().expression().to_string(),
                domain_conditions(rewrite.result()),
                Vec::new(),
                rewrite.rule_id().map(str::to_owned),
                nodes,
            ))
        }
        DomainRewriteOutcome::Blocked(required) => Ok((
            "blocked".to_owned(),
            source.expression().to_string(),
            domain_conditions(&source),
            required.iter().map(condition_text).collect(),
            None,
            Vec::new(),
        )),
        DomainRewriteOutcome::Refuted(condition) => Ok((
            "refuted".to_owned(),
            source.expression().to_string(),
            domain_conditions(&source),
            vec![condition_text(&condition)],
            None,
            Vec::new(),
        )),
        DomainRewriteOutcome::NotApplicable => Ok((
            "unchanged".to_owned(),
            source.expression().to_string(),
            domain_conditions(&source),
            Vec::new(),
            None,
            Vec::new(),
        )),
    }
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

type PyEuclideanStep = (
    PyPolynomialData,
    PyPolynomialData,
    PyPolynomialData,
    PyPolynomialData,
);
type PyExtendedGcd = (
    PyPolynomialData,
    PyPolynomialData,
    PyPolynomialData,
    Vec<PyEuclideanStep>,
);

#[pyfunction(name = "extended_gcd")]
fn py_extended_gcd(left: &str, right: &str, variable: &str) -> PyResult<PyExtendedGcd> {
    let symbol = selected_variable(variable)?;
    let first = selected_polynomial(left, &symbol)?;
    let second = selected_polynomial(right, &symbol)?;
    let result = EvaluationContext::default()
        .extended_gcd(&first, &second)
        .map_err(map_evaluation_error)?;
    let steps = result
        .divisions()
        .iter()
        .map(|step| {
            (
                poly_data(step.dividend(), &symbol),
                poly_data(step.divisor(), &symbol),
                poly_data(step.quotient(), &symbol),
                poly_data(step.remainder(), &symbol),
            )
        })
        .collect();
    Ok((
        poly_data(result.gcd(), &symbol),
        poly_data(result.left_coefficient(), &symbol),
        poly_data(result.right_coefficient(), &symbol),
        steps,
    ))
}

#[pyfunction(name = "modular_inverse")]
fn py_modular_inverse(
    value: &str,
    modulus: &str,
    variable: &str,
) -> PyResult<Option<PyPolynomialData>> {
    let symbol = selected_variable(variable)?;
    let left = selected_polynomial(value, &symbol)?;
    let modulus = selected_polynomial(modulus, &symbol)?;
    EvaluationContext::default()
        .inverse_mod_polynomial(&left, &modulus)
        .map(|value| value.map(|polynomial| poly_data(&polynomial, &symbol)))
        .map_err(map_evaluation_error)
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

fn map_rational_expression(error: RationalExpressionError) -> PyErr {
    match error {
        RationalExpressionError::Evaluation(reason) => map_evaluation_error(reason),
        RationalExpressionError::RationalFunction(_)
        | RationalExpressionError::OtherSymbol
        | RationalExpressionError::UnsupportedExponent
        | RationalExpressionError::ExponentLimitExceeded
        | RationalExpressionError::UnsupportedCondition => {
            UnsupportedMathematicsError::new_err(error.to_string())
        }
    }
}

fn selected_rational_expression(source: &str, variable: &Symbol) -> PyResult<RationalFunction> {
    let parsed = checked_domain(source)?;
    EvaluationContext::default()
        .rational_expression(&parsed, variable)
        .map_err(map_rational_expression)
}

#[pyfunction(name = "rational_simplify")]
fn py_rational_simplify(source: &str, variable: &str) -> PyResult<PyRationalFunctionData> {
    let symbol = selected_variable(variable)?;
    let value = selected_rational_expression(source, &symbol)?;
    Ok(rational_data(&value, &symbol))
}

#[pyfunction(name = "rational_derivative")]
fn py_rational_derivative(source: &str, variable: &str) -> PyResult<PyRationalFunctionData> {
    let symbol = selected_variable(variable)?;
    let value = selected_rational_expression(source, &symbol)?;
    let result = EvaluationContext::default()
        .differentiate_rational_function(&value)
        .map_err(map_evaluation_error)?;
    Ok(rational_data(&result, &symbol))
}

#[pyfunction(name = "rational_value")]
fn py_rational_value(source: &str, variable: &str, at: &str) -> PyResult<String> {
    let symbol = selected_variable(variable)?;
    let value = selected_rational_expression(source, &symbol)?;
    let point = selected_scalar(at)?;
    EvaluationContext::default()
        .evaluate_rational_function(&value, &point)
        .map(|result| result.to_string())
        .map_err(map_evaluation_error)
}

type PyRationalRoots = (String, Vec<String>, Vec<PyPolynomialData>);

#[pyfunction(name = "rational_solve")]
fn py_rational_solve(source: &str, variable: &str) -> PyResult<PyRationalRoots> {
    let symbol = selected_variable(variable)?;
    let parsed = parse_domain_statement(source).map_err(map_domain_parse)?;
    let DomainStatement::Equation { left, right } = parsed else {
        return Err(MathSyntaxError::new_err(
            "expected an equation containing '='",
        ));
    };
    let ctx = EvaluationContext::default();
    let lhs = ctx
        .rational_expression(&left, &symbol)
        .map_err(map_rational_expression)?;
    let rhs = ctx
        .rational_expression(&right, &symbol)
        .map_err(map_rational_expression)?;
    let result = ctx
        .solve_rational_quadratic_equation(&lhs, &rhs)
        .map_err(map_evaluation_error)?;
    let (status, roots) = match result.roots() {
        RationalQuadraticRoots::AllAdmissibleRationals => ("all_admissible".to_owned(), Vec::new()),
        RationalQuadraticRoots::Finite(values) => (
            "finite".to_owned(),
            values.iter().map(ToString::to_string).collect(),
        ),
    };
    let exclusions = result
        .domain_conditions()
        .iter()
        .map(|polynomial| poly_data(polynomial, &symbol))
        .collect();
    Ok((status, roots, exclusions))
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

#[pyfunction(name = "rational_solve_quadratic")]
fn py_rational_solve_quadratic(
    left_numerator: &str,
    left_denominator: &str,
    right_numerator: &str,
    right_denominator: &str,
    variable: &str,
) -> PyResult<(String, Vec<String>, Vec<PyPolynomialData>)> {
    let symbol = selected_variable(variable)?;
    let lhs = selected_rational(left_numerator, left_denominator, &symbol)?;
    let rhs = selected_rational(right_numerator, right_denominator, &symbol)?;
    let result = EvaluationContext::default()
        .solve_rational_quadratic_equation(&lhs, &rhs)
        .map_err(map_evaluation_error)?;
    let (kind, roots) = match result.roots() {
        RationalQuadraticRoots::AllAdmissibleRationals => ("all_admissible".to_owned(), Vec::new()),
        RationalQuadraticRoots::Finite(roots) => (
            "finite".to_owned(),
            roots.iter().map(ToString::to_string).collect(),
        ),
    };
    let excluded = result
        .domain_conditions()
        .iter()
        .map(|polynomial| poly_data(polynomial, &symbol))
        .collect();
    Ok((kind, roots, excluded))
}

type PyAlgebraicValue = (
    String,
    Option<String>,
    Option<PyPolynomialData>,
    Option<String>,
);
type PyAlgebraicStep = (String, Option<PyPolynomialData>, Option<String>);
type PyAlgebraicResult = (String, Vec<PyAlgebraicValue>, Vec<PyAlgebraicStep>);

fn map_quadratic_field_error(error: QuadraticFieldError) -> PyErr {
    match error {
        QuadraticFieldError::Evaluation(reason) => map_evaluation_error(reason),
        _ => UnsupportedMathematicsError::new_err(error.to_string()),
    }
}

fn selected_quadratic_branch(branch: &str) -> PyResult<QuadraticBranch> {
    match branch {
        "lower_real" => Ok(QuadraticBranch::LowerReal),
        "upper_real" => Ok(QuadraticBranch::UpperReal),
        "negative_imaginary" => Ok(QuadraticBranch::NegativeImaginary),
        "positive_imaginary" => Ok(QuadraticBranch::PositiveImaginary),
        _ => Err(UnsupportedMathematicsError::new_err("invalid quadratic root branch")),
    }
}

fn selected_quadratic_element(
    polynomial: &str,
    variable: &str,
    branch: &str,
    constant: &str,
    coefficient: &str,
) -> PyResult<(QuadraticElement, Symbol)> {
    let ctx = EvaluationContext::default();
    let symbol = selected_variable(variable)?;
    let minimal = selected_polynomial(polynomial, &symbol)?;
    let field = QuadraticField::new(&ctx, minimal, selected_quadratic_branch(branch)?)
        .map_err(map_quadratic_field_error)?;
    let value = QuadraticElement::new(
        &ctx,
        field,
        selected_scalar(constant)?,
        selected_scalar(coefficient)?,
    )
    .map_err(map_quadratic_field_error)?;
    Ok((value, symbol))
}

type PyQuadraticValue = (
    PyPolynomialData,
    String,
    (String, String),
    PyPolynomialData,
    Option<String>,
    (String, String),
);

fn quadratic_value_data(
    value: &QuadraticElement,
    symbol: &Symbol,
) -> PyResult<PyQuadraticValue> {
    let ctx = EvaluationContext::default();
    let minimal = value.minimal_polynomial(&ctx).map_err(map_quadratic_field_error)?;
    let selected = value.as_exact_root(&ctx).map_err(map_quadratic_field_error)?;
    let trace = value.trace(&ctx).map_err(map_quadratic_field_error)?;
    let norm = value.norm(&ctx).map_err(map_quadratic_field_error)?;
    Ok((
        poly_data(value.field().polynomial(), symbol),
        value.field().branch().stable_id().to_owned(),
        (value.constant().to_string(), value.coefficient().to_string()),
        poly_data(&minimal, symbol),
        selected.branch().map(|branch| branch.stable_id().to_owned()),
        (trace.to_string(), norm.to_string()),
    ))
}

#[pyfunction(name = "quadratic_value")]
fn py_quadratic_value(
    polynomial: &str,
    variable: &str,
    branch: &str,
    constant: &str,
    coefficient: &str,
) -> PyResult<PyQuadraticValue> {
    let (value, symbol) =
        selected_quadratic_element(polynomial, variable, branch, constant, coefficient)?;
    quadratic_value_data(&value, &symbol)
}

#[pyfunction(name = "quadratic_operation")]
fn py_quadratic_operation(
    operation: &str,
    polynomial: &str,
    variable: &str,
    branch: &str,
    a: &str,
    b: &str,
    c: &str,
    d: &str,
) -> PyResult<PyQuadraticValue> {
    let (left, symbol) = selected_quadratic_element(polynomial, variable, branch, a, b)?;
    let (right, _) = selected_quadratic_element(polynomial, variable, branch, c, d)?;
    let ctx = EvaluationContext::default();
    let result = match operation {
        "add" => left.add(&right, &ctx),
        "sub" => left.subtract(&right, &ctx),
        "mul" => left.multiply(&right, &ctx),
        "div" => left.divide(&right, &ctx),
        _ => return Err(UnsupportedMathematicsError::new_err("expected add, sub, mul or div")),
    }
    .map_err(map_quadratic_field_error)?;
    quadratic_value_data(&result, &symbol)
}

#[pyfunction(name = "quadratic_inverse")]
fn py_quadratic_inverse(
    polynomial: &str,
    variable: &str,
    branch: &str,
    constant: &str,
    coefficient: &str,
) -> PyResult<PyQuadraticValue> {
    let (value, symbol) =
        selected_quadratic_element(polynomial, variable, branch, constant, coefficient)?;
    let inverse = value.inverse(&EvaluationContext::default())
        .map_err(map_quadratic_field_error)?;
    quadratic_value_data(&inverse, &symbol)
}

#[pyfunction(name = "algebraic_roots")]
fn py_algebraic_roots(source: &str, variable: &str) -> PyResult<PyAlgebraicResult> {
    let selected = selected_variable(variable)?;
    let polynomial = selected_polynomial(source, &selected)?;
    let result = EvaluationContext::default()
        .algebraic_roots(&polynomial)
        .map_err(map_evaluation_error)?;
    let (kind, roots) = match result.roots() {
        AlgebraicRootSet::AllComplexNumbers => ("all_complex".to_owned(), Vec::new()),
        AlgebraicRootSet::Finite(values) => (
            "finite".to_owned(),
            values
                .iter()
                .map(|value| match value {
                    ExactQuadraticRoot::Rational(rational) => (
                        "rational".to_owned(),
                        Some(rational.to_string()),
                        None,
                        None,
                    ),
                    ExactQuadraticRoot::Algebraic {
                        minimal_polynomial,
                        branch,
                    } => (
                        if matches!(
                            branch,
                            QuadraticBranch::LowerReal | QuadraticBranch::UpperReal
                        ) {
                            "real_quadratic"
                        } else {
                            "complex_quadratic"
                        }
                        .to_owned(),
                        None,
                        Some(poly_data(minimal_polynomial, &selected)),
                        Some(branch.stable_id().to_owned()),
                    ),
                })
                .collect(),
        ),
    };
    let evidence = result
        .evidence()
        .iter()
        .map(|item| match item {
            AlgebraicRootEvidence::MonicNormalization(monic) => (
                item.rule().stable_id().to_owned(),
                Some(poly_data(monic, &selected)),
                None,
            ),
            AlgebraicRootEvidence::Discriminant(value) => (
                item.rule().stable_id().to_owned(),
                None,
                Some(value.to_string()),
            ),
            AlgebraicRootEvidence::Classification(_) => {
                (item.rule().stable_id().to_owned(), None, None)
            }
        })
        .collect();
    Ok((kind, roots, evidence))
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_expression, m)?)?;
    m.add_function(wrap_pyfunction!(py_domain_expression, m)?)?;
    m.add_function(wrap_pyfunction!(py_domain_equation, m)?)?;
    m.add_function(wrap_pyfunction!(py_substitute_domain, m)?)?;
    m.add_function(wrap_pyfunction!(py_rewrite_domain, m)?)?;
    m.add_function(wrap_pyfunction!(py_expand, m)?)?;
    m.add_function(wrap_pyfunction!(py_solve, m)?)?;
    m.add_function(wrap_pyfunction!(py_polynomial, m)?)?;
    m.add_function(wrap_pyfunction!(py_evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(py_divide, m)?)?;
    m.add_function(wrap_pyfunction!(py_gcd, m)?)?;
    m.add_function(wrap_pyfunction!(py_extended_gcd, m)?)?;
    m.add_function(wrap_pyfunction!(py_modular_inverse, m)?)?;
    m.add_function(wrap_pyfunction!(py_differentiate, m)?)?;
    m.add_function(wrap_pyfunction!(py_square_free, m)?)?;
    m.add_function(wrap_pyfunction!(py_differentiate_expression, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_function, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_simplify, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_derivative, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_value, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_solve, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_evaluate, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_differentiate, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_operation, m)?)?;
    m.add_function(wrap_pyfunction!(py_solve_quadratic, m)?)?;
    m.add_function(wrap_pyfunction!(py_algebraic_roots, m)?)?;
    m.add_function(wrap_pyfunction!(py_quadratic_value, m)?)?;
    m.add_function(wrap_pyfunction!(py_quadratic_operation, m)?)?;
    m.add_function(wrap_pyfunction!(py_quadratic_inverse, m)?)?;
    m.add_function(wrap_pyfunction!(py_rational_solve_quadratic, m)?)?;
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
