use mathema::{Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{PolynomialConversionError, RationalPolynomial};

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}
fn sym(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}
fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn poly(coefficients: &[(i64, i64)]) -> RationalPolynomial {
    RationalPolynomial::new(coefficients.iter().map(|(n, d)| q(*n, *d)).collect())
}
fn convert(expr: &Expr) -> Result<RationalPolynomial, PolynomialConversionError> {
    RationalPolynomial::try_from_expr(expr, &x(), 16)
}

#[test]
fn converts_exact_scalars_and_the_chosen_symbol() {
    assert_eq!(convert(&Expr::from(-2)).unwrap(), poly(&[(-2, 1)]));
    assert_eq!(convert(&Expr::from(q(2, 3))).unwrap(), poly(&[(2, 3)]));
    assert_eq!(convert(&sym("x")).unwrap(), poly(&[(0, 1), (1, 1)]));
}

#[test]
fn collects_terms_and_distributes_inside_the_specialized_domain() {
    let x = sym("x");
    let expression = Expr::add([x.clone(), x.clone(), Expr::from(3)]);
    assert_eq!(convert(&expression).unwrap(), poly(&[(3, 1), (2, 1)]));

    let lhs = Expr::add([x.clone(), 1.into()]);
    let rhs = Expr::add([x.clone(), (-1).into()]);
    let product = Expr::mul([lhs, rhs]);
    assert_eq!(convert(&product).unwrap(), poly(&[(-1, 1), (0, 1), (1, 1)]));

    let squared = Expr::pow(Expr::add([x, 1.into()]), 2.into());
    assert_eq!(convert(&squared).unwrap(), poly(&[(1, 1), (2, 1), (1, 1)]));
}

#[test]
fn exactly_converts_fractional_coefficients() {
    let expr = Expr::mul([
        Expr::from(q(1, 2)),
        Expr::add([sym("x"), Expr::from(q(1, 3))]),
    ]);
    assert_eq!(convert(&expr).unwrap(), poly(&[(1, 6), (1, 2)]));
}

#[test]
fn round_trips_polynomials_by_value_not_expression_structure() {
    for p in [
        poly(&[]),
        poly(&[(3, 4)]),
        poly(&[(0, 1), (2, 3)]),
        poly(&[(1, 2), (0, 1), (-5, 6), (1, 3)]),
    ] {
        let expr = p.to_expr(&x());
        assert_eq!(convert(&expr).unwrap(), p);
    }
}

#[test]
fn conversion_is_explicit_and_rejects_other_symbols() {
    assert_eq!(
        convert(&sym("y")),
        Err(PolynomialConversionError::OtherSymbol)
    );
    assert_eq!(
        convert(&Expr::mul([sym("x"), sym("y")])),
        Err(PolynomialConversionError::OtherSymbol)
    );
}

#[test]
fn rejects_ambiguous_and_non_polynomial_powers() {
    let x = sym("x");
    for expr in [
        Expr::pow(x.clone(), 0.into()),
        Expr::pow(Expr::from(0), 0.into()),
        Expr::pow(x.clone(), (-1).into()),
        Expr::pow(x.clone(), sym("y")),
        Expr::pow(x, Expr::from(q(1, 2))),
    ] {
        assert_eq!(
            convert(&expr),
            Err(PolynomialConversionError::UnsupportedPower)
        );
    }
}

#[test]
fn enforces_degree_and_exponent_limits_before_large_expansion() {
    let expression = Expr::pow(sym("x"), 4.into());
    assert_eq!(
        RationalPolynomial::try_from_expr(&expression, &x(), 3),
        Err(PolynomialConversionError::DegreeLimitExceeded)
    );
    assert_eq!(
        RationalPolynomial::try_from_expr(&sym("x"), &x(), 0),
        Err(PolynomialConversionError::DegreeLimitExceeded)
    );
    let huge = Expr::pow(sym("x"), Expr::from(u128::MAX));
    assert_eq!(
        RationalPolynomial::try_from_expr(&huge, &x(), 16),
        Err(PolynomialConversionError::DegreeLimitExceeded)
    );
    let product = Expr::mul([Expr::pow(sym("x"), 2.into()), Expr::pow(sym("x"), 2.into())]);
    assert_eq!(
        RationalPolynomial::try_from_expr(&product, &x(), 3),
        Err(PolynomialConversionError::DegreeLimitExceeded)
    );
}

#[test]
fn semantically_equivalent_shapes_produce_the_same_polynomial() {
    let a = Expr::pow(Expr::add([sym("x"), 1.into()]), 2.into());
    let b = Expr::add([
        Expr::pow(sym("x"), 2.into()),
        Expr::mul([2.into(), sym("x")]),
        1.into(),
    ]);
    assert_eq!(convert(&a), convert(&b));
}
