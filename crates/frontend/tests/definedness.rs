use mathema::{Condition, Expr, Symbol};
use mathema_frontend::{
    DomainParseError, DomainStatement, parse_domain_expression, parse_domain_statement,
    parse_expression,
};

fn sym(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

#[test]
fn preserves_reciprocal_holes_before_scalar_folding() {
    let x = sym("x");
    let parsed = parse_domain_expression("0*x^-1").unwrap();
    assert_eq!(parsed.expression(), &Expr::from(0));
    assert_eq!(parsed.conditions(), &[Condition::NonZero(x.clone())]);
    assert_eq!(parse_expression("0*x^-1").unwrap(), Expr::from(0));

    let canceled = parse_domain_expression("x^-1-x^-1").unwrap();
    assert_eq!(canceled.conditions(), &[Condition::NonZero(x)]);
}

#[test]
fn nested_exponents_preserve_the_exponents_own_restrictions() {
    let x = sym("x");
    let y = sym("y");
    let parsed = parse_domain_expression("y^(0*x^-1)").unwrap();
    assert_eq!(
        parsed.conditions(),
        &[Condition::NonZero(x), Condition::NonZero(y),]
    );
}

#[test]
fn equations_keep_both_domains() {
    let x = sym("x");
    let y = sym("y");
    let DomainStatement::Equation { left, right } = parse_domain_statement("0*x^-1=y^-2").unwrap()
    else {
        panic!("expected equation");
    };
    assert_eq!(left.expression(), &Expr::from(0));
    assert_eq!(left.conditions(), &[Condition::NonZero(x)]);
    assert_eq!(right.conditions(), &[Condition::NonZero(y)]);
}

#[test]
fn rejects_noninteger_exponents_in_domain_mode_only() {
    for input in ["x^y", "x^(1/2)", "x^2^3"] {
        assert!(matches!(
            parse_domain_expression(input),
            Err(DomainParseError::Domain(_))
        ));
        assert!(parse_expression(input).is_ok());
    }
    assert!(matches!(
        parse_domain_expression("x=1"),
        Err(DomainParseError::Syntax(_))
    ));
}

#[test]
fn signed_arithmetic_does_not_drop_restrictions() {
    let x = sym("x");
    for input in ["-(0*x^-1)", "0-(0*x^-1)", "0+(0*x^-1)"] {
        let parsed = parse_domain_expression(input).unwrap();
        assert_eq!(parsed.expression(), &Expr::from(0));
        assert_eq!(parsed.conditions(), &[Condition::NonZero(x.clone())]);
    }
}
