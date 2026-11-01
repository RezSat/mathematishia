use mathema::{Expr, Integer, Rational, Symbol};
use mathema_frontend::{ParseErrorKind, Statement, parse_expression, parse_statement};

fn x() -> Expr {
    Expr::from(Symbol::new("x").unwrap())
}
fn rational(n: i64, d: i64) -> Expr {
    Rational::new(Integer::from(n), Integer::from(d))
        .unwrap()
        .into()
}

#[test]
fn parses_exact_big_integers_beyond_i128() {
    let huge = "340282366920938463463374607431768211456";
    let got = parse_expression(huge).unwrap();
    assert_eq!(got.to_string(), huge);
    assert_eq!(
        parse_expression("999999999999999999999999999999999999999999999").unwrap().to_string(),
        "999999999999999999999999999999999999999999999"
    );
}

#[test]
fn parses_and_reduces_rational_literals() {
    assert_eq!(parse_expression("6/8").unwrap(), rational(3, 4));
    assert_eq!(parse_expression("-6/8").unwrap(), rational(-3, 4));
    assert_eq!(parse_expression("1 / 2").unwrap(), rational(1, 2));
    assert_eq!(parse_expression("4/2").unwrap(), Expr::from(2));
    assert_eq!(parse_expression("0/5").unwrap(), Expr::from(0));
    assert_eq!(
        parse_expression("3/0").unwrap_err().kind(),
        &ParseErrorKind::ZeroDenominator
    );
}

#[test]
fn respects_precedence_and_right_associativity() {
    assert_eq!(
        parse_expression("1+2*3").unwrap(),
        Expr::from(7)
    );
    let powers = Expr::pow(x(), Expr::pow(Expr::from(2), Expr::from(3)));
    assert_eq!(parse_expression("x^2^3").unwrap(), powers);
    assert_eq!(
        parse_expression("-x^2").unwrap(),
        Expr::mul([(-1).into(), Expr::pow(x(), Expr::from(2))])
    );
    assert_eq!(
        parse_expression("(-x)^2").unwrap(),
        Expr::pow(Expr::mul([(-1).into(), x()]), Expr::from(2))
    );
    assert_eq!(
        parse_expression("x^-2").unwrap(),
        Expr::pow(x(), Expr::from(-2))
    );
}

#[test]
fn lowers_subtraction_and_unary_operators_through_semantics() {
    assert_eq!(parse_expression("5-3-1").unwrap(), Expr::from(1));
    assert_eq!(parse_expression("x--2").unwrap(), Expr::add([x(), 2.into()]));
    assert_eq!(parse_expression("+(+x)").unwrap(), x());
    assert_eq!(parse_expression("-(x+1)").unwrap(), Expr::mul([(-1).into(), Expr::add([x(), 1.into()])]));
}

#[test]
fn distinguishes_equations_from_expressions() {
    let Statement::Equation(eq) = parse_statement("2*x + 4 = 10").unwrap() else {
        panic!("expected equation");
    };
    assert_eq!(
        eq.left(),
        &Expr::add([Expr::mul([2.into(), x()]), 4.into()])
    );
    assert_eq!(eq.right(), &Expr::from(10));
    assert_eq!(
        parse_expression("x=2").unwrap_err().kind(),
        &ParseErrorKind::ExpectedExpression
    );
}

#[test]
fn accepts_unicode_identifier_without_normalizing_its_name() {
    let name = Symbol::new("π_α2").unwrap();
    assert_eq!(parse_expression("π_α2").unwrap(), Expr::from(name));
}

#[test]
fn rejects_implicit_multiplication_and_unsupported_division() {
    for invalid in ["2x", "2(x+1)", "x/2", "1/(2)", "1/0", "3.5", "x==1", "x=y=z"] {
        assert!(parse_statement(invalid).is_err(), "unexpectedly parsed {invalid}");
    }
}

#[test]
fn reports_utf8_byte_offsets_and_missing_parentheses() {
    let error = parse_expression("π + $").unwrap_err();
    assert_eq!(error.offset(), 5);
    assert_eq!(error.kind(), &ParseErrorKind::UnexpectedCharacter('$'));
    assert_eq!(
        parse_expression("(x+2").unwrap_err().kind(),
        &ParseErrorKind::ExpectedClosingParenthesis
    );
    assert_eq!(
        parse_expression(" ").unwrap_err().kind(),
        &ParseErrorKind::EmptyInput
    );
}

#[test]
fn bounded_input_and_nesting_report_errors() {
    let too_long = "1".repeat(32_769);
    assert_eq!(
        parse_expression(&too_long).unwrap_err().kind(),
        &ParseErrorKind::InputLimitExceeded
    );
    let nested = format!("{}x{}", "(".repeat(300), ")".repeat(300));
    assert_eq!(
        parse_expression(&nested).unwrap_err().kind(),
        &ParseErrorKind::NestingLimitExceeded
    );
    let many_tokens = "x+".repeat(4097);
    assert_eq!(
        parse_expression(&many_tokens).unwrap_err().kind(),
        &ParseErrorKind::TokenLimitExceeded
    );
}

#[test]
fn canonical_inputs_have_deterministic_semantic_values() {
    assert_eq!(parse_expression("x+1").unwrap(), parse_expression("1+x").unwrap());
    assert_eq!(parse_expression("x*(y+2)").unwrap(), parse_expression("(2+y)*x").unwrap());
}

#[test]
fn parentheses_do_not_turn_powers_into_numeric_evaluation() {
    assert_eq!(
        parse_expression("2^3").unwrap(),
        Expr::pow(2.into(), 3.into())
    );
}
