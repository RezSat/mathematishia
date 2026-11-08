use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, QuadraticError, RationalFunction,
    RationalPolynomial, RationalQuadraticRoots, verify_rational_quadratic_equation,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .copied()
            .map(|n| Rational::from(Integer::from(n)))
            .collect(),
    )
}
fn rational(num: &[i64], den: &[i64]) -> RationalFunction {
    RationalFunction::new(p(num), p(den)).unwrap()
}
fn roots(values: &[i64]) -> RationalQuadraticRoots {
    RationalQuadraticRoots::Finite(
        values
            .iter()
            .copied()
            .map(|n| Rational::from(Integer::from(n)))
            .collect(),
    )
}

#[test]
fn canceled_pole_is_not_a_quadratic_equation_solution() {
    let ctx = EvaluationContext::default();
    let lhs = rational(&[6, -5, 1], &[-2, 1]);
    let zero = RationalFunction::from_polynomial(p(&[]));
    let result = ctx.solve_rational_quadratic_equation(&lhs, &zero).unwrap();
    assert_eq!(result.roots(), &roots(&[3]));
    assert_eq!(result.domain_conditions(), &[p(&[-2, 1])]);
    assert_eq!(verify_rational_quadratic_equation(&result), Ok(()));
}

#[test]
fn two_distinct_roots_or_no_rational_roots_with_denominators() {
    let ctx = EvaluationContext::default();
    let zero = RationalFunction::from_polynomial(p(&[]));
    let two = rational(&[6, -5, 1], &[0, 1]);
    let result = ctx.solve_rational_quadratic_equation(&two, &zero).unwrap();
    assert_eq!(result.roots(), &roots(&[2, 3]));
    assert_eq!(verify_rational_quadratic_equation(&result), Ok(()));
    let irrational = rational(&[-2, 0, 1], &[0, 1]);
    let result = ctx.solve_rational_quadratic_equation(&irrational, &zero).unwrap();
    assert_eq!(result.roots(), &roots(&[]));
    assert_eq!(verify_rational_quadratic_equation(&result), Ok(()));
}

#[test]
fn identity_is_all_admissible_values_not_all_q() {
    let ctx = EvaluationContext::default();
    let lhs = rational(&[-1, 1], &[-1, 1]);
    let rhs = RationalFunction::from_polynomial(p(&[1]));
    let result = ctx.solve_rational_quadratic_equation(&lhs, &rhs).unwrap();
    assert_eq!(result.roots(), &RationalQuadraticRoots::AllAdmissibleRationals);
    assert_eq!(result.domain_conditions(), &[p(&[-1, 1])]);
    assert_eq!(verify_rational_quadratic_equation(&result), Ok(()));
}

#[test]
fn higher_degree_numerators_fail_explicitly() {
    let ctx = EvaluationContext::default();
    let zero = RationalFunction::from_polynomial(p(&[]));
    let cubic = RationalFunction::from_polynomial(p(&[1, 0, 0, 1]));
    assert_eq!(
        ctx.solve_rational_quadratic_equation(&cubic, &zero),
        Err(EvaluationError::Quadratic(QuadraticError::DegreeExceeded))
    );
}

#[test]
fn deterministic_125_domain_aware_rational_equations_verify() {
    let ctx = EvaluationContext::default();
    let zero = RationalFunction::from_polynomial(p(&[]));
    let mut tested = 0usize;
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                let lhs = rational(&[a, b, c], &[-1, 1]);
                let first = ctx.solve_rational_quadratic_equation(&lhs, &zero).unwrap();
                let second = ctx.solve_rational_quadratic_equation(&lhs, &zero).unwrap();
                assert_eq!(first, second);
                assert_eq!(verify_rational_quadratic_equation(&first), Ok(()));
                tested += 1;
            }
        }
    }
    assert_eq!(tested, 125);
}
