//! Text admission for opt-in school mathematics, with original source domains.
use crate::{
    DomainStatement, RealComparisonOperator,
    advanced::AdvancedError,
    natural, parse_bound_domain_expression, parse_bound_domain_statement,
    parse_bound_real_comparison,
    problem::{self, RequestError},
};
use mathema::{DomainExpression, Expr, ExprView, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    AbsoluteAffine, ComparisonOperator, EvaluationContext, ExactQuadraticRoot, GuardedAffine,
    InfinityEnd, PiMultiple, PiecewiseAffine, PiecewiseEquation, PiecewiseInterval,
    PiecewiseSourceDomain, ProblemOperation, ProblemRequest, ProblemResult, ProblemStatement,
    SchoolError, SpecialAngleOutcome, TrigFunction, WeightedAbsoluteTerm, checked_infinity_json,
    checked_periodic_json, checked_piecewise_json, checked_school_json, checked_sqrt_json,
    checked_trig_identity_json,
};

fn school_error(error: SchoolError) -> RequestError {
    match error {
        SchoolError::Evaluation(error) => RequestError::Evaluation(error),
        SchoolError::Problem(error) => RequestError::Problem(error),
        SchoolError::Wire(error) => RequestError::Wire(error),
        SchoolError::Unsupported(message) => advanced(AdvancedError::Unsupported(message)),
        SchoolError::Verification(message) => advanced(AdvancedError::Verification(message)),
        SchoolError::Domain(message) => advanced(AdvancedError::Domain(message)),
        SchoolError::Resource(message) => advanced(AdvancedError::Resource(message)),
    }
}
fn advanced(error: AdvancedError) -> RequestError {
    RequestError::Advanced(Box::new(error))
}
fn source(input: &str, domain: ScalarDomain) -> Result<DomainExpression, RequestError> {
    parse_bound_domain_expression(input, domain).map_err(RequestError::Parse)
}
fn domain(name: &str) -> Result<ScalarDomain, RequestError> {
    match name {
        "Q" => Ok(ScalarDomain::Rational),
        "R" => Ok(ScalarDomain::Real),
        "C" => Ok(ScalarDomain::Complex),
        _ => Err(RequestError::Invalid("expected Q, R or C")),
    }
}
fn operands(
    input: &str,
    domain: ScalarDomain,
) -> Result<(DomainExpression, DomainExpression), RequestError> {
    match parse_bound_domain_statement(input, domain).map_err(RequestError::Parse)? {
        DomainStatement::Equation { left, right } => Ok((left, right)),
        _ => Err(RequestError::Invalid("expected an equation")),
    }
}
fn comparison(input: &str) -> Result<ProblemStatement, RequestError> {
    let parsed = parse_bound_real_comparison(input).map_err(RequestError::Parse)?;
    let operator = match parsed.operator {
        RealComparisonOperator::Less => ComparisonOperator::Less,
        RealComparisonOperator::LessOrEqual => ComparisonOperator::LessOrEqual,
        RealComparisonOperator::Greater => ComparisonOperator::Greater,
        RealComparisonOperator::GreaterOrEqual => ComparisonOperator::GreaterOrEqual,
    };
    Ok(ProblemStatement::Comparison {
        left: parsed.left,
        right: parsed.right,
        operator,
    })
}
fn expression_request(
    context: &EvaluationContext,
    operation: ProblemOperation,
    input: &str,
    name: &str,
) -> Result<ProblemRequest, RequestError> {
    Ok(ProblemRequest {
        operation,
        statement: ProblemStatement::Expression(source(input, ScalarDomain::Real)?),
        variables: vec![problem::variable(context, name)?],
        domain: ScalarDomain::Real,
    })
}
fn equation_request(
    context: &EvaluationContext,
    input: &str,
    name: &str,
    domain: ScalarDomain,
) -> Result<ProblemRequest, RequestError> {
    let (left, right) = operands(input, domain)?;
    Ok(ProblemRequest {
        operation: ProblemOperation::SolveSourceEquation,
        statement: ProblemStatement::SourceEquation { left, right },
        variables: vec![problem::variable(context, name)?],
        domain,
    })
}
fn natural_request(
    context: &EvaluationContext,
    input: &str,
    name: &str,
) -> Result<ProblemRequest, RequestError> {
    let mut request = natural::parse_v2(context, input, name).map_err(advanced)?;
    // Re-lower the validated original syntax, never a normalized display expression.
    let normalized: String = input
        .chars()
        .map(|c| match c {
            '−' => '-',
            '×' => '*',
            '÷' => '/',
            _ => c,
        })
        .collect();
    let original = normalized
        .split_once(':')
        .map_or(normalized.as_str(), |(_, rest)| rest)
        .trim();
    request.statement = match request.statement {
        ProblemStatement::Expression(_) => {
            ProblemStatement::Expression(source(original, request.domain)?)
        }
        ProblemStatement::SourceEquation { .. } => {
            let (left, right) = operands(original, request.domain)?;
            ProblemStatement::SourceEquation { left, right }
        }
        ProblemStatement::Comparison { .. } => comparison(original)?,
        other => other,
    };
    Ok(request)
}
fn zero() -> Rational {
    Integer::zero().into()
}
fn one() -> Rational {
    Integer::one().into()
}
fn affine(
    context: &EvaluationContext,
    expression: &Expr,
    variable: &Symbol,
) -> Result<AbsoluteAffine, RequestError> {
    let poly = context
        .polynomial(expression, variable)
        .map_err(RequestError::Evaluation)?;
    if poly.degree().is_some_and(|degree| degree > 1) {
        return Err(RequestError::Unsupported("expected an affine expression"));
    }
    Ok(AbsoluteAffine {
        slope: poly.coefficients().get(1).cloned().unwrap_or_else(zero),
        constant: poly.coefficients().first().cloned().unwrap_or_else(zero),
    })
}
fn literal(expression: &Expr) -> Option<Rational> {
    match expression.view() {
        ExprView::Integer(value) => Some(value.clone().into()),
        ExprView::Rational(value) => Some(value.clone()),
        _ => None,
    }
}
fn absolute_form(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
) -> Result<PiecewiseAffine, RequestError> {
    if !source.conditions().is_empty()
        || source.construction_domain() != Some(ScalarDomain::Real)
        || source.has_external_history()
    {
        return Err(advanced(AdvancedError::Domain(
            "piecewise admission requires an unrestricted real source".into(),
        )));
    }
    context
        .check_expression(source.expression())
        .map_err(RequestError::Evaluation)?;
    let parts = match source.expression().view() {
        ExprView::Add(terms) => terms,
        _ => std::slice::from_ref(source.expression()),
    };
    let mut base = AbsoluteAffine {
        slope: zero(),
        constant: zero(),
    };
    let mut terms = Vec::new();
    for part in parts {
        let factors = match part.view() {
            ExprView::Mul(factors) => factors,
            _ => std::slice::from_ref(part),
        };
        let mut absolute = None;
        let mut weight = one();
        let mut all_other_scalar = true;
        for factor in factors {
            match factor.view() {
                ExprView::Function { name, argument } if name.name() == "abs" => {
                    if absolute.replace(argument).is_some() {
                        return Err(RequestError::Unsupported(
                            "products of absolute values are unsupported",
                        ));
                    }
                }
                _ => match literal(factor) {
                    Some(value) => {
                        weight = &weight * &value;
                        context
                            .check_rational(&weight)
                            .map_err(RequestError::Evaluation)?;
                    }
                    None => all_other_scalar = false,
                },
            }
        }
        if let Some(argument) = absolute {
            if !all_other_scalar {
                return Err(RequestError::Unsupported(
                    "absolute terms require a rational multiplier",
                ));
            }
            terms.push(WeightedAbsoluteTerm {
                weight,
                argument: affine(context, argument, variable)?,
            });
            if terms.len() > 16 {
                return Err(advanced(AdvancedError::Resource(
                    "absolute term limit exceeded".into(),
                )));
            }
        } else {
            let term = affine(context, part, variable)?;
            base.slope = &base.slope + &term.slope;
            base.constant = &base.constant + &term.constant;
            context
                .check_rational(&base.slope)
                .map_err(RequestError::Evaluation)?;
            context
                .check_rational(&base.constant)
                .map_err(RequestError::Evaluation)?;
        }
    }
    Ok(PiecewiseAffine::AbsoluteSum { base, terms })
}
fn abs_sum(context: &EvaluationContext, input: &str, name: &str) -> Result<String, RequestError> {
    let (left, right) = operands(input, ScalarDomain::Real)?;
    let variable = problem::variable(context, name)?;
    let equation = PiecewiseEquation {
        left: absolute_form(context, &left, &variable)?,
        right: absolute_form(context, &right, &variable)?,
        variable,
        domain: ScalarDomain::Real,
        source_domain: PiecewiseSourceDomain::Total,
    };
    checked_piecewise_json(context, &equation).map_err(school_error)
}
fn explicit_form(
    context: &EvaluationContext,
    input: &str,
) -> Result<PiecewiseAffine, RequestError> {
    let mut cases = Vec::new();
    for row in input.split(';') {
        let values: Vec<_> = row.split(',').map(str::trim).collect();
        let [lower, upper, lc, uc, slope, constant] = values.as_slice() else {
            return Err(RequestError::Invalid(
                "case requires lower,upper,lower_closed,upper_closed,slope,constant",
            ));
        };
        let closed = |value: &str| match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(RequestError::Invalid("closure must be 0 or 1")),
        };
        cases.push(GuardedAffine {
            guard: PiecewiseInterval {
                lower: if *lower == "-inf" {
                    None
                } else {
                    Some(problem::scalar(context, lower)?)
                },
                upper: if *upper == "+inf" {
                    None
                } else {
                    Some(problem::scalar(context, upper)?)
                },
                lower_closed: closed(lc)?,
                upper_closed: closed(uc)?,
            },
            form: AbsoluteAffine {
                slope: problem::scalar(context, slope)?,
                constant: problem::scalar(context, constant)?,
            },
        });
        if cases.len() > 16 {
            return Err(advanced(AdvancedError::Resource(
                "piecewise case limit exceeded".into(),
            )));
        }
    }
    Ok(PiecewiseAffine::Explicit(cases))
}
fn trig_function(name: &str) -> Result<TrigFunction, RequestError> {
    match name {
        "sin" => Ok(TrigFunction::Sin),
        "cos" => Ok(TrigFunction::Cos),
        "tan" => Ok(TrigFunction::Tan),
        _ => Err(RequestError::Invalid("expected sin, cos or tan")),
    }
}
fn periodic_target(
    context: &EvaluationContext,
    target: &str,
    name: &str,
) -> Result<ExactQuadraticRoot, RequestError> {
    if target.contains("pi") || target.contains('π') {
        let request = natural::parse_v1(context, target, name).map_err(advanced)?;
        let natural::NaturalOutcome::SpecialAngle(result) =
            natural::evaluate_v1(context, &request).map_err(advanced)?
        else {
            return Err(RequestError::Invalid("expected a special-angle target"));
        };
        match result.outcome {
            SpecialAngleOutcome::Exact(value) => Ok(value),
            SpecialAngleOutcome::UndefinedTangentPole => Err(advanced(AdvancedError::Domain(
                "undefined tangent target".into(),
            ))),
        }
    } else {
        Ok(ExactQuadraticRoot::Rational(problem::scalar(
            context, target,
        )?))
    }
}
pub fn evaluate_json(context: &EvaluationContext, args: &[&str]) -> Result<String, RequestError> {
    if args.len() > 32
        || args
            .iter()
            .try_fold(0usize, |total, value| total.checked_add(value.len()))
            .is_none_or(|total| total > 32768)
    {
        return Err(advanced(AdvancedError::Resource(
            "school request limit exceeded".into(),
        )));
    }
    let request = match args {
        ["natural", input] => natural_request(context, input, "x")?,
        ["natural", input, name] => natural_request(context, input, name)?,
        ["solve", input, name] => equation_request(context, input, name, ScalarDomain::Real)?,
        ["solve", input, name, field] => equation_request(context, input, name, domain(field)?)?,
        ["differentiate", input, name] => {
            expression_request(context, ProblemOperation::Differentiate, input, name)?
        }
        ["integrate", input, name] => {
            expression_request(context, ProblemOperation::Integrate, input, name)?
        }
        ["inequality", input, name] => ProblemRequest {
            operation: ProblemOperation::Inequality,
            statement: comparison(input)?,
            variables: vec![problem::variable(context, name)?],
            domain: ScalarDomain::Real,
        },
        ["limit", input, name, "+infinity" | "-infinity"] => {
            let end = if args[3] == "+infinity" {
                InfinityEnd::Positive
            } else {
                InfinityEnd::Negative
            };
            return checked_infinity_json(
                context,
                &source(input, ScalarDomain::Real)?,
                &problem::variable(context, name)?,
                end,
            )
            .map_err(school_error);
        }
        ["limit", input, _, _] | ["limit", input, _, _, _] => {
            let mut request = problem::request(context, args)?;
            if let ProblemStatement::Limit { source: value, .. } = &mut request.statement {
                *value = source(input, ScalarDomain::Real)?;
            }
            request
        }
        ["sqrt", input] => {
            return checked_sqrt_json(context, &source(input, ScalarDomain::Real)?)
                .map_err(school_error);
        }
        ["periodic", function, slope, shift, target, name] => {
            let variable = problem::variable(context, name)?;
            let equation = trig_function(function)?.periodic_equation(
                problem::scalar(context, slope)?,
                PiMultiple::new(problem::scalar(context, shift)?),
                periodic_target(context, target, name)?,
                ScalarDomain::Real,
            );
            return checked_periodic_json(context, &equation, &variable).map_err(school_error);
        }
        ["trig_identity", a, b, c, d] => {
            let coefficients = [
                problem::scalar(context, a)?,
                problem::scalar(context, b)?,
                problem::scalar(context, c)?,
                problem::scalar(context, d)?,
            ];
            return checked_trig_identity_json(context, &coefficients).map_err(school_error);
        }
        ["abs_sum", input, name] => return abs_sum(context, input, name),
        ["piecewise", name, left, right] => {
            let equation = PiecewiseEquation {
                variable: problem::variable(context, name)?,
                domain: ScalarDomain::Real,
                source_domain: PiecewiseSourceDomain::Total,
                left: explicit_form(context, left)?,
                right: explicit_form(context, right)?,
            };
            return checked_piecewise_json(context, &equation).map_err(school_error);
        }
        ["methods", rest @ ..] => {
            return ProblemResult::dispatch_with_methods(
                context,
                &problem::request(context, rest)?,
                true,
            )
            .map_err(RequestError::Problem)?
            .to_json()
            .map_err(RequestError::Wire);
        }
        _ => {
            return Err(RequestError::Invalid(
                "expected a supported school operation",
            ));
        }
    };
    checked_school_json(context, &request).map_err(school_error)
}
