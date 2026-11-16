use mathema::{
    Condition, DomainExpression, Expr, ExprView, Integer, Rational, ScalarDomain, Symbol,
};
use mathematishia_kernel::{
    DomainMultivariatePolynomial, MultivariateError as M, MultivariateExprError as E,
    MultivariateLimits as L, MultivariatePolynomial as P,
};

fn s(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}
fn q(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}
fn frac(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn x() -> Expr {
    Expr::from(s("x"))
}
fn y() -> Expr {
    Expr::from(s("y"))
}
fn vars() -> Vec<Symbol> {
    vec![s("y"), s("x")]
}
fn convert(expr: &Expr) -> P {
    P::try_from_expr(expr, &vars(), L::default()).unwrap()
}
fn poly(terms: Vec<(Vec<i32>, Rational)>) -> P {
    P::try_new(vars(), terms, L::default()).unwrap()
}

#[test]
fn explicit_ambient_ring_zero_and_constants() {
    for ambient in [vec![], vec![s("z")], vars()] {
        for scalar in [q(0), q(5), frac(-7, 3)] {
            let expr = Expr::from(scalar.clone());
            let result = P::try_from_expr(&expr, &ambient, L::default()).unwrap();
            assert_eq!(result.variables(), {
                let mut sorted = ambient.clone();
                sorted.sort();
                sorted
            });
            assert_eq!(result.total_degree(), if scalar.numerator().is_zero() { None } else { Some(0) });
            assert_eq!(
                result,
                P::try_from_expr(&result.to_expr(L::default()).unwrap(), &ambient, L::default())
                    .unwrap()
            );
        }
    }
    assert_eq!(
        P::try_from_expr(&Expr::from(7), &[], L::default()).unwrap().coefficient(&[]),
        Ok(q(7))
    );
    assert_eq!(
        P::try_from_expr(&x(), &[], L::default()),
        Err(E::UndeclaredSymbol)
    );
    assert_eq!(
        P::try_from_expr(&Expr::from(0), &[s("x"), s("x")], L::default()),
        Err(E::Polynomial(M::DuplicateVariable))
    );
}

#[test]
fn canonical_variable_orders_and_nested_expansion() {
    let lhs = Expr::mul([
        Expr::add([x(), y()]),
        Expr::add([x(), Expr::from(-2)]),
    ]);
    let result = convert(&lhs);
    let same = P::try_from_expr(&lhs, &[s("x"), s("y")], L::default()).unwrap();
    assert_eq!(result, same);
    assert_eq!(result.coefficient(&[2, 0]), Ok(q(1)));
    assert_eq!(result.coefficient(&[1, 1]), Ok(q(1)));
    assert_eq!(result.coefficient(&[1, 0]), Ok(q(-2)));
    assert_eq!(result.coefficient(&[0, 1]), Ok(q(-2)));
    assert_eq!(result.total_degree(), Some(2));
    assert_eq!(result, convert(&result.to_expr(L::default()).unwrap()));

    let square = Expr::pow(Expr::add([x(), y()]), Expr::from(2));
    let squared = convert(&square);
    assert_eq!(squared.coefficient(&[2, 0]), Ok(q(1)));
    assert_eq!(squared.coefficient(&[1, 1]), Ok(q(2)));
    assert_eq!(squared.coefficient(&[0, 2]), Ok(q(1)));
    assert_eq!(squared, convert(&squared.to_expr(L::default()).unwrap()));
}

#[test]
fn cancellation_and_exact_mixed_rationals() {
    let f = Expr::add([
        Expr::mul([Expr::from(frac(3, 4)), x(), y()]),
        Expr::mul([Expr::from(frac(-3, 4)), y(), x()]),
        Expr::mul([Expr::from(frac(7, 6)), x()]),
        Expr::mul([Expr::from(frac(-1, 3)), x()]),
        Expr::from(frac(5, 8)),
    ]);
    let result = convert(&f);
    assert_eq!(result.coefficient(&[1, 1]), Ok(q(0)));
    assert_eq!(result.coefficient(&[1, 0]), Ok(frac(5, 6)));
    assert_eq!(result.coefficient(&[0, 0]), Ok(frac(5, 8)));
    assert_eq!(result.terms().len(), 2);
    assert_eq!(result, convert(&result.to_expr(L::default()).unwrap()));

    let cancelled = Expr::add([x(), Expr::mul([Expr::from(-1), x()])]);
    assert!(convert(&cancelled).is_zero());
    assert_eq!(convert(&cancelled).to_expr(L::default()).unwrap(), Expr::from(0));
}

#[test]
fn rejects_unsupported_powers_and_functions_without_approximation() {
    for (expr, failure) in [
        (Expr::pow(x(), Expr::from(0)), E::ZeroExponent),
        (Expr::pow(Expr::from(0), Expr::from(0)), E::ZeroExponent),
        (Expr::pow(x(), Expr::from(-1)), E::NegativeExponent),
        (Expr::pow(x(), Expr::from(frac(1, 2))), E::NonIntegerExponent),
        (Expr::pow(x(), y()), E::SymbolicExponent),
        (Expr::pow(x(), Expr::from(129)), E::ExponentLimit),
        (Expr::sin(x()), E::UnsupportedAnalyticFunction),
        (Expr::cos(x()), E::UnsupportedAnalyticFunction),
        (Expr::tan(x()), E::UnsupportedAnalyticFunction),
        (Expr::exp(x()), E::UnsupportedAnalyticFunction),
        (Expr::log(x()), E::UnsupportedAnalyticFunction),
        (
            Expr::function(s("custom"), Expr::from(0)),
            E::UnsupportedUserFunction,
        ),
        (Expr::add([x(), Expr::sin(Expr::from(0))]), E::UnsupportedAnalyticFunction),
    ] {
        assert_eq!(P::try_from_expr(&expr, &vars(), L::default()), Err(failure));
    }
    assert_eq!(
        P::try_from_expr(&Expr::from(s("z")), &vars(), L::default()),
        Err(E::UndeclaredSymbol)
    );
    let giant: Integer = "9".repeat(500).parse().unwrap();
    assert_eq!(
        P::try_from_expr(&Expr::pow(x(), Expr::from(giant)), &vars(), L::default()),
        Err(E::ExponentLimit)
    );
    let mut nested = x();
    for _ in 0..129 {
        nested = Expr::pow(nested, Expr::from(2));
    }
    assert_eq!(
        P::try_from_expr(&nested, &vars(), L::default()),
        Err(E::ExpressionDepthLimit)
    );
}

#[test]
fn conservative_limits_prevent_intermediate_overflow() {
    let mut limits = L::default();
    limits.max_variables = 1;
    assert_eq!(
        P::try_from_expr(&Expr::from(0), &vars(), limits),
        Err(E::Polynomial(M::VariableLimit))
    );
    limits = L::default();
    limits.max_scalar_bits = 1;
    assert_eq!(
        P::try_from_expr(&Expr::from(3), &[s("x")], limits),
        Err(E::Polynomial(M::ScalarBitLimit))
    );
    limits = L::default();
    limits.max_total_degree = 2;
    assert_eq!(
        P::try_from_expr(&Expr::pow(x(), Expr::from(3)), &[s("x")], limits),
        Err(E::ExponentLimit)
    );
    assert_eq!(
        P::try_from_expr(&Expr::mul([Expr::pow(x(), Expr::from(2)), y()]), &vars(), limits),
        Err(E::Polynomial(M::TotalDegreeLimit))
    );
    limits = L::default();
    limits.max_terms = 1;
    assert_eq!(
        P::try_from_expr(&Expr::add([x(), y()]), &vars(), limits),
        Err(E::Polynomial(M::TermLimit))
    );
    limits = L::default();
    limits.max_input_terms = 1;
    assert_eq!(
        P::try_from_expr(&Expr::add([x(), y()]), &vars(), limits),
        Err(E::Polynomial(M::InputTermLimit))
    );
    limits = L::default();
    limits.max_work_units = 1;
    assert_eq!(
        P::try_from_expr(&x(), &vars(), limits),
        Err(E::Polynomial(M::WorkLimit))
    );
    let p = poly(vec![(vec![1, 1], q(3))]);
    limits = L::default();
    limits.max_scalar_bits = 1;
    assert_eq!(p.to_expr(limits), Err(E::Polynomial(M::ScalarBitLimit)));
    limits = L::default();
    limits.max_work_units = 1;
    assert_eq!(p.to_expr(limits), Err(E::Polynomial(M::WorkLimit)));

    // Dense intermediate exceeds the bounded sparse representation; do not
    // silently return an incomplete expanded result.
    let eight = Expr::add((0..8).map(|i| Expr::from(s(&format!("v{i}")))));
    let expanded = Expr::pow(eight, Expr::from(5));
    let variables: Vec<_> = (0..8).map(|i| s(&format!("v{i}"))).collect();
    let refusal = P::try_from_expr(&expanded, &variables, L::default()).unwrap_err();
    assert!(matches!(
        refusal,
        E::Polynomial(M::InputTermLimit | M::TermLimit | M::WorkLimit)
    ));
}

#[test]
fn domain_entrypoint_keeps_history_that_bare_expr_cannot_recover() {
    let reciprocal = DomainExpression::from_expr(Expr::pow(x(), Expr::from(-1)), 32).unwrap();
    let zero = DomainExpression::from_expr(Expr::from(0), 32).unwrap();
    let historical = DomainExpression::multiply([zero, reciprocal], 16).unwrap();
    assert_eq!(historical.expression(), &Expr::from(0));
    assert_eq!(historical.conditions(), &[Condition::NonZero(x())]);
    let converted = P::try_from_domain_expression(&historical, &vars(), L::default()).unwrap();
    assert!(converted.polynomial().is_zero());
    assert_eq!(converted.conditions(), historical.conditions());
    let restored = converted.to_domain_expression(L::default()).unwrap();
    assert_eq!(restored.conditions(), historical.conditions());
    assert_eq!(restored.expression(), &Expr::from(0));

    let bare = P::try_from_expr(historical.expression(), &vars(), L::default()).unwrap();
    assert!(bare.is_zero()); // No claim that bare Expr retains past obligations.
}

#[test]
fn domain_real_and_complex_conditions_and_exhaustion() {
    let real_log = DomainExpression::from_expr_in_domain(
        Expr::log(x()),
        ScalarDomain::Real,
        100,
        10,
    )
    .unwrap();
    let complex_log = DomainExpression::from_expr_in_domain(
        Expr::log(x()),
        ScalarDomain::Complex,
        100,
        10,
    )
    .unwrap();
    let make_zero = |domain_call| {
        DomainExpression::multiply(
            [DomainExpression::from_expr(Expr::from(0), 10).unwrap(), domain_call],
            10,
        )
        .unwrap()
    };
    let real = make_zero(real_log);
    let complex = make_zero(complex_log);
    let saved_real = P::try_from_domain_expression(&real, &vars(), L::default()).unwrap();
    let saved_complex = P::try_from_domain_expression(&complex, &vars(), L::default()).unwrap();
    assert_eq!(saved_real.conditions(), &[Condition::Positive(x())]);
    assert_eq!(saved_complex.conditions(), &[Condition::NonZero(x())]);
    assert_ne!(saved_real.conditions(), saved_complex.conditions());
    assert_eq!(
        P::try_from_domain_expression(
            &DomainExpression::from_expr_in_domain(
                Expr::log(x()), ScalarDomain::Real, 100, 10
            ).unwrap(),
            &vars(),
            L::default(),
        ),
        Err(E::UnsupportedAnalyticFunction)
    );

    let many = DomainExpression::with_conditions(
        Expr::from(0),
        (0..129).map(|i| Condition::NonZero(Expr::from(s(&format!("v{i}"))))),
        100,
        256,
    )
    .unwrap();
    assert_eq!(
        P::try_from_domain_expression(&many, &vars(), L::default()),
        Err(E::ConditionLimit)
    );
}

fn eval_expr(expr: &Expr, x_val: &Rational, y_val: &Rational) -> Rational {
    match expr.view() {
        ExprView::Integer(n) => Rational::from(n.clone()),
        ExprView::Rational(r) => r.clone(),
        ExprView::Symbol(s) if s.name() == "x" => x_val.clone(),
        ExprView::Symbol(s) if s.name() == "y" => y_val.clone(),
        ExprView::Symbol(_) => panic!("unexpected variable"),
        ExprView::Add(xs) => xs.iter().fold(q(0), |acc, x| &acc + &eval_expr(x, x_val, y_val)),
        ExprView::Mul(xs) => xs.iter().fold(q(1), |acc, x| &acc * &eval_expr(x, x_val, y_val)),
        ExprView::Pow { base, exponent } => {
            let ExprView::Integer(n) = exponent.view() else {
                panic!("test generator uses only positive integer powers");
            };
            let power = n.try_i64().unwrap() as usize;
            let value = eval_expr(base, x_val, y_val);
            (0..power).fold(q(1), |acc, _| &acc * &value)
        }
        ExprView::Function { .. } => panic!("function is unsupported"),
    }
}
fn next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed
}

