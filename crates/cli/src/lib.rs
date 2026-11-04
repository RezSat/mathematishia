//! Command-line entry points for the existing exact Mathematishia kernel.
//!
//! The CLI owns input/output formatting only; mathematical behavior lives in
//! mathema-frontend and mathematishia-kernel.

use std::fmt;
use std::fmt::Write as _;

use mathema::{Equation, Expr, ExprView, Rational, Symbol};
use mathema_frontend::{Statement, parse_expression, parse_statement};
use mathematishia_kernel::{
    EvaluationContext, LinearSolution, RationalPolynomial, VerifiedCondition,
};

pub const HELP: &str = "Mathematishia (early exact-math CLI)
Usage:
  mathematishia expr <EXPRESSION>
  mathematishia expand <EXPRESSION> [MAX_STEPS]
  mathematishia solve <EQUATION> <VARIABLE>
  mathematishia poly <EXPRESSION> <VARIABLE>
  mathematishia eval <POLYNOMIAL> <VARIABLE> <RATIONAL>
  mathematishia divide <DIVIDEND> <DIVISOR> <VARIABLE>
  mathematishia gcd <LEFT> <RIGHT> <VARIABLE>
  mathematishia diff <POLYNOMIAL> <VARIABLE>
  mathematishia sqfree <POLYNOMIAL> <VARIABLE>
  mathematishia --help

Examples:
  mathematishia expr '(x+1)^2'
  mathematishia expand '2*(x+3)' 4
  mathematishia solve '2*x+4=10' x
  mathematishia poly '(x+1)^2' x
  mathematishia eval 'x^2+1/2' x '1/2'
  mathematishia divide 'x^2-1' 'x-1' x
  mathematishia gcd 'x^2-1' 'x-1' x
  mathematishia diff 'x^3+1/2*x' x
  mathematishia sqfree '2*(x-1)^2*(x+2)^3' x

Syntax: integers, exact fraction literals n/d, identifiers, + - * ^ ( ) =.
Use explicit multiplication. '/' only separates literal integers, not expressions.
The parser and solver are deliberately bounded; this is not a general CAS.";

const MAX_DISTRIBUTION_STEPS: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliError(String);

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CliError {}

fn invalid(message: impl Into<String>) -> CliError {
    CliError(message.into())
}

fn expression(input: &str) -> Result<Expr, CliError> {
    let parsed = parse_expression(input).map_err(|error| invalid(error.to_string()))?;
    EvaluationContext::default()
        .check_expression(&parsed)
        .map_err(|error| invalid(error.to_string()))?;
    Ok(parsed)
}

fn variable(input: &str) -> Result<Symbol, CliError> {
    let parsed = expression(input)?;
    match parsed.view() {
        ExprView::Symbol(symbol) => Ok(symbol.clone()),
        _ => Err(invalid("expected a single variable name")),
    }
}

fn equation(input: &str) -> Result<Equation, CliError> {
    match parse_statement(input).map_err(|error| invalid(error.to_string()))? {
        Statement::Equation(equation) => Ok(equation),
        Statement::Expression(_) => Err(invalid("expected an equation containing '='")),
    }
}

fn polynomial(input: &str, name: &Symbol) -> Result<RationalPolynomial, CliError> {
    let parsed = expression(input)?;
    EvaluationContext::default()
        .polynomial(&parsed, name)
        .map_err(|error| invalid(error.to_string()))
}

fn scalar(input: &str) -> Result<Rational, CliError> {
    let parsed = expression(input)?;
    match parsed.view() {
        ExprView::Integer(value) => Ok(Rational::from(value.clone())),
        ExprView::Rational(value) => Ok(value.clone()),
        _ => Err(invalid("expected an exact rational number")),
    }
}

fn show_equation(equation: &Equation) -> String {
    format!("{} = {}", equation.left(), equation.right())
}

