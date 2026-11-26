//! Shared text-to-request adapter. Mathematics and wire encoding remain in the kernel.

use std::{collections::BTreeSet, error::Error, fmt};

use mathema::{DomainExpression, Expr, ExprView, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    ComparisonOperator, EvaluationContext, EvaluationError, LimitDirection, PiMultiple,
    ProblemError, ProblemOperation, ProblemRequest, ProblemResult, ProblemStatement,
    ProblemWireError, TrigFunction, dispatch_problem,
};

use crate::{
    DomainParseError, DomainStatement, RealComparisonOperator, parse_domain_expression_in_domain,
    parse_domain_statement_in_domain, parse_expression, parse_real_comparison,
};

#[derive(Debug)]
pub enum RequestError {
    Parse(DomainParseError),
    Invalid(&'static str),
    Unsupported(&'static str),
    Evaluation(EvaluationError),
    Problem(ProblemError),
    Wire(ProblemWireError),
    Advanced(Box<crate::advanced::AdvancedError>),
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Invalid(message) | Self::Unsupported(message) => f.write_str(message),
            Self::Evaluation(error) => error.fmt(f),
            Self::Problem(error) => error.fmt(f),
            Self::Wire(error) => error.fmt(f),
            Self::Advanced(error) => error.fmt(f),
        }
    }
}

impl Error for RequestError {}

pub fn variable(context: &EvaluationContext, name: &str) -> Result<Symbol, RequestError> {
    let expr = parse_expression(name).map_err(|e| RequestError::Parse(e.into()))?;
    context
        .check_expression(&expr)
        .map_err(RequestError::Evaluation)?;
    match expr.view() {
        ExprView::Symbol(symbol) if symbol.name() == name.trim() => Ok(symbol.clone()),
        _ => Err(RequestError::Invalid("expected one variable identifier")),
    }
}

pub fn source(input: &str, domain: ScalarDomain) -> Result<DomainExpression, RequestError> {
    parse_domain_expression_in_domain(input, domain).map_err(RequestError::Parse)
}

pub fn scalar(context: &EvaluationContext, input: &str) -> Result<Rational, RequestError> {
    let expr = source(input, ScalarDomain::Real)?;
    if !expr.conditions().is_empty() {
        return Err(RequestError::Unsupported(
            "scalar input has retained domain restrictions",
        ));
    }
    context
        .check_expression(expr.expression())
        .map_err(RequestError::Evaluation)?;
    let value = match expr.expression().view() {
        ExprView::Integer(value) => Rational::from(value.clone()),
        ExprView::Rational(value) => value.clone(),
        _ => {
            return Err(RequestError::Unsupported(
                "expected an exact rational scalar",
            ));
        }
    };
    context
        .check_rational(&value)
        .map_err(RequestError::Evaluation)?;
    Ok(value)
}

fn collect_symbols(expression: &Expr, found: &mut BTreeSet<Symbol>) {
    match expression.view() {
        ExprView::Symbol(symbol) => {
            found.insert(symbol.clone());
        }
        ExprView::Add(children) | ExprView::Mul(children) => {
            for child in children {
                collect_symbols(child, found);
            }
        }
        ExprView::Pow { base, exponent } => {
            collect_symbols(base, found);
            collect_symbols(exponent, found);
        }
        ExprView::Function { argument, .. } => collect_symbols(argument, found),
        _ => {}
    }
}

fn comparison(
    context: &EvaluationContext,
    input: &str,
    name: &str,
) -> Result<ProblemRequest, RequestError> {
    let parsed = parse_real_comparison(input).map_err(RequestError::Parse)?;
    let operator = match parsed.operator {
        RealComparisonOperator::Less => ComparisonOperator::Less,
        RealComparisonOperator::LessOrEqual => ComparisonOperator::LessOrEqual,
        RealComparisonOperator::Greater => ComparisonOperator::Greater,
        RealComparisonOperator::GreaterOrEqual => ComparisonOperator::GreaterOrEqual,
    };
    Ok(ProblemRequest {
        operation: ProblemOperation::Inequality,
        statement: ProblemStatement::Comparison {
            left: parsed.left,
            right: parsed.right,
            operator,
        },
        variables: vec![variable(context, name)?],
        domain: ScalarDomain::Real,
    })
}

