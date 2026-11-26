//! Opt-in natural input v1. This adapter does not change the legacy parser.
//!
//! Angles: sin/cos/tan(signed integer coefficient times pi or π, optionally
//! divided by a positive integer). Multiplication before pi may be implicit.
//! Equations: one outer abs(...) or |...| per side, with total rational-affine
//! operands in one named real variable. At least one side must contain abs.
//! Nested bars, general function composition and radicals are not admitted.

use mathema::{DefinednessError, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    AbsoluteAffine, AbsoluteEquation, AbsoluteEquationError, AbsoluteEquationResult,
    AbsoluteExpression, EvaluationContext, EvaluationError, PiMultiple, SpecialAngleCertificate,
    TrigError, TrigFunction, TrigLimits, evaluate_special_angle, solve_absolute_equation,
    verify_absolute_equation, verify_special_angle,
};

use crate::advanced::AdvancedError;
use crate::problem::{self, RequestError};
use crate::{DomainParseError, TokenKind, tokenize};

const MAX_BYTES: usize = 4096;
const MAX_TOKENS: usize = 128;
const MAX_LITERAL_DIGITS: usize = 64;

/// A frontend-local request, ready for the checked real-domain kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NaturalRequest {
    SpecialAngle {
        function: TrigFunction,
        angle: PiMultiple,
    },
    AbsoluteEquation(Box<AbsoluteEquation>),
}

/// An exact checked outcome. A tangent pole remains explicitly undefined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NaturalOutcome {
    SpecialAngle(Box<SpecialAngleCertificate>),
    AbsoluteEquation(AbsoluteEquationResult),
}

fn invalid(message: &'static str) -> AdvancedError {
    AdvancedError::Input(RequestError::Invalid(message))
}

fn unsupported(message: &'static str) -> AdvancedError {
    AdvancedError::Unsupported(message.into())
}

fn domain(message: &'static str) -> AdvancedError {
    AdvancedError::Domain(message.into())
}

fn resource() -> AdvancedError {
    AdvancedError::Resource("natural v1 input limit exceeded".into())
}

fn evaluation(error: EvaluationError) -> AdvancedError {
    match error {
        EvaluationError::Limit(_) => AdvancedError::Resource(error.to_string()),
        EvaluationError::DivisionByZero => domain("division by zero"),
        _ => AdvancedError::Unsupported(error.to_string()),
    }
}

fn request_error(error: RequestError) -> AdvancedError {
    match error {
        RequestError::Evaluation(error) => evaluation(error),
        RequestError::Parse(DomainParseError::Domain(DefinednessError::ZeroDenominator)) => {
            domain("division by zero")
        }
        RequestError::Parse(DomainParseError::Domain(
            DefinednessError::NodeLimitExceeded | DefinednessError::ConditionLimitExceeded,
        )) => resource(),
        RequestError::Unsupported(message) => unsupported(message),
        _ => AdvancedError::Input(error),
    }
}

fn tokens(input: &str) -> Result<Vec<TokenKind<'_>>, AdvancedError> {
    let tokens = tokenize(input).map_err(|error| RequestError::Parse(error.into()))?;
    if tokens.len() > MAX_TOKENS
        || tokens.iter().any(|token| match token.kind {
            TokenKind::Integer(digits) | TokenKind::Decimal(digits) => {
                digits.len() > MAX_LITERAL_DIGITS
            }
            _ => false,
        })
    {
        return Err(resource());
    }
    Ok(tokens
        .into_iter()
        .filter_map(|token| (token.kind != TokenKind::End).then_some(token.kind))
        .collect())
}

fn integer(digits: &str) -> Result<Integer, AdvancedError> {
    digits.parse().map_err(|_| invalid("invalid angle integer"))
}

fn angle(context: &EvaluationContext, argument: &str) -> Result<PiMultiple, AdvancedError> {
    use TokenKind::{Identifier, Integer as Digits, Minus, Plus, Slash, Star};

    let tokens = tokens(argument)?;
    let (negative, rest) = match tokens.as_slice() {
        [Minus, rest @ ..] => (true, rest),
        [Plus, rest @ ..] => (false, rest),
        rest => (false, rest),
    };
    let (numerator, rest) = match rest {
        [Digits(digits), Star, rest @ ..] | [Digits(digits), rest @ ..] => (*digits, rest),
        rest => ("1", rest),
    };
    let denominator = match rest {
        [Identifier("pi" | "π")] => "1",
        [Identifier("pi" | "π"), Slash, Digits(digits)] => *digits,
        _ => return Err(unsupported("expected a signed integer multiple of pi[/integer]")),
    };
    let numerator = integer(numerator)?;
    let numerator = if negative { -numerator } else { numerator };
    let coefficient = Rational::new(numerator, integer(denominator)?)
        .map_err(|_| domain("angle denominator must not be zero"))?;
    context.check_rational(&coefficient).map_err(evaluation)?;
    Ok(PiMultiple::new(coefficient))
}

