//! Command-line entry points for the existing exact Mathematishia kernel.
//!
//! The CLI owns input/output formatting only; mathematical behavior lives in
//! mathema-frontend and mathematishia-kernel.

use std::fmt;
use std::fmt::Write as _;

use mathema::{Equation, Expr, ExprView, Symbol};
use mathema_frontend::{Statement, parse_expression, parse_statement};
use mathematishia_kernel::{
    LinearSolution, VerifiedCondition, expand_distributively, solve_linear_equation,
};

pub const HELP: &str = "Mathematishia (early exact-math CLI)
Usage:
  mathematishia expr <EXPRESSION>
  mathematishia expand <EXPRESSION> [MAX_STEPS]
  mathematishia solve <EQUATION> <VARIABLE>
  mathematishia --help

Examples:
  mathematishia expr '(x+1)^2'
  mathematishia expand '2*(x+3)' 4
  mathematishia solve '2*x+4=10' x

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
    parse_expression(input).map_err(|error| invalid(error.to_string()))
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
            let result = expand_distributively(&parsed, budget);
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
            let solved = solve_linear_equation(&eq, &symbol)
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
        _ => Err(invalid("invalid command or argument count; use --help")),
    }
}