fn equation(
    context: &EvaluationContext,
    input: &str,
    name: &str,
) -> Result<ProblemRequest, RequestError> {
    if input
        .chars()
        .any(|c| matches!(c, '<' | '>' | '\u{2264}' | '\u{2265}'))
    {
        return comparison(context, input, name);
    }
    let statement = parse_domain_statement_in_domain(input, ScalarDomain::Complex)
        .map_err(RequestError::Parse)?;
    let DomainStatement::Equation { left, right } = statement else {
        return Err(RequestError::Invalid("expected an equation containing '='"));
    };
    Ok(ProblemRequest {
        operation: ProblemOperation::SolveAuto,
        statement: ProblemStatement::SourceEquation { left, right },
        variables: vec![variable(context, name)?],
        domain: ScalarDomain::Complex,
    })
}

fn expression_request(
    context: &EvaluationContext,
    operation: ProblemOperation,
    input: &str,
    name: &str,
    domain: ScalarDomain,
) -> Result<ProblemRequest, RequestError> {
    Ok(ProblemRequest {
        operation,
        statement: ProblemStatement::Expression(source(input, domain)?),
        variables: vec![variable(context, name)?],
        domain,
    })
}

fn roots(
    context: &EvaluationContext,
    input: &str,
    name: &str,
) -> Result<ProblemRequest, RequestError> {
    let symbol = variable(context, name)?;
    let expression = source(input, ScalarDomain::Complex)?;
    if !expression.conditions().is_empty() {
        return Err(RequestError::Unsupported(
            "polynomial roots cannot discard source restrictions",
        ));
    }
    let polynomial = context
        .polynomial(expression.expression(), &symbol)
        .map_err(RequestError::Evaluation)?;
    Ok(ProblemRequest {
        operation: ProblemOperation::ComplexPolynomialRoots,
        statement: ProblemStatement::PolynomialZero(polynomial),
        variables: vec![symbol],
        domain: ScalarDomain::Complex,
    })
}

fn limit(
    context: &EvaluationContext,
    input: &str,
    name: &str,
    at: &str,
    side: &str,
) -> Result<ProblemRequest, RequestError> {
    let direction = match side {
        "left" => LimitDirection::Left,
        "right" => LimitDirection::Right,
        "two-sided" => LimitDirection::TwoSided,
        _ => {
            return Err(RequestError::Invalid(
                "direction must be left, right or two-sided",
            ));
        }
    };
    Ok(ProblemRequest {
        operation: ProblemOperation::Limit,
        statement: ProblemStatement::Limit {
            source: source(input, ScalarDomain::Real)?,
            at: scalar(context, at)?,
            direction,
        },
        variables: vec![variable(context, name)?],
        domain: ScalarDomain::Real,
    })
}

fn advanced_error(error: crate::advanced::AdvancedError) -> RequestError {
    RequestError::Advanced(Box::new(error))
}

fn natural_request(
    context: &EvaluationContext,
    input: &str,
    name: &str,
) -> Result<ProblemRequest, RequestError> {
    use crate::natural::NaturalRequest;
    let (operation, statement, variables) =
        match crate::natural::parse_v1(context, input, name).map_err(advanced_error)? {
            NaturalRequest::SpecialAngle { function, angle } => (
                ProblemOperation::EvaluateSpecialAngle,
                ProblemStatement::SpecialAngle { angle, function },
                vec![],
            ),
            NaturalRequest::AbsoluteEquation(equation) => (
                ProblemOperation::SolveAbsolute,
                ProblemStatement::Absolute(equation),
                vec![variable(context, name)?],
            ),
        };
    Ok(ProblemRequest {
        operation,
        statement,
        variables,
        domain: ScalarDomain::Real,
    })
}

