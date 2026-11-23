use mathema::{Condition, DomainExpression, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    MultivariateError as M, MultivariateExprError as E, MultivariateLimits as L,
    PolynomialExpansionOutcome as Outcome, expand_polynomial, verify_polynomial_expansion,
};

fn s(value: &str) -> Symbol {
    Symbol::new(value).unwrap()
}
fn x() -> Expr {
    Expr::from(s("x"))
}
fn y() -> Expr {
    Expr::from(s("y"))
}
fn n(value: i32) -> Expr {
    Expr::from(value)
}
fn q(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}
fn input(value: Expr) -> DomainExpression {
    DomainExpression::from_expr(value, 2048).unwrap()
}
fn exact(
    source: &DomainExpression,
    variables: &[Symbol],
    limits: L,
) -> mathematishia_kernel::PolynomialExpansion {
    match expand_polynomial(source, variables, limits) {
        Outcome::Exact(result) => {
            assert!(result.complete());
            assert_eq!(result.expanded().conditions(), source.conditions());
            assert_eq!(
                verify_polynomial_expansion(source, &result, variables, limits),
                Ok(true)
            );
            result
        }
        other => panic!("expected certified exact expansion, got {other:?}"),
    }
}
fn coefficient(
    result: &mathematishia_kernel::PolynomialExpansion,
    powers: &[u32],
    numerator: i64,
    denominator: i64,
) {
    assert_eq!(
        result.polynomial().coefficient(powers),
        Ok(q(numerator, denominator))
    );
}

#[test]
fn expands_five_normal_school_algebra_questions_exactly() {
    let ring = [s("x"), s("y")];
    let square = input(Expr::pow(Expr::add([x(), n(1)]), n(2)));
    let r = exact(&square, &ring, L::default());
    coefficient(&r, &[2, 0], 1, 1);
    coefficient(&r, &[1, 0], 2, 1);
    coefficient(&r, &[0, 0], 1, 1);
    assert_eq!(r.polynomial().terms().len(), 3);
    assert!(!r.syntactically_unchanged());

    let twice = input(Expr::mul([n(2), Expr::add([x(), n(3)])]));
    let r = exact(&twice, &ring, L::default());
    coefficient(&r, &[1, 0], 2, 1);
    coefficient(&r, &[0, 0], 6, 1);

    let cube = input(Expr::pow(Expr::add([x(), Expr::mul([n(-1), y()])]), n(3)));
    let r = exact(&cube, &ring, L::default());
    coefficient(&r, &[3, 0], 1, 1);
    coefficient(&r, &[2, 1], -3, 1);
    coefficient(&r, &[1, 2], 3, 1);
    coefficient(&r, &[0, 3], -1, 1);

    let nested = input(Expr::mul([
        Expr::add([x(), n(2)]),
        Expr::add([x(), n(-2)]),
        Expr::add([y(), n(1)]),
    ]));
    let r = exact(&nested, &ring, L::default());
    coefficient(&r, &[2, 1], 1, 1);
    coefficient(&r, &[2, 0], 1, 1);
    coefficient(&r, &[0, 1], -4, 1);
    coefficient(&r, &[0, 0], -4, 1);

    let halves = input(Expr::pow(
        Expr::add([Expr::mul([Expr::from(q(1, 2)), x()]), Expr::from(q(1, 3))]),
        n(2),
    ));
    let r = exact(&halves, &ring, L::default());
    coefficient(&r, &[2, 0], 1, 4);
    coefficient(&r, &[1, 0], 1, 3);
    coefficient(&r, &[0, 0], 1, 9);
}

#[test]
fn preserves_zero_ambient_ring_and_original_exclusions() {
    let ring = [s("y"), s("x")];
    let historical = DomainExpression::multiply(
        [
            input(n(0)),
            DomainExpression::from_expr(Expr::pow(x(), n(-1)), 64).unwrap(),
        ],
        16,
    )
    .unwrap();
    let r = exact(&historical, &ring, L::default());
    assert!(r.polynomial().is_zero());
    assert_eq!(r.variables(), &[s("x"), s("y")]);
    assert_eq!(r.expanded().conditions(), &[Condition::NonZero(x())]);

    let scalar = exact(&input(Expr::from(q(-7, 3))), &[], L::default());
    coefficient(&scalar, &[], -7, 3);
    let constant_zero = exact(&input(n(0)), &[], L::default());
    assert!(constant_zero.polynomial().is_zero());
}

