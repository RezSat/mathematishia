use mathema::{Condition, Equation, Expr, Integer, Rational, Symbol};
use mathema_frontend::{
    DomainParseError, DomainStatement, ParseErrorKind, Statement, parse_domain_expression,
    parse_domain_expression_in_domain, parse_domain_statement, parse_expression, parse_statement,
};

fn symbol(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

fn fraction(numerator: i64, denominator: i64) -> Expr {
    Rational::new(Integer::from(numerator), Integer::from(denominator))
        .unwrap()
        .into()
}

#[test]
fn school_decimals_are_exact_rationals() {
    let cases = [
        ("0.25 + 0.5", fraction(3, 4)),
        (".5 + .25", fraction(3, 4)),
        ("3.50 - 1.25", fraction(9, 4)),
        ("2.0 * 4", Expr::from(8)),
        ("-0.125 * 8", Expr::from(-1)),
        ("000.0500", fraction(1, 20)),
    ];
    for (input, expected) in cases {
        assert_eq!(parse_expression(input).unwrap(), expected, "{input}");
        assert_eq!(
            parse_domain_expression(input).unwrap().expression(),
            &expected,
            "{input}"
        );
    }
    let tiny = format!("0.{}1", "0".repeat(200));
    let expected = Rational::new(
        Integer::one(),
        format!("1{}", "0".repeat(201)).parse().unwrap(),
    )
    .unwrap();
    assert_eq!(parse_expression(&tiny).unwrap(), Expr::from(expected));
}

#[test]
fn implicit_products_preserve_semantic_precedence() {
    let x = symbol("x");
    let x_plus_one = Expr::add([x.clone(), 1.into()]);
    let x_minus_one = Expr::add([x.clone(), (-1).into()]);
    let cases = [
        (
            "2x+1",
            Expr::add([Expr::mul([2.into(), x.clone()]), 1.into()]),
        ),
        ("2(x+1)", Expr::mul([2.into(), x_plus_one.clone()])),
        (
            "(x+1)(x-1)",
            Expr::mul([x_plus_one.clone(), x_minus_one.clone()]),
        ),
        (
            "2 (x+1)(x-1)",
            Expr::mul([2.into(), x_plus_one.clone(), x_minus_one.clone()]),
        ),
        (
            "2x^2",
            Expr::mul([2.into(), Expr::pow(x.clone(), 2.into())]),
        ),
        ("1/2x", Expr::mul([fraction(1, 2), x.clone()])),
        (
            "-.25(x+1)",
            Expr::mul([fraction(-1, 4), x_plus_one.clone()]),
        ),
        ("2sin(x)", Expr::mul([2.into(), Expr::sin(x.clone())])),
    ];
    for (input, expected) in cases {
        assert_eq!(parse_expression(input).unwrap(), expected, "{input}");
        assert_eq!(
            parse_domain_expression(input).unwrap().expression(),
            &expected,
            "{input}"
        );
    }
    assert_eq!(
        parse_expression("2π_α2").unwrap(),
        Expr::mul([2.into(), symbol("π_α2")])
    );
}

#[test]
fn equation_semantics_and_domains_are_preserved() {
    let x = symbol("x");
    assert_eq!(
        parse_statement("2x+0.5=1.25"),
        Ok(Statement::Equation(Equation::new(
            Expr::add([Expr::mul([2.into(), x.clone()]), fraction(1, 2)]),
            fraction(5, 4),
        )))
    );
    let DomainStatement::Equation { left, right } =
        parse_domain_statement("0.25/(x-.5)=2(x+1)").unwrap()
    else {
        panic!("expected domain-aware equation");
    };
    assert_eq!(
        left.conditions(),
        &[Condition::NonZero(Expr::add([x.clone(), fraction(-1, 2)]))]
    );
    assert!(right.conditions().is_empty());
    assert_eq!(
        left.expression(),
        &Expr::mul([
            fraction(1, 4),
            Expr::pow(Expr::add([x.clone(), fraction(-1, 2)]), (-1).into())
        ])
    );
    assert_eq!(
        right.expression(),
        &Expr::mul([2.into(), Expr::add([x, 1.into()])])
    );
}

#[test]
fn implicit_products_do_not_erase_excluded_values() {
    let x = symbol("x");
    let product = parse_domain_expression("0(x^-1)").unwrap();
    assert_eq!(product.expression(), &Expr::from(0));
    assert_eq!(product.conditions(), &[Condition::NonZero(x.clone())]);

    let fraction_product = parse_domain_expression("0.25(1/x)").unwrap();
    assert_eq!(
        fraction_product.expression(),
        &Expr::mul([fraction(1, 4), Expr::pow(x.clone(), (-1).into())])
    );
    assert_eq!(
        fraction_product.conditions(),
        &[Condition::NonZero(x.clone())]
    );
    let real =
        parse_domain_expression_in_domain("2tan(log(x))", mathema::ScalarDomain::Real).unwrap();
    assert_eq!(
        real.conditions(),
        &[
            Condition::Positive(x.clone()),
            Condition::NonZero(Expr::cos(Expr::log(x))),
        ]
    );
}

#[test]
fn malformed_decimals_report_precise_byte_offsets() {
    for (input, offset) in [(".", 0), ("1.", 1), ("1..2", 1), ("x + .", 4), ("0.25.", 4)] {
        let error = parse_expression(input).unwrap_err();
        assert_eq!(error.kind(), &ParseErrorKind::InvalidDecimal, "{input}");
        assert_eq!(error.offset(), offset, "{input}");
    }
    for (input, offset) in [("2e3", 1), ("1.25E-3", 4)] {
        let error = parse_expression(input).unwrap_err();
        assert_eq!(
            error.kind(),
            &ParseErrorKind::UnsupportedScientificNotation,
            "{input}"
        );
        assert_eq!(error.offset(), offset, "{input}");
    }
    let error = parse_expression("π + 2.").unwrap_err();
    assert_eq!(error.offset(), 6);
    assert_eq!(error.kind(), &ParseErrorKind::InvalidDecimal);
}

#[test]
fn unsupported_ambiguities_and_functions_are_not_reinterpreted() {
    for (input, kind, offset) in [
        ("x(y+1)", ParseErrorKind::UnknownFunction("x".into()), 0),
        ("sqrt(2)", ParseErrorKind::UnknownFunction("sqrt".into()), 0),
        ("abs(-2)", ParseErrorKind::UnknownFunction("abs".into()), 0),
        ("2 3", ParseErrorKind::UnexpectedToken, 2),
        ("(x+1)y", ParseErrorKind::UnexpectedToken, 5),
        ("x^2-4>0", ParseErrorKind::UnexpectedCharacter('>'), 5),
        ("x^2-4<=0", ParseErrorKind::UnexpectedCharacter('<'), 5),
        ("x==1", ParseErrorKind::UnexpectedToken, 2),
    ] {
        let error = parse_statement(input).unwrap_err();
        assert_eq!(error.kind(), &kind, "{input}");
        assert_eq!(error.offset(), offset, "{input}");
    }
    assert!(matches!(
        parse_domain_expression("2x=1"),
        Err(DomainParseError::Syntax(_))
    ));
    assert_eq!(
        parse_expression("0.25/0.0").unwrap_err().kind(),
        &ParseErrorKind::ZeroDenominator
    );
    assert_eq!(
        parse_domain_expression("0.25/0.0").unwrap_err().to_string(),
        "division by zero"
    );
}

#[test]
fn resource_limits_apply_to_decimal_literals_and_implicit_products() {
    let max_decimal = format!("0.{}", "1".repeat(255));
    assert!(parse_expression(&max_decimal).is_ok());
    for input in [
        format!("0.{}", "1".repeat(256)),
        format!("{}.1", "1".repeat(256)),
    ] {
        let error = parse_expression(&input).unwrap_err();
        assert_eq!(error.kind(), &ParseErrorKind::DecimalDigitLimitExceeded);
        assert_eq!(error.offset(), 0);
    }
    assert_eq!(
        parse_expression(&"1".repeat(32_769)).unwrap_err().kind(),
        &ParseErrorKind::InputLimitExceeded
    );
    assert_eq!(
        parse_expression(&"2 x ".repeat(2_100)).unwrap_err().kind(),
        &ParseErrorKind::TokenLimitExceeded
    );
    assert_eq!(
        parse_expression(&format!("2{}x{}", "(".repeat(300), ")".repeat(300)))
            .unwrap_err()
            .kind(),
        &ParseErrorKind::NestingLimitExceeded
    );
}

#[test]
fn equivalent_implicit_and_explicit_inputs_normalize_deterministically() {
    for (implicit, explicit) in [
        ("2x+1", "2*x+1"),
        ("2(x+1)", "2*(x+1)"),
        ("(x+1)(x-1)", "(x+1)*(x-1)"),
        ("2sin(x)", "2*sin(x)"),
        ("0.25 + 0.5", "1/4 + 1/2"),
        ("-.125x", "-(1/8)*x"),
        ("2/3x", "(2/3)*x"),
    ] {
        assert_eq!(
            parse_expression(implicit).unwrap(),
            parse_expression(explicit).unwrap(),
            "{implicit}"
        );
        assert_eq!(
            parse_domain_expression(implicit).unwrap(),
            parse_domain_expression(explicit).unwrap(),
            "{implicit}"
        );
    }
}
