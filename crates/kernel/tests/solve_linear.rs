use mathema::{Equation, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    EquationRuleId, LinearSolution, LinearSolveError, PolynomialConversionError,
    Relation, VerifiedCondition, solve_linear_equation,
};

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}
fn symbol(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}
fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn equation(left: Expr, right: Expr) -> Equation {
    Equation::new(left, right)
}
fn solve(eq: &Equation) -> Result<mathematishia_kernel::LinearSolveResult, LinearSolveError> {
    solve_linear_equation(eq, &x())
}

#[test]
fn finds_unique_exact_rational_root_with_verified_division() {
    let eq = equation(
        Expr::add([Expr::mul([2.into(), symbol("x")]), 4.into()]),
        Expr::from(10),
    );
    let result = solve(&eq).unwrap();
    assert_eq!(result.original(), &eq);
    assert_eq!(result.solution(), &LinearSolution::Unique(q(3, 1)));
    assert_eq!(result.residual().evaluate(&q(3, 1)), q(0, 1));
    assert!(!result.steps().is_empty());
    assert_eq!(result.steps()[0].before(), &eq);
    for pair in result.steps().windows(2) {
        assert_eq!(pair[0].after(), pair[1].before());
    }
    let last = result.steps().last().unwrap();
    assert_eq!(last.after(), &equation(symbol("x"), Expr::from(3)));
    assert_eq!(last.rule(), EquationRuleId::DivideByNonzeroCoefficient);
    assert_eq!(last.rule().stable_id(), "equation.divide_by_nonzero_coefficient");
    assert_eq!(last.relation(), Relation::Equivalent);
    assert_eq!(last.verified_condition(), Some(&VerifiedCondition::Nonzero(q(2, 1))));
}

#[test]
fn solves_repeated_variables_and_parenthesized_linear_forms() {
    let eq = equation(Expr::add([symbol("x"), symbol("x")]), 4.into());
    assert_eq!(solve(&eq).unwrap().solution(), &LinearSolution::Unique(q(2, 1)));
    let nested = equation(
        Expr::mul([2.into(), Expr::add([symbol("x"), 1.into()])]),
        Expr::from(6),
    );
    assert_eq!(
        solve(&nested).unwrap().solution(),
        &LinearSolution::Unique(q(2, 1))
    );
}

#[test]
fn solves_negative_and_fractional_coefficients() {
    let eq = equation(
        Expr::add([
            Expr::mul([Expr::from(q(-3, 2)), symbol("x")]),
            Expr::from(q(1, 3)),
        ]),
        Expr::from(0),
    );
    let result = solve(&eq).unwrap();
    assert_eq!(result.solution(), &LinearSolution::Unique(q(2, 9)));
    assert_eq!(result.residual().evaluate(&q(2, 9)), q(0, 1));
    let last = result.steps().last().unwrap();
    assert_eq!(last.verified_condition(), Some(&VerifiedCondition::Nonzero(q(-3, 2))));
}

#[test]
fn classifies_true_and_false_constant_equations() {
    let identity = equation(Expr::from(3), Expr::from(3));
    let all = solve(&identity).unwrap();
    assert_eq!(all.solution(), &LinearSolution::AllRationals);
    assert!(all.residual().is_zero());

    let contradiction = equation(Expr::from(3), Expr::from(4));
    let none = solve(&contradiction).unwrap();
    assert_eq!(none.solution(), &LinearSolution::NoSolution);
    assert_eq!(none.residual().evaluate(&q(27, 1)), q(-1, 1));
    assert!(all.steps().iter().all(|step| step.verified_condition().is_none()));
    assert!(none.steps().iter().all(|step| step.verified_condition().is_none()));
}

#[test]
fn distinguishes_cancelling_variables() {
    let identity = equation(
        Expr::add([symbol("x"), 1.into()]),
        Expr::add([symbol("x"), 1.into()]),
    );
    let contradiction = equation(
        Expr::add([symbol("x"), 1.into()]),
        Expr::add([symbol("x"), 2.into()]),
    );
    assert_eq!(solve(&identity).unwrap().solution(), &LinearSolution::AllRationals);
    assert_eq!(solve(&contradiction).unwrap().solution(), &LinearSolution::NoSolution);
}

#[test]
fn rejects_nonlinear_and_unrelated_variables() {
    let quadratic = equation(Expr::pow(symbol("x"), 2.into()), 1.into());
    assert_eq!(solve(&quadratic), Err(LinearSolveError::NonlinearEquation));
    let another = equation(symbol("y"), 0.into());
    assert_eq!(
        solve(&another),
        Err(LinearSolveError::UnsupportedExpression(PolynomialConversionError::OtherSymbol))
    );
    let ambiguous = equation(Expr::pow(symbol("x"), 0.into()), 1.into());
    assert_eq!(
        solve(&ambiguous),
        Err(LinearSolveError::UnsupportedExpression(PolynomialConversionError::UnsupportedPower))
    );
}

#[test]
fn a_simple_equation_is_already_solved_without_unnecessary_steps() {
    let eq = equation(symbol("x"), Expr::from(0));
    let result = solve(&eq).unwrap();
    assert_eq!(result.solution(), &LinearSolution::Unique(q(0, 1)));
    assert!(result.steps().is_empty());
}

#[test]
fn verified_condition_only_occurs_on_coefficient_division() {
    let eq = equation(
        Expr::add([Expr::mul([3.into(), symbol("x")]), 5.into()]),
        2.into(),
    );
    let result = solve(&eq).unwrap();
    for step in result.steps() {
        assert_eq!(step.relation(), Relation::Equivalent);
        if step.rule() == EquationRuleId::DivideByNonzeroCoefficient {
            assert_eq!(step.verified_condition(), Some(&VerifiedCondition::Nonzero(q(3, 1))));
        } else {
            assert!(step.verified_condition().is_none());
        }
    }
}

#[test]
fn unique_solutions_satisfy_both_original_sides() {
    let samples = [
        equation(
            Expr::add([Expr::mul([2.into(), symbol("x")]), 3.into()]),
            5.into(),
        ),
        equation(
            Expr::add([symbol("x"), Expr::from(q(2, 3))]),
            Expr::mul([Expr::from(q(1, 2)), symbol("x")]),
        ),
        equation(Expr::mul([4.into(), Expr::add([symbol("x"), 2.into()])]), 0.into()),
    ];
    for eq in samples {
        let result = solve(&eq).unwrap();
        let LinearSolution::Unique(root) = result.solution() else {
            panic!("expected a unique root");
        };
        let a = mathematishia_kernel::RationalPolynomial::try_from_expr(eq.left(), &x(), 32).unwrap();
        let b = mathematishia_kernel::RationalPolynomial::try_from_expr(eq.right(), &x(), 32).unwrap();
        assert_eq!(a.evaluate(root), b.evaluate(root));
        assert_eq!(result.residual().evaluate(root), q(0, 1));
    }
}

#[test]
fn leaves_original_equation_and_expressions_unchanged() {
    let lhs = Expr::add([symbol("x"), 3.into()]);
    let rhs = Expr::from(5);
    let eq = equation(lhs.clone(), rhs.clone());
    let saved = eq.clone();
    let _ = solve(&eq).unwrap();
    assert_eq!(eq, saved);
    assert_eq!(eq.left(), &lhs);
    assert_eq!(eq.right(), &rhs);
}