#[test]
fn structural_identity_is_not_confused_with_algebraic_equality() {
    let source = input(Expr::add([x(), n(2)]));
    let result = exact(&source, &[s("x")], L::default());
    // The expanded canonical AST may normalize to a different structural form.
    assert_eq!(
        result.syntactically_unchanged(),
        source.expression() == result.expanded().expression()
    );
    assert_eq!(
        verify_polynomial_expansion(
            &input(Expr::add([x(), n(3)])),
            &result,
            &[s("x")],
            L::default()
        ),
        Ok(false)
    );
    assert_eq!(
        verify_polynomial_expansion(&source, &result, &[s("x"), s("y")], L::default()),
        Ok(false)
    );
}

#[test]
fn unsupported_inputs_never_claim_completion() {
    let vars = [s("x"), s("y")];
    for (expression, expected) in [
        (Expr::pow(x(), n(0)), E::ZeroExponent),
        (Expr::pow(x(), n(-2)), E::NegativeExponent),
        (Expr::from(s("z")), E::UndeclaredSymbol),
    ] {
        assert_eq!(
            expand_polynomial(&input(expression), &vars, L::default()),
            Outcome::Unsupported(expected)
        );
    }
    for (expression, expected) in [
        (Expr::sin(x()), E::UnsupportedAnalyticFunction),
        (Expr::log(x()), E::UnsupportedAnalyticFunction),
    ] {
        let domain_input =
            DomainExpression::from_expr_in_domain(expression, mathema::ScalarDomain::Real, 128, 16)
                .unwrap();
        assert_eq!(
            expand_polynomial(&domain_input, &vars, L::default()),
            Outcome::Unsupported(expected)
        );
    }
    assert_eq!(
        expand_polynomial(&input(n(1)), &[s("x"), s("x")], L::default()),
        Outcome::Unsupported(E::Polynomial(M::DuplicateVariable))
    );
}

#[test]
fn resource_refusals_are_typed_and_never_partial_success() {
    let vars = [s("x"), s("y")];
    let square = input(Expr::pow(Expr::add([x(), y()]), n(2)));
    let degree = L {
        max_total_degree: 1,
        ..L::default()
    };
    assert!(matches!(
        expand_polynomial(&square, &vars, degree),
        Outcome::ResourceLimited(E::ExponentLimit | E::Polynomial(M::TotalDegreeLimit))
    ));
    let work = L {
        max_work_units: 1,
        ..L::default()
    };
    assert_eq!(
        expand_polynomial(&square, &vars, work),
        Outcome::ResourceLimited(E::Polynomial(M::WorkLimit))
    );
    let terms = L {
        max_terms: 1,
        ..L::default()
    };
    assert!(matches!(
        expand_polynomial(&square, &vars, terms),
        Outcome::ResourceLimited(E::Polynomial(M::TermLimit | M::InputTermLimit))
    ));
    let variables = (0..17)
        .map(|index| s(&format!("v{index}")))
        .collect::<Vec<_>>();
    assert_eq!(
        expand_polynomial(&input(n(1)), &variables, L::default()),
        Outcome::ResourceLimited(E::Polynomial(M::VariableLimit))
    );
}

#[test]
fn repeatability_and_wrong_certificate_source_are_checked() {
    let ring = [s("y"), s("x")];
    let source = input(Expr::pow(Expr::add([x(), y(), n(1)]), n(4)));
    let first = exact(&source, &ring, L::default());
    let second = exact(&source, &ring, L::default());
    assert_eq!(first, second);
    assert_eq!(first.polynomial().terms().len(), 15);
    assert_eq!(
        verify_polynomial_expansion(&input(n(0)), &first, &ring, L::default()),
        Ok(false)
    );
}
