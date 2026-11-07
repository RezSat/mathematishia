use mathema::{Equation, Expr, Integer, Rational, Symbol};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalPolynomial, ResourceLimitKind, ResourceLimits,
};

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}
fn q(value: i64) -> Rational {
    Rational::from(Integer::from(value))
}
fn poly(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(values.iter().copied().map(q).collect())
}
fn limits(work: u64) -> EvaluationContext {
    EvaluationContext::new(ResourceLimits {
        max_work_units: work,
        ..ResourceLimits::default()
    })
}

#[test]
fn default_budgets_are_instance_local_and_nonzero() {
    let options = ResourceLimits::default();
    assert!(options.max_work_units > 0);
    assert!(options.max_intermediate_scalar_bits >= options.max_scalar_bits);
    assert_eq!(
        EvaluationContext::default(),
        EvaluationContext::new(options)
    );
}

#[test]
fn work_zero_denies_polynomial_arithmetic_and_conversion() {
    let ctx = limits(0);
    let polynomial = poly(&[1, 2, 3]);
    assert_eq!(
        ctx.differentiate(&polynomial),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    assert_eq!(
        ctx.evaluate_polynomial(&polynomial, &q(3)),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    assert_eq!(
        ctx.polynomial(&Expr::from(x()), &x()),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
}

#[test]
fn exact_boundary_is_deterministic_across_calls() {
    let ctx = limits(1);
    let linear = poly(&[1, 1]);
    assert_eq!(ctx.differentiate(&linear), Ok(poly(&[1])));
    assert_eq!(ctx.differentiate(&linear), Ok(poly(&[1])));
    assert_eq!(
        ctx.differentiate(&poly(&[1, 2, 3])),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
}

#[test]
fn oversized_intermediate_is_rejected_even_if_final_cancels() {
    let ctx = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 64,
        max_intermediate_scalar_bits: 8,
        ..ResourceLimits::default()
    });
    let at = Rational::new(Integer::from(200), Integer::from(199)).unwrap();
    let polynomial = RationalPolynomial::new(vec![at.clone(), -at.clone()]);
    assert_eq!(
        ctx.evaluate_polynomial(&polynomial, &q(1)),
        Err(EvaluationError::Limit(
            ResourceLimitKind::IntermediateScalarBits
        ))
    );
}

#[test]
fn checked_gcd_division_and_squarefree_fail_under_small_work_budget() {
    let ctx = limits(2);
    let a = poly(&[1, 2, 1]);
    let b = poly(&[1, 1]);
    assert_eq!(
        ctx.divide(&a, &b),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    assert_eq!(
        ctx.gcd(&a, &b),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    assert_eq!(
        ctx.square_free_decomposition(&a),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    let default = EvaluationContext::default();
    assert!(default.divide(&a, &b).is_ok());
    assert!(default.gcd(&a, &b).is_ok());
    assert!(default.square_free_decomposition(&a).is_ok());
}

#[test]
fn high_degree_cancellation_cannot_bypass_preflight() {
    let variable = Expr::from(x());
    let large = Expr::pow(variable.clone(), Expr::from(12));
    let original = Equation::new(Expr::add([large.clone(), variable]), large);
    let ctx = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 2,
        ..ResourceLimits::default()
    });
    assert!(ctx.solve_linear(&original, &x()).is_err());
}

#[test]
fn linear_solver_preflights_intermediate_rational_quotients() {
    let coefficient = Rational::new(Integer::from(200), Integer::from(199)).unwrap();
    let variable = Expr::from(x());
    let equation = Equation::new(
        Expr::mul([Expr::from(coefficient.clone()), variable]),
        Expr::from(coefficient),
    );
    let ctx = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 64,
        max_intermediate_scalar_bits: 8,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.solve_linear(&equation, &x()),
        Err(EvaluationError::Limit(
            ResourceLimitKind::IntermediateScalarBits
        ))
    );
}

#[test]
fn expansion_and_solver_have_explicit_work_limits() {
    let variable = Expr::from(x());
    let input = Expr::mul([variable.clone(), Expr::add([variable.clone(), 1.into()])]);
    assert_eq!(
        limits(0).expand(&input),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    let equation = Equation::new(variable, 3.into());
    assert_eq!(
        limits(0).solve_linear(&equation, &x()),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
}

#[test]
fn repeated_checked_squarefree_results_are_exact_and_equal() {
    let p = poly(&[-2, 3, -1]);
    let ctx = EvaluationContext::default();
    let a = ctx.square_free_decomposition(&p).unwrap();
    let b = ctx.square_free_decomposition(&p).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.unit(), &q(-1));
}
