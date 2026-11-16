use mathema::{Equation, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    EvaluationContext, ExpressionLinearSystemSet as Set,
    ExpressionLinearSystemSolution as Solution, LinearSystemExprError as Error, ResourceLimitKind,
    ResourceLimits, solve_linear_system_expr, verify_linear_system_expr,
};

fn s(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}
fn v(name: &str) -> Expr {
    Expr::from(s(name))
}
fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn eq(a: Expr, b: Expr) -> Equation {
    Equation::new(a, b)
}
fn mul(n: i64, e: Expr) -> Expr {
    Expr::mul([Expr::from(n), e])
}
fn solve(e: &[Equation], vars: &[Symbol]) -> Solution {
    solve_linear_system_expr(&EvaluationContext::default(), e, vars).unwrap()
}

#[test]
fn rational_unique_overdetermined_and_symbol_order() {
    let e = [
        eq(Expr::add([mul(2, v("x")), v("y")]), 1.into()),
        eq(Expr::add([v("x"), mul(-1, v("y"))]), 0.into()),
        eq(Expr::add([mul(4, v("x")), mul(2, v("y"))]), 2.into()),
    ];
    let vars = [s("x"), s("y")];
    assert_eq!(solve(&e, &vars).set(), &Set::Unique(vec![q(1, 3), q(1, 3)]));
    assert_eq!(
        solve(&e, &[s("y"), s("x")]).set(),
        &Set::Unique(vec![q(1, 3), q(1, 3)])
    );
    let e2 = [
        eq(Expr::add([v("x"), v("y")]), 5.into()),
        eq(Expr::add([v("x"), mul(-1, v("y"))]), 1.into()),
    ];
    assert_eq!(
        solve(&e2, &vars).set(),
        &Set::Unique(vec![q(3, 1), q(2, 1)])
    );
    assert_eq!(
        solve(&e2, &[s("y"), s("x")]).set(),
        &Set::Unique(vec![q(2, 1), q(3, 1)])
    );
}

#[test]
fn dependent_affine_and_inconsistent_cases() {
    let vars = [s("z"), s("x"), s("y")];
    let sum = Expr::add([v("x"), v("y"), v("z")]);
    let e = [eq(sum.clone(), 3.into()), eq(mul(2, sum.clone()), 6.into())];
    let result = solve(&e, &vars);
    assert_eq!(
        result.set(),
        &Set::Affine {
            particular: vec![q(3, 1), q(0, 1), q(0, 1)],
            basis: vec![
                vec![q(-1, 1), q(1, 1), q(0, 1)],
                vec![q(-1, 1), q(0, 1), q(1, 1)],
            ],
        }
    );
    assert!(verify_linear_system_expr(&EvaluationContext::default(), &e, &vars, &result).unwrap());
    let bad = [eq(sum.clone(), 3.into()), eq(mul(2, sum), 7.into())];
    assert_eq!(solve(&bad, &vars).set(), &Set::Empty);
    let identity = [eq(0.into(), 0.into())];
    assert_eq!(
        solve(&identity, &[s("x")]).set(),
        &Set::Affine {
            particular: vec![q(0, 1)],
            basis: vec![vec![q(1, 1)]]
        }
    );
    assert_eq!(
        solve(&[eq(0.into(), 1.into())], &[s("x")]).set(),
        &Set::Empty
    );
}

#[test]
fn affine_distribution_and_exact_constant_power() {
    let e = [eq(
        Expr::mul([2.into(), Expr::add([v("x"), 1.into()])]),
        Expr::pow(2.into(), (-2).into()),
    )];
    assert_eq!(solve(&e, &[s("x")]).set(), &Set::Unique(vec![q(-7, 8)]));
}