fn trig_function(name: &str) -> Result<TrigFunction, RequestError> {
    match name {
        "sin" => Ok(TrigFunction::Sin),
        "cos" => Ok(TrigFunction::Cos),
        "tan" => Ok(TrigFunction::Tan),
        _ => Err(RequestError::Invalid("expected sin, cos or tan")),
    }
}

fn ideal_request(
    context: &EvaluationContext,
    names: &str,
    generators: &[&str],
    target: Option<&str>,
) -> Result<ProblemRequest, RequestError> {
    if generators.len() > 8 || names.split(',').count() > 3 {
        return Err(RequestError::Evaluation(EvaluationError::Limit(
            mathematishia_kernel::ResourceLimitKind::ExpressionNodes,
        )));
    }
    let variables = names
        .split(',')
        .map(|name| variable(context, name))
        .collect::<Result<Vec<_>, _>>()?;
    if variables.iter().collect::<BTreeSet<_>>().len() != variables.len() {
        return Err(RequestError::Invalid("variables must be distinct"));
    }
    let domain = ScalarDomain::Rational;
    Ok(ProblemRequest {
        operation: if target.is_some() {
            ProblemOperation::IdealMembership
        } else {
            ProblemOperation::GroebnerBasis
        },
        statement: ProblemStatement::Ideal {
            generators: generators
                .iter()
                .map(|input| source(input, domain))
                .collect::<Result<Vec<_>, _>>()?,
            target: target.map(|input| source(input, domain)).transpose()?,
        },
        variables,
        domain,
    })
}

fn absolute_request(
    context: &EvaluationContext,
    input: &str,
    name: &str,
) -> Result<ProblemRequest, RequestError> {
    let equation =
        crate::advanced::absolute_request(context, input, name).map_err(advanced_error)?;
    Ok(ProblemRequest {
        operation: ProblemOperation::SolveAbsolute,
        statement: ProblemStatement::Absolute(Box::new(equation)),
        variables: vec![variable(context, name)?],
        domain: ScalarDomain::Real,
    })
}