fn affine(
    context: &EvaluationContext,
    source: &str,
    variable: &Symbol,
) -> Result<AbsoluteAffine, AdvancedError> {
    let tokens = tokens(source)?;
    for token in &tokens {
        match token {
            TokenKind::Identifier(name) if *name == variable.name() => {}
            TokenKind::Identifier(_) | TokenKind::Caret => {
                return Err(unsupported("expected rational-affine input in the named variable"));
            }
            _ => {}
        }
    }
    if tokens.windows(3).any(|window| {
        matches!(
            window,
            [
                TokenKind::Slash,
                TokenKind::Integer(_) | TokenKind::Decimal(_),
                TokenKind::Identifier(_) | TokenKind::LeftParen
            ]
        )
    }) {
        return Err(invalid("ambiguous implicit multiplication after division"));
    }
    let parsed = problem::source(source, ScalarDomain::Real).map_err(request_error)?;
    if !parsed.conditions().is_empty() {
        return Err(domain("absolute equations cannot discard source restrictions"));
    }
    let polynomial = context
        .polynomial(parsed.expression(), variable)
        .map_err(evaluation)?;
    if polynomial.degree().is_some_and(|degree| degree > 1) {
        return Err(unsupported("absolute-value operands must be affine"));
    }
    let coefficient = |index| {
        polynomial
            .coefficients()
            .get(index)
            .cloned()
            .unwrap_or_else(|| Rational::from(Integer::zero()))
    };
    Ok(AbsoluteAffine {
        slope: coefficient(1),
        constant: coefficient(0),
    })
}

fn side(
    context: &EvaluationContext,
    source: &str,
    variable: &Symbol,
) -> Result<AbsoluteExpression, AdvancedError> {
    let source = source.trim();
    let (operand, absolute) = if let Some(rest) = source.strip_prefix('|') {
        (
            rest.strip_suffix('|')
                .ok_or_else(|| invalid("unclosed absolute-value bar"))?,
            true,
        )
    } else if let Some(rest) = source.strip_prefix("abs") {
        let rest = rest.trim_start();
        (
            rest.strip_prefix('(')
                .and_then(|inner| inner.strip_suffix(')'))
                .ok_or_else(|| invalid("expected abs(...)"))?,
            true,
        )
    } else {
        (source, false)
    };
    if operand.contains('|') {
        return Err(invalid("nested or unmatched absolute-value bars"));
    }
    let base = affine(context, operand, variable)?;
    let layers = if absolute {
        vec![AbsoluteAffine {
            slope: Rational::from(Integer::from(1)),
            constant: Rational::from(Integer::zero()),
        }]
    } else {
        Vec::new()
    };
    Ok(AbsoluteExpression {
        variable: variable.clone(),
        base,
        layers,
    })
}