/// Executes one CLI command without performing I/O.
///
/// This function is deterministic for fixed inputs and is suitable for tests.
pub fn run(args: &[&str]) -> Result<String, CliError> {
    match args {
        [] | ["--help" | "-h" | "help"] => Ok(HELP.to_owned()),
        ["expr", input] => Ok(format!("expression: {}", expression(input)?)),
        ["expand", input] | ["expand", input, _] => {
            let budget = match args {
                ["expand", _, value] => value
                    .parse::<usize>()
                    .map_err(|_| invalid("MAX_STEPS must be a nonnegative integer"))?,
                _ => 8,
            };
            if budget > MAX_DISTRIBUTION_STEPS {
                return Err(invalid(format!(
                    "MAX_STEPS must not exceed {MAX_DISTRIBUTION_STEPS}"
                )));
            }
            let parsed = expression(input)?;
            let result = EvaluationContext::default()
                .with_distribution_steps(budget)
                .expand(&parsed)
                .map_err(|error| invalid(error.to_string()))?;
            let mut out = format!(
                "result: {}\nsteps: {}\ncomplete: {}",
                result.result(),
                result.steps().len(),
                result.is_complete()
            );
            for (index, step) in result.steps().iter().enumerate() {
                write!(
                    &mut out,
                    "\n{}. {} -> {} [{}]",
                    index + 1,
                    step.before(),
                    step.after(),
                    step.rule().stable_id()
                )
                .expect("writing to a String is infallible");
            }
            Ok(out)
        }
        ["solve", input, name] => {
            let eq = equation(input)?;
            let symbol = variable(name)?;
            let solved = EvaluationContext::default()
                .solve_linear(&eq, &symbol)
                .map_err(|error| invalid(error.to_string()))?;
            let mut out = match solved.solution() {
                LinearSolution::Unique(value) => format!("solution: {symbol} = {value}"),
                LinearSolution::NoSolution => "solution: none in Q".to_owned(),
                LinearSolution::AllRationals => "solution: all rational values".to_owned(),
            };
            write!(&mut out, "\nsteps: {}", solved.steps().len())
                .expect("writing to a String is infallible");
            for (index, step) in solved.steps().iter().enumerate() {
                write!(
                    &mut out,
                    "\n{}. {} -> {} [{}]",
                    index + 1,
                    show_equation(step.before()),
                    show_equation(step.after()),
                    step.rule().stable_id()
                )
                .expect("writing to a String is infallible");
                if let Some(VerifiedCondition::Nonzero(value)) = step.verified_condition() {
                    write!(&mut out, " (verified: {value} != 0)")
                        .expect("writing to a String is infallible");
                }
            }
            Ok(out)
        }
        ["poly", input, name] => {
            let symbol = variable(name)?;
            let poly = polynomial(input, &symbol)?;
            let degree = poly
                .degree()
                .map_or_else(|| "undefined".to_owned(), |n| n.to_string());
            let coefficients = poly
                .coefficients()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!(
                "polynomial: {}\ndegree: {degree}\ncoefficients: [{coefficients}]",
                poly.to_expr(&symbol)
            ))
        }
        ["eval", input, name, value] => {
            let symbol = variable(name)?;
            let poly = polynomial(input, &symbol)?;
            let at = scalar(value)?;
            let evaluated = EvaluationContext::default()
                .evaluate_polynomial(&poly, &at)
                .map_err(|error| invalid(error.to_string()))?;
            Ok(format!("value: {evaluated}"))
        }
        ["divide", dividend, divisor, name] => {
            let symbol = variable(name)?;
            let left = polynomial(dividend, &symbol)?;
            let right = polynomial(divisor, &symbol)?;
            let (quotient, remainder) = EvaluationContext::default()
                .divide(&left, &right)
                .map_err(|error| invalid(error.to_string()))?;
            Ok(format!(
                "quotient: {}\nremainder: {}",
                quotient.to_expr(&symbol),
                remainder.to_expr(&symbol)
            ))
        }
        ["gcd", left, right, name] => {
            let symbol = variable(name)?;
            let first = polynomial(left, &symbol)?;
            let second = polynomial(right, &symbol)?;
            Ok(format!(
                "gcd: {}",
                EvaluationContext::default()
                    .gcd(&first, &second)
                    .map_err(|error| invalid(error.to_string()))?
                    .to_expr(&symbol)
            ))
        }
        ["diff", input, name] => {
            let symbol = variable(name)?;
            let poly = polynomial(input, &symbol)?;
            let derivative = EvaluationContext::default()
                .differentiate(&poly)
                .map_err(|error| invalid(error.to_string()))?;
            Ok(format!("derivative: {}", derivative.to_expr(&symbol)))
        }
        ["sqfree", input, name] => {
            let symbol = variable(name)?;
            let poly = polynomial(input, &symbol)?;
            let decomposition = EvaluationContext::default()
                .square_free_decomposition(&poly)
                .map_err(|error| invalid(error.to_string()))?;
            let mut out = format!("unit: {}", decomposition.unit());
            for entry in decomposition.factors() {
                write!(
                    &mut out,
                    "\nfactor (multiplicity {}): {}",
                    entry.multiplicity(),
                    entry.polynomial().to_expr(&symbol)
                )
                .expect("writing to a String is infallible");
            }
            Ok(out)
        }
        _ => Err(invalid("invalid command or argument count; use --help")),
    }
}
