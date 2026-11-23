//! Checked CLI adapters for natural equation solving, roots and calculus.

use std::fmt::Write as _;

use mathema::{DomainExpression, Equation, Expr, ExprView, ScalarDomain, Symbol};
use mathema_frontend::{
    DomainStatement, parse_domain_expression_in_domain, parse_domain_statement_in_domain,
};
use mathematishia_kernel::{
    AnalyticConstantLimits, AnalyticConstantOutcome, ElementaryIntegrationError, EvaluationContext,
    EvaluationError, ExactQuadraticRoot, IntegrationError, LinearSolution, PartialPolynomialRootError,
    PolynomialZeroSet, RationalPolynomial, RationalQuadraticRoots,
    rewrite_analytic_constant_once, verify_analytic_constant_rewrite,
    verify_partial_polynomial_roots, verify_rational_quadratic_equation,
};

use super::{CliError, CliErrorKind, check_domain, condition_text, invalid, problem_error, scalar, variable};

fn unsupported(message: impl Into<String>) -> CliError {
    problem_error(CliErrorKind::Unsupported, message)
}

fn verification(message: impl Into<String>) -> CliError {
    problem_error(CliErrorKind::Verification, message)
}

fn evaluation_error(error: EvaluationError) -> CliError {
    match error {
        EvaluationError::Limit(_)
        | EvaluationError::Polynomial(
            mathematishia_kernel::PolynomialConversionError::DegreeLimitExceeded,
        ) => problem_error(CliErrorKind::Resource, error.to_string()),
        EvaluationError::Polynomial(_) => unsupported(error.to_string()),
        _ => invalid(error.to_string()),
    }
}

fn show_root(root: &ExactQuadraticRoot, variable: &Symbol) -> String {
    match root {
        ExactQuadraticRoot::Rational(value) => value.to_string(),
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch,
        } => format!(
            "{} of minimal polynomial {}",
            branch.stable_id(),
            minimal_polynomial.to_expr(variable)
        ),
    }
}

fn polynomial_roots(
    context: &EvaluationContext,
    polynomial: &RationalPolynomial,
    symbol: &Symbol,
) -> Result<String, CliError> {
    let answer = context
        .partial_polynomial_roots(polynomial)
        .map_err(|error| match error {
            PartialPolynomialRootError::Evaluation(error) => evaluation_error(error),
            _ => unsupported(error.to_string()),
        })?;
    if !verify_partial_polynomial_roots(context, polynomial, &answer)
        .map_err(|error| verification(error.to_string()))?
    {
        return Err(verification("polynomial root evidence failed verification"));
    }
    let mut output = format!("polynomial: {}", polynomial.to_expr(symbol));
    match answer.zero_set() {
        PolynomialZeroSet::AllComplexNumbers => {
            output.push_str("\nstatus: exact\ndomain: C\nsolution: all complex numbers");
        }
        PolynomialZeroSet::Finite {
            roots,
            unresolved_cofactor,
            complete,
        } => {
            write!(
                &mut output,
                "\nstatus: {}\ndomain: C\nroots: {}",
                if *complete { "exact" } else { "partial" },
                roots.len()
            )
            .expect("writing to String is infallible");
            for (index, item) in roots.iter().enumerate() {
                write!(
                    &mut output,
                    "\nroot {}: {} ; multiplicity: {}",
                    index + 1,
                    show_root(item.root(), symbol),
                    item.multiplicity()
                )
                .expect("writing to String is infallible");
            }
            if let Some(cofactor) = unresolved_cofactor {
                write!(&mut output, "\nunresolved_cofactor: {}", cofactor.to_expr(symbol))
                    .expect("writing to String is infallible");
                output.push_str("\nnote: additional complex roots remain unresolved");
            }
        }
    }
    output.push_str("\nverified: true");
    Ok(output)
}

pub(super) fn roots(input: &str, name: &str) -> Result<String, CliError> {
    let symbol = variable(name)?;
    let source = parse_domain_expression_in_domain(input, ScalarDomain::Complex)
        .map_err(|error| invalid(error.to_string()))?;
    check_domain(&source)?;
    if !source.conditions().is_empty() {
        return Err(problem_error(
            CliErrorKind::Domain,
            "polynomial root solving cannot discard source-domain exclusions",
        ));
    }
    let ctx = EvaluationContext::default();
    let polynomial = ctx
        .polynomial(source.expression(), &symbol)
        .map_err(evaluation_error)?;
    polynomial_roots(&ctx, &polynomial, &symbol)
}