/// Parse only natural grammar v1; never fall back to legacy symbol semantics.
/// The variable argument is used only for equations.
pub fn parse_v1(
    context: &EvaluationContext,
    source: &str,
    variable: &str,
) -> Result<NaturalRequest, AdvancedError> {
    if source.len() > MAX_BYTES || variable.len() > MAX_LITERAL_DIGITS {
        return Err(resource());
    }
    let source = source.trim();
    if let Some((left, right)) = source.split_once('=') {
        if right.contains('=') {
            return Err(invalid("expected exactly one equality"));
        }
        let variable = problem::variable(context, variable).map_err(request_error)?;
        if matches!(variable.name(), "pi" | "π" | "abs" | "sqrt" | "sin" | "cos" | "tan") {
            return Err(invalid("reserved natural v1 variable"));
        }
        let left = side(context, left, &variable)?;
        let right = side(context, right, &variable)?;
        if left.layers.is_empty() && right.layers.is_empty() {
            return Err(unsupported("expected at least one absolute-value operand"));
        }
        return Ok(NaturalRequest::AbsoluteEquation(Box::new(AbsoluteEquation {
            domain: ScalarDomain::Real,
            left,
            right,
        })));
    }
    let (name, argument) = source
        .split_once('(')
        .ok_or_else(|| invalid("expected a natural v1 call or equation"))?;
    let argument = argument
        .strip_suffix(')')
        .ok_or_else(|| invalid("expected a closing parenthesis"))?;
    let function = match name.trim() {
        "sin" => TrigFunction::Sin,
        "cos" => TrigFunction::Cos,
        "tan" => TrigFunction::Tan,
        "sqrt" => {
            let kinds = tokens(argument)?;
            if kinds
                .iter()
                .any(|kind| matches!(kind, TokenKind::Identifier(_) | TokenKind::Caret))
            {
                return Err(unsupported("square roots require a checked rational argument"));
            }
            let value = problem::scalar(context, argument).map_err(request_error)?;
            if value.numerator() < &Integer::zero() {
                return Err(domain("negative square root is undefined over the reals"));
            }
            return Err(unsupported("principal rational square root dispatch is not admitted"));
        }
        _ => return Err(unsupported("expected sin, cos, tan or an absolute equation")),
    };
    Ok(NaturalRequest::SpecialAngle {
        function,
        angle: angle(context, argument)?,
    })
}

fn trig_error(error: TrigError) -> AdvancedError {
    match error {
        TrigError::ScalarLimit | TrigError::InvalidLimits => {
            AdvancedError::Resource(error.to_string())
        }
        TrigError::InvalidEvidence => AdvancedError::Verification(error.to_string()),
        _ => AdvancedError::Unsupported(error.to_string()),
    }
}

fn absolute_error(error: AbsoluteEquationError) -> AdvancedError {
    match error {
        AbsoluteEquationError::Evaluation(error) => evaluation(error),
        AbsoluteEquationError::CaseLimit => AdvancedError::Resource(error.to_string()),
        AbsoluteEquationError::VerificationFailed => AdvancedError::Verification(error.to_string()),
        _ => AdvancedError::Unsupported(error.to_string()),
    }
}