#[test]
fn invalid_inputs_and_unsupported_expressions() {
    let ctx = EvaluationContext::default();
    let vars = [s("x"), s("y")];
    let zero = [eq(0.into(), 0.into())];
    assert_eq!(
        solve_linear_system_expr(&ctx, &[], &vars).unwrap_err(),
        Error::EmptyEquations
    );
    assert_eq!(
        solve_linear_system_expr(&ctx, &zero, &[]).unwrap_err(),
        Error::EmptyVariables
    );
    assert_eq!(
        solve_linear_system_expr(&ctx, &zero, &[s("x"), s("x")]).unwrap_err(),
        Error::DuplicateVariable
    );
    let too_many = (0..17).map(|i| s(&format!("x{i}"))).collect::<Vec<_>>();
    assert_eq!(
        solve_linear_system_expr(&ctx, &zero, &too_many).unwrap_err(),
        Error::DimensionLimit
    );
    assert_eq!(
        solve_linear_system_expr(&ctx, &[eq(v("z"), 0.into())], &vars).unwrap_err(),
        Error::UnlistedSymbol
    );
    assert_eq!(
        solve_linear_system_expr(&ctx, &[eq(Expr::mul([v("x"), v("y")]), 0.into())], &vars)
            .unwrap_err(),
        Error::NonAffine
    );
    for function in [
        Expr::sin(v("x")),
        Expr::log(v("x")),
        Expr::function(s("custom"), v("x")),
    ] {
        assert_eq!(
            solve_linear_system_expr(&ctx, &[eq(function, 0.into())], &vars).unwrap_err(),
            Error::UnsupportedFunction
        );
    }
    for bad in [
        Expr::pow(v("x"), 0.into()),
        Expr::pow(v("x"), 2.into()),
        Expr::pow(v("x"), (-1).into()),
        Expr::pow(0.into(), (-1).into()),
        Expr::pow(0.into(), 0.into()),
        Expr::pow(2.into(), v("y")),
        Expr::pow(2.into(), 33.into()),
    ] {
        assert_eq!(
            solve_linear_system_expr(&ctx, &[eq(bad, 0.into())], &vars).unwrap_err(),
            Error::UnsupportedPower
        );
    }
}

#[test]
fn forged_point_basis_order_and_classification_are_rejected() {
    let ctx = EvaluationContext::default();
    let vars = [s("x"), s("y")];
    let e = [eq(Expr::add([v("x"), v("y")]), 3.into())];
    let genuine = solve(&e, &vars);
    assert!(verify_linear_system_expr(&ctx, &e, &vars, &genuine).unwrap());
    let cases = [
        Solution::from_parts(vars.to_vec(), Set::Empty),
        Solution::from_parts(vars.to_vec(), Set::Unique(vec![q(3, 1), q(0, 1)])),
        Solution::from_parts(
            vars.to_vec(),
            Set::Affine {
                particular: vec![q(4, 1), q(0, 1)],
                basis: vec![vec![q(-1, 1), q(1, 1)]],
            },
        ),
        Solution::from_parts(
            vars.to_vec(),
            Set::Affine {
                particular: vec![q(3, 1), q(0, 1)],
                basis: vec![vec![q(1, 1), q(1, 1)]],
            },
        ),
        Solution::from_parts(vec![s("y"), s("x")], genuine.set().clone()),
    ];
    for forged in &cases {
        assert!(!verify_linear_system_expr(&ctx, &e, &vars, forged).unwrap());
    }
}

#[test]
fn bounded_admission_and_determinism() {
    let equations = [eq(Expr::add([v("x"), 1.into()]), 2.into())];
    let vars = [s("x")];
    let low = EvaluationContext::new(ResourceLimits {
        max_expression_nodes: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        solve_linear_system_expr(&low, &equations, &vars).unwrap_err(),
        Error::Evaluation(mathematishia_kernel::EvaluationError::Limit(
            ResourceLimitKind::ExpressionNodes
        ))
    );
    let no_work = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        solve_linear_system_expr(&no_work, &equations, &vars).unwrap_err(),
        Error::Evaluation(mathematishia_kernel::EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        ))
    );
    let first = solve(&equations, &vars);
    for _ in 0..10 {
        assert_eq!(first, solve(&equations, &vars));
    }
}
