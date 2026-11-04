use mathema::{Equation, Expr, Integer, Symbol};
use mathematishia_kernel::{
    VerificationError, expand_distributively, solve_linear_equation, verify_distribution,
    verify_distribution_step, verify_equation_step, verify_linear_solution,
};

fn symbol(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}
fn expr(name: &str) -> Expr {
    Expr::from(symbol(name))
}

#[test]
fn verifies_root_and_nested_distributive_steps_without_replaying_generator() {
    let original = Expr::mul([
        Expr::add([expr("a"), expr("b")]),
        Expr::add([expr("c"), expr("d")]),
    ]);
    let result = expand_distributively(&original, 3);
    assert!(result.is_complete());
    assert_eq!(verify_distribution(&original, &result), Ok(()));
    for step in result.steps() {
        assert_eq!(verify_distribution_step(step), Ok(()));
    }
    let partial = expand_distributively(&original, 1);
    assert_eq!(verify_distribution(&original, &partial), Ok(()));
    assert!(!partial.is_complete());
}

#[test]
fn verifies_zero_budget_and_detects_incorrect_start() {
    let original = Expr::mul([expr("x"), Expr::add([expr("y"), 1.into()])]);
    let zero = expand_distributively(&original, 0);
    assert_eq!(verify_distribution(&original, &zero), Ok(()));
    assert_eq!(
        verify_distribution(&expr("unrelated"), &zero),
        Err(VerificationError::IncorrectResult)
    );
    let nonzero = expand_distributively(&original, 1);
    assert_eq!(
        verify_distribution(&expr("unrelated"), &nonzero),
        Err(VerificationError::BrokenChain)
    );
}

#[test]
fn verifies_exact_linear_equation_steps_and_conditions() {
    let eq = Equation::new(
        Expr::add([Expr::mul([2.into(), expr("x")]), 4.into()]),
        10.into(),
    );
    let result = solve_linear_equation(&eq, &symbol("x")).unwrap();
    assert_eq!(verify_linear_solution(&result, &symbol("x")), Ok(()));
    for step in result.steps() {
        assert_eq!(verify_equation_step(step, &symbol("x")), Ok(()));
    }
    assert_eq!(
        verify_linear_solution(&result, &symbol("z")),
        Err(VerificationError::UnsupportedDomain)
    );
}

#[test]
fn verifies_empty_and_universal_rational_solution_sets() {
    for (lhs, rhs) in [(3, 4), (3, 3)] {
        let eq = Equation::new(Expr::from(lhs), Expr::from(rhs));
        let result = solve_linear_equation(&eq, &symbol("x")).unwrap();
        assert_eq!(verify_linear_solution(&result, &symbol("x")), Ok(()));
    }
    let eq = Equation::new(
        Expr::add([expr("x"), 1.into()]),
        Expr::add([expr("x"), 2.into()]),
    );
    assert_eq!(
        verify_linear_solution(
            &solve_linear_equation(&eq, &symbol("x")).unwrap(),
            &symbol("x")
        ),
        Ok(())
    );
}

#[test]
fn verifies_large_exact_coefficients_without_sampling() {
    let huge = Expr::from(Integer::from(u128::MAX));
    let eq = Equation::new(Expr::mul([huge, expr("x")]), Expr::from(0));
    let result = solve_linear_equation(&eq, &symbol("x")).unwrap();
    assert_eq!(verify_linear_solution(&result, &symbol("x")), Ok(()));
}