/// Evaluate an admitted request through existing independently checked kernels.
pub fn evaluate_v1(
    context: &EvaluationContext,
    request: &NaturalRequest,
) -> Result<NaturalOutcome, AdvancedError> {
    match request {
        NaturalRequest::SpecialAngle { function, angle } => {
            context.check_rational(angle.coefficient()).map_err(evaluation)?;
            let limits = TrigLimits::default();
            let domain = ScalarDomain::Real;
            let result =
                evaluate_special_angle(angle, *function, domain, limits).map_err(trig_error)?;
            if !verify_special_angle(angle, *function, domain, &result, limits).map_err(trig_error)? {
                return Err(AdvancedError::Verification(
                    "special angle evidence failed verification".into(),
                ));
            }
            Ok(NaturalOutcome::SpecialAngle(Box::new(result)))
        }
        NaturalRequest::AbsoluteEquation(equation) => {
            let result = solve_absolute_equation(context, equation).map_err(absolute_error)?;
            if !verify_absolute_equation(context, equation, &result).map_err(absolute_error)? {
                return Err(AdvancedError::Verification(
                    "absolute equation evidence failed verification".into(),
                ));
            }
            Ok(NaturalOutcome::AbsoluteEquation(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathema::ExprView;
    use mathematishia_kernel::{ExactQuadraticRoot, ResourceLimits, SpecialAngleOutcome};

    fn context() -> EvaluationContext {
        EvaluationContext::new(ResourceLimits::default())
    }

    fn ratio(n: i32, d: i32) -> Rational {
        Rational::new(Integer::from(n), Integer::from(d)).unwrap()
    }

    #[test]
    fn exact_rational_angles() {
        let context = context();
        for (source, expected) in [
            ("sin(pi/6)", ratio(1, 2)),
            ("cos(-2π/3)", ratio(-1, 2)),
            ("tan(5*pi/4)", ratio(1, 1)),
            ("sin(+pi)", ratio(0, 1)),
            ("cos(4pi)", ratio(1, 1)),
        ] {
            let request = parse_v1(&context, source, "x").unwrap();
            let NaturalOutcome::SpecialAngle(result) = evaluate_v1(&context, &request).unwrap()
            else {
                panic!("expected checked angle");
            };
            assert_eq!(
                result.outcome,
                SpecialAngleOutcome::Exact(ExactQuadraticRoot::Rational(expected))
            );
        }
    }

    #[test]
    fn tangent_pole_is_not_a_value() {
        let context = context();
        let request = parse_v1(&context, "tan(pi/2)", "x").unwrap();
        let NaturalOutcome::SpecialAngle(result) = evaluate_v1(&context, &request).unwrap() else {
            panic!("expected checked angle");
        };
        assert_eq!(result.outcome, SpecialAngleOutcome::UndefinedTangentPole);
    }

    #[test]
    fn absolute_equations_preserve_exact_solutions() {
        let context = context();
        for source in ["|2x-5|=7", "abs(2x-5)=7", "7=abs (2*x-5)"] {
            let request = parse_v1(&context, source, "x").unwrap();
            let NaturalOutcome::AbsoluteEquation(result) = evaluate_v1(&context, &request).unwrap()
            else {
                panic!("expected checked absolute equation");
            };
            assert_eq!(result.solutions.len(), 2);
            for (interval, expected) in result.solutions.iter().zip([ratio(-1, 1), ratio(6, 1)]) {
                assert_eq!(interval.lower.as_ref(), Some(&expected));
                assert_eq!(interval.upper.as_ref(), Some(&expected));
            }
        }
    }

    #[test]
    fn absolute_real_intervals_and_empty_sets() {
        let context = context();
        for (source, count) in [("|x|=abs(x)", 1), ("|x|=-1", 0)] {
            let request = parse_v1(&context, source, "x").unwrap();
            let NaturalOutcome::AbsoluteEquation(result) = evaluate_v1(&context, &request).unwrap()
            else {
                panic!("expected checked absolute equation");
            };
            assert_eq!(result.solutions.len(), count);
            if count == 1 {
                assert_eq!(result.solutions[0].lower, None);
                assert_eq!(result.solutions[0].upper, None);
            }
        }
    }

    #[test]
    fn legacy_pi_is_still_a_symbol() {
        let expression = crate::parse_expression("pi").unwrap();
        assert!(matches!(expression.view(), ExprView::Symbol(s) if s.name() == "pi"));
        assert!(parse_v1(&context(), "pi", "x").is_err());
    }

    #[test]
    fn refuses_ambiguous_and_unsupported_syntax() {
        let context = context();
        for source in [
            "|x=1",
            "||x||=1",
            "|x|+|x|=1",
            "abs(abs(x))=1",
            "|x^2|=1",
            "|1/2x|=1",
            "|x+0*y|=1",
            "|x|=1=2",
            "sin(pizza/6)",
            "sin(pi*pi)",
            "sin(1/2pi)",
            "sin(pi/6)+1",
        ] {
            assert!(parse_v1(&context, source, "x").is_err(), "{source}");
        }
    }

    #[test]
    fn source_restrictions_and_zero_denominators_are_not_erased() {
        let context = context();
        for source in ["|0*(1/x)|=0", "|x/x|=1", "|1/0|=1", "sin(pi/0)"] {
            assert!(
                matches!(parse_v1(&context, source, "x"), Err(AdvancedError::Domain(_))),
                "{source}"
            );
        }
    }

    #[test]
    fn radical_dispatch_is_gated_and_negative_radicand_is_undefined() {
        let context = context();
        for source in ["sqrt(16)", "sqrt(2)", "sqrt(x)"] {
            assert!(matches!(
                parse_v1(&context, source, "x"),
                Err(AdvancedError::Unsupported(_))
            ));
        }
        assert!(matches!(
            parse_v1(&context, "sqrt(-1)", "x"),
            Err(AdvancedError::Domain(_))
        ));
    }

    #[test]
    fn unsupported_table_angle_and_resource_limits_are_explicit() {
        let context = context();
        let request = parse_v1(&context, "sin(pi/7)", "x").unwrap();
        assert!(matches!(
            evaluate_v1(&context, &request),
            Err(AdvancedError::Unsupported(_))
        ));
        let oversized = format!("sin({}pi)", "9".repeat(65));
        assert!(matches!(
            parse_v1(&context, &oversized, "x"),
            Err(AdvancedError::Resource(_))
        ));
        assert!(matches!(
            parse_v1(&context, &" ".repeat(MAX_BYTES + 1), "x"),
            Err(AdvancedError::Resource(_))
        ));
    }
}
