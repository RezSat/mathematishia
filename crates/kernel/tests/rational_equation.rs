use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalEquationError, RationalEquationSolution,
    RationalFunction, RationalPolynomial, verify_rational_equation,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&v| Rational::from(Integer::from(v)))
            .collect(),
    )
}

fn f(n: &[i64], d: &[i64]) -> RationalFunction {
    RationalFunction::new(p(n), p(d)).unwrap()
}

#[test]
fn exact_linear_rational_equation_has_unique_admissible_root() {
    let ctx = EvaluationContext::default();
    let left = f(&[1, 2], &[0, 1]);
    let right = RationalFunction::from_polynomial(p(&[3]));
    let result = ctx.solve_rational_equation(&left, &right).unwrap();
    assert_eq!(
        result.solution(),
        &RationalEquationSolution::Unique(Rational::from(Integer::from(1)))
    );
    assert_eq!(verify_rational_equation(&result), Ok(()));
    assert_eq!(result.domain_conditions(), &[p(&[0, 1])]);
}

#[test]
fn canceled_pole_cannot_be_promoted_to_an_equation_root() {
    let ctx = EvaluationContext::default();
    let left = f(&[-1, 0, 1], &[-1, 1]);
    let right = RationalFunction::from_polynomial(p(&[2]));
    let result = ctx.solve_rational_equation(&left, &right).unwrap();
    assert_eq!(result.solution(), &RationalEquationSolution::NoSolution);
    assert_eq!(result.domain_conditions(), &[p(&[-1, 1])]);
    assert_eq!(verify_rational_equation(&result), Ok(()));
}

#[test]
fn identity_equation_retains_domain_instead_of_claiming_all_q() {
    let ctx = EvaluationContext::default();
    let left = f(&[-1, 1], &[-1, 1]);
    let right = RationalFunction::from_polynomial(p(&[1]));
    let result = ctx.solve_rational_equation(&left, &right).unwrap();
    assert_eq!(
        result.solution(),
        &RationalEquationSolution::AllAdmissibleRationals
    );
    assert_eq!(result.domain_conditions(), &[p(&[-1, 1])]);
    assert_eq!(verify_rational_equation(&result), Ok(()));
}

#[test]
fn nonzero_constant_and_high_degree_numerators_are_classified() {
    let ctx = EvaluationContext::default();
    let zero = RationalFunction::from_polynomial(p(&[]));
    let constant = f(&[1], &[0, 1]);
    let result = ctx.solve_rational_equation(&constant, &zero).unwrap();
    assert_eq!(result.solution(), &RationalEquationSolution::NoSolution);
    let nonlinear = f(&[-1, 0, 1], &[-2, 1]);
    assert_eq!(
        ctx.solve_rational_equation(&nonlinear, &zero),
        Err(EvaluationError::RationalEquation(
            RationalEquationError::NonlinearNumerator
        ))
    );
}

#[test]
fn rational_identity_and_distinct_denominators_have_exact_domain_union() {
    let ctx = EvaluationContext::default();
    let left = f(&[1], &[-1, 1]);
    let right = f(&[1], &[0, 1]);
    let result = ctx.solve_rational_equation(&left, &right).unwrap();
    assert_eq!(result.solution(), &RationalEquationSolution::NoSolution);
    assert_eq!(result.domain_conditions(), &[p(&[-1, 1]), p(&[0, 1])]);
}

#[test]
fn determinism_for_125_rational_linear_equations() {
    let ctx = EvaluationContext::default();
    let zero = RationalFunction::from_polynomial(p(&[]));
    let mut cases = 0;
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                let f = f(&[a, b], &[c, 1]);
                let first = ctx.solve_rational_equation(&f, &zero).unwrap();
                let second = ctx.solve_rational_equation(&f, &zero).unwrap();
                assert_eq!(first, second);
                assert_eq!(verify_rational_equation(&first), Ok(()));
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 125);
}