fn rational_solutions(
    context: &EvaluationContext,
    left: &DomainExpression,
    right: &DomainExpression,
    symbol: &Symbol,
) -> Result<String, CliError> {
    let left = context
        .rational_expression(left, symbol)
        .map_err(|error| unsupported(format!("unsupported left-hand side: {error}")))?;
    let right = context
        .rational_expression(right, symbol)
        .map_err(|error| unsupported(format!("unsupported right-hand side: {error}")))?;
    let solved = context
        .solve_rational_quadratic_equation(&left, &right)
        .map_err(evaluation_error)?;
    verify_rational_quadratic_equation(&solved)
        .map_err(|error| verification(error.to_string()))?;
    let mut output = String::from(
        "status: partial over C\nexact_scope: rational solutions in Q; non-rational complex roots not classified",
    );
    match solved.roots() {
        RationalQuadraticRoots::AllAdmissibleRationals => {
            output.push_str("\nsolution: all admissible rational values");
        }
        RationalQuadraticRoots::Finite(values) if values.is_empty() => {
            output.push_str("\nrational roots: none");
        }
        RationalQuadraticRoots::Finite(values) => {
            write!(
                &mut output,
                "\nrational roots: {}",
                values.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
            )
            .expect("writing to String is infallible");
        }
    }
    if solved.domain_conditions().is_empty() {
        output.push_str("\nconditions: none");
    } else {
        for restriction in solved.domain_conditions() {
            write!(&mut output, "\ncondition: {} != 0", restriction.to_expr(symbol))
                .expect("writing to String is infallible");
        }
    }
    output.push_str("\nverified: true");
    Ok(output)
}

pub(super) fn solve_auto(input: &str, name: &str) -> Result<String, CliError> {
    let symbol = variable(name)?;
    let statement = parse_domain_statement_in_domain(input, ScalarDomain::Complex)
        .map_err(|error| invalid(error.to_string()))?;
    let DomainStatement::Equation { left, right } = statement else {
        return Err(invalid("solveauto expects an equation containing '='"));
    };
    check_domain(&left)?;
    check_domain(&right)?;
    let ctx = EvaluationContext::default();
    if left.conditions().is_empty() && right.conditions().is_empty() {
        let residual = Expr::add([
            left.expression().clone(),
            Expr::mul([Expr::from(-1), right.expression().clone()]),
        ]);
        match ctx.polynomial(&residual, &symbol) {
            Ok(polynomial) => {
                if polynomial.degree().is_none_or(|degree| degree <= 1) {
                    let equation = Equation::new(
                        left.expression().clone(),
                        right.expression().clone(),
                    );
                    let result = ctx.solve_linear(&equation, &symbol).map_err(evaluation_error)?;
                    return Ok(match result.solution() {
                        LinearSolution::Unique(value) => format!(
                            "status: exact\ndomain: Q\nsolution: {symbol} = {value}\nverified: true"
                        ),
                        LinearSolution::NoSolution => {
                            "status: exact\ndomain: Q\nsolution: none\nverified: true".to_owned()
                        }
                        LinearSolution::AllRationals => {
                            "status: exact\ndomain: Q\nsolution: all rational values\nverified: true"
                                .to_owned()
                        }
                    });
                }
                return polynomial_roots(&ctx, &polynomial, &symbol);
            }
            Err(EvaluationError::Polynomial(_)) => {}
            Err(error) => return Err(evaluation_error(error)),
        }
    }
    rational_solutions(&ctx, &left, &right, &symbol)
}