#[test]
fn seeded_independent_exact_evaluations_and_full_coefficient_roundtrips() {
    let mut seed = 0xa43e_231c_5837_1011_u64;
    for _ in 0..80 {
        let terms = (0..10)
            .map(|_| {
                let a = (next(&mut seed) % 4) as i32;
                let b = (next(&mut seed) % 4) as i32;
                let n = (next(&mut seed) % 11) as i64 - 5;
                let d = (next(&mut seed) % 5) as i64 + 1;
                (vec![a, b], frac(n, d))
            })
            .collect();
        let polynomial = poly(terms);
        let expr = polynomial.to_expr(L::default()).unwrap();
        assert_eq!(P::try_from_expr(&expr, &vars(), L::default()), Ok(polynomial.clone()));
        for a in -2..=2 {
            for b in -2..=2 {
                let independent = eval_expr(&expr, &q(a), &q(b));
                assert_eq!(
                    polynomial.evaluate(&[q(a), q(b)], L::default()),
                    Ok(independent)
                );
            }
        }
    }
}

#[test]
fn embedded_positive_powers_do_not_claim_general_simplification() {
    let p = convert(&Expr::pow(Expr::add([x(), Expr::from(1)]), Expr::from(8)));
    assert_eq!(p.coefficient(&[7, 0]), Ok(q(8)));
    assert_eq!(p.coefficient(&[6, 0]), Ok(q(28)));
    assert_eq!(p.coefficient(&[0, 0]), Ok(q(1)));
    assert_eq!(p.to_expr(L::default()).and_then(|expr| P::try_from_expr(&expr, &vars(), L::default())), Ok(p));
}