/// Commands accepted by the versioned result adapter. Legacy text and typed
/// interfaces remain available independently of this bounded schema.
pub fn request(context: &EvaluationContext, args: &[&str]) -> Result<ProblemRequest, RequestError> {
    match args {
        ["natural", input] => natural_request(context, input, "x"),
        ["natural", input, name] => natural_request(context, input, name),
        ["abssolve", input] => absolute_request(context, input, "x"),
        ["abssolve", input, name] => absolute_request(context, input, name),
        ["trig", function, coefficient] => Ok(ProblemRequest {
            operation: ProblemOperation::EvaluateSpecialAngle,
            statement: ProblemStatement::SpecialAngle {
                function: trig_function(function)?,
                angle: PiMultiple::new(scalar(context, coefficient)?),
            },
            variables: vec![],
            domain: ScalarDomain::Real,
        }),
        ["trigzeros", function] => Ok(ProblemRequest {
            operation: ProblemOperation::TrigZeros,
            statement: ProblemStatement::TrigZeros(trig_function(function)?),
            variables: vec![variable(context, "x")?],
            domain: ScalarDomain::Real,
        }),
        ["groebner", names, generators @ ..] => ideal_request(context, names, generators, None),
        ["idealmember", names, target, generators @ ..] => {
            ideal_request(context, names, generators, Some(target))
        }
        ["solvefield", input, name, field] => {
            let domain = match *field {
                "Q" => ScalarDomain::Rational,
                "R" => ScalarDomain::Real,
                "C" => ScalarDomain::Complex,
                _ => return Err(RequestError::Invalid("expected Q, R or C")),
            };
            let statement =
                parse_domain_statement_in_domain(input, domain).map_err(RequestError::Parse)?;
            let DomainStatement::Equation { left, right } = statement else {
                return Err(RequestError::Invalid("expected an equation containing '='"));
            };
            Ok(ProblemRequest {
                operation: ProblemOperation::SolveSourceEquation,
                statement: ProblemStatement::SourceEquation { left, right },
                variables: vec![variable(context, name)?],
                domain,
            })
        }
        ["solveauto", input] => equation(context, input, "x"),
        ["solveauto", input, name] => equation(context, input, name),
        ["inequality", input] => comparison(context, input, "x"),
        ["inequality", input, name] => comparison(context, input, name),
        ["roots", input] => roots(context, input, "x"),
        ["roots", input, name] => roots(context, input, name),
        ["rootsfield", input, name, field] => {
            let domain = match *field {
                "Q" => ScalarDomain::Rational,
                "R" => ScalarDomain::Real,
                "C" => ScalarDomain::Complex,
                _ => return Err(RequestError::Invalid("expected Q, R or C")),
            };
            let symbol = variable(context, name)?;
            let expression = source(input, domain)?;
            if !expression.conditions().is_empty() {
                return Err(RequestError::Unsupported(
                    "polynomial roots cannot discard source restrictions",
                ));
            }
            let polynomial = context
                .polynomial(expression.expression(), &symbol)
                .map_err(RequestError::Evaluation)?;
            Ok(ProblemRequest {
                operation: ProblemOperation::PolynomialEquation,
                statement: ProblemStatement::PolynomialZero(polynomial),
                variables: vec![symbol],
                domain,
            })
        }
        ["integrate", input] => expression_request(
            context,
            ProblemOperation::Integrate,
            input,
            "x",
            ScalarDomain::Real,
        ),
        ["integrate", input, name] => expression_request(
            context,
            ProblemOperation::Integrate,
            input,
            name,
            ScalarDomain::Real,
        ),
        ["diffreal", input, name] => expression_request(
            context,
            ProblemOperation::Differentiate,
            input,
            name,
            ScalarDomain::Real,
        ),
        ["diffexpr", input, name] => expression_request(
            context,
            ProblemOperation::Differentiate,
            input,
            name,
            ScalarDomain::Complex,
        ),
        ["limit", input, name, at] => limit(context, input, name, at, "two-sided"),
        ["limit", input, name, at, side] => limit(context, input, name, at, side),
        ["evalexact", input] => Ok(ProblemRequest {
            operation: ProblemOperation::EvaluateExact,
            statement: ProblemStatement::Expression(source(input, ScalarDomain::Real)?),
            variables: vec![],
            domain: ScalarDomain::Real,
        }),
        ["expandall", input, names @ ..] => {
            let expression = source(input, ScalarDomain::Real)?;
            context
                .check_expression(expression.expression())
                .map_err(RequestError::Evaluation)?;
            let variables = if names.is_empty() {
                let mut found = BTreeSet::new();
                collect_symbols(expression.expression(), &mut found);
                found.into_iter().collect()
            } else {
                if names.len() > 16 {
                    return Err(RequestError::Unsupported("too many ambient variables"));
                }
                let variables = names
                    .iter()
                    .map(|name| variable(context, name))
                    .collect::<Result<Vec<_>, _>>()?;
                if variables.iter().collect::<BTreeSet<_>>().len() != variables.len() {
                    return Err(RequestError::Invalid("variables must be distinct"));
                }
                variables
            };
            Ok(ProblemRequest {
                operation: ProblemOperation::Expand,
                statement: ProblemStatement::Expression(expression),
                variables,
                domain: ScalarDomain::Real,
            })
        }
        _ => Err(RequestError::Unsupported(
            "operation or argument count is not available in result schema v1",
        )),
    }
}

pub fn evaluate(context: &EvaluationContext, args: &[&str]) -> Result<ProblemResult, RequestError> {
    dispatch_problem(context, &request(context, args)?).map_err(RequestError::Problem)
}

pub fn evaluate_json(context: &EvaluationContext, args: &[&str]) -> Result<String, RequestError> {
    evaluate(context, args)?
        .to_json()
        .map_err(RequestError::Wire)
}