pub(super) fn integrate(
    input: &str,
    name: &str,
    bounds: Option<(&str, &str)>,
) -> Result<String, CliError> {
    let symbol = variable(name)?;
    let source = parse_domain_expression_in_domain(input, ScalarDomain::Real)
        .map_err(|error| invalid(error.to_string()))?;
    check_domain(&source)?;
    let ctx = EvaluationContext::default();
    if let Some((lower, upper)) = bounds {
        let lower = scalar(lower)?;
        let upper = scalar(upper)?;
        let result = ctx
            .integrate_domain_expression_between(&source, &symbol, &lower, &upper)
            .map_err(|error| unsupported(error.to_string()))?;
        ctx.verify_definite_polynomial_integral(&result)
            .map_err(|error| verification(error.to_string()))?;
        return Ok(format!(
            "status: exact\ndomain: R\nintegral: {}\nfrom: {}\nto: {}\nprimitive: {}\nconditions: none\nverified: true",
            result.value(),
            result.lower(),
            result.upper(),
            result.primitive().primitive().to_expr(&symbol)
        ));
    }
    match ctx.integrate_domain_expression(&source, &symbol) {
        Ok(result) => {
            ctx.verify_polynomial_primitive(&result)
                .map_err(|error| verification(error.to_string()))?;
            Ok(format!(
                "status: exact\ndomain: R\nprimitive: {}\nconstant: 0 (canonical choice)\nconditions: none\nverified: true",
                result.primitive().to_expr(&symbol)
            ))
        }
        Err(IntegrationError::UnsupportedIntegrand(_))
        | Err(IntegrationError::RestrictedDomain) => {
            let result = ctx
                .integrate_elementary_affine(&source, &symbol, ScalarDomain::Real)
                .map_err(|error| match error {
                    ElementaryIntegrationError::Evaluation(EvaluationError::Limit(_)) => {
                        problem_error(CliErrorKind::Resource, error.to_string())
                    }
                    _ => unsupported(error.to_string()),
                })?;
            ctx.verify_elementary_antiderivative(
                &source,
                &symbol,
                ScalarDomain::Real,
                &result,
            )
            .map_err(|error| verification(error.to_string()))?;
            let mut output = format!(
                "status: {}\ndomain: R\nprimitive: {}\nconstant: 0 (canonical choice)\nrule: {}",
                if result.conditions().is_empty() { "exact" } else { "conditional" },
                result.primitive().expression(),
                result.rule().stable_id()
            );
            if result.conditions().is_empty() {
                output.push_str("\nconditions: none");
            } else {
                for condition in result.conditions() {
                    write!(&mut output, "\nrequired: {}", condition_text(condition))
                        .expect("writing to String is infallible");
                }
            }
            output.push_str("\nverified: true");
            Ok(output)
        }
        Err(error) => Err(unsupported(error.to_string())),
    }
}

pub(super) fn eval_exact(input: &str) -> Result<String, CliError> {
    let limits = AnalyticConstantLimits::default();
    let mut source = parse_domain_expression_in_domain(input, ScalarDomain::Real)
        .map_err(|error| problem_error(CliErrorKind::Domain, error.to_string()))?;
    check_domain(&source)?;
    let mut rules = Vec::new();
    for _ in 0..16 {
        let outcome = rewrite_analytic_constant_once(&source, ScalarDomain::Real, limits)
            .map_err(|error| problem_error(CliErrorKind::Domain, error.to_string()))?;
        match outcome {
            AnalyticConstantOutcome::Exact(proof) => {
                if !verify_analytic_constant_rewrite(
                    &source,
                    ScalarDomain::Real,
                    &proof,
                    limits,
                )
                .map_err(|error| verification(error.to_string()))?
                {
                    return Err(verification("analytic constant identity failed verification"));
                }
                rules.push(proof.rule.stable_id());
                source = proof.result;
            }
            AnalyticConstantOutcome::NoRule { .. } => break,
            AnalyticConstantOutcome::Unsupported(reason) => {
                return Err(unsupported(format!("exact evaluation unsupported: {reason:?}")));
            }
        }
    }
    let mut output = match source.expression().view() {
        ExprView::Integer(value) => format!("status: exact\nvalue: {value}"),
        ExprView::Rational(value) => format!("status: exact\nvalue: {value}"),
        _ => format!(
            "status: partial\nexpression: {}\nnote: general analytic evaluation is unsupported",
            source.expression()
        ),
    };
    if source.conditions().is_empty() {
        output.push_str("\nconditions: none");
    } else {
        for condition in source.conditions() {
            write!(&mut output, "\nrequired: {}", condition_text(condition))
                .expect("writing to String is infallible");
        }
    }
    for rule in rules {
        write!(&mut output, "\nrule: {rule}").expect("writing to String is infallible");
    }
    output.push_str("\nverified: true");
    Ok(output)
}
