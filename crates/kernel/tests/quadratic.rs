use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, QuadraticError, QuadraticRoots, RationalPolynomial,
    ResourceLimitKind, ResourceLimits, verify_quadratic_roots,
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
fn roots(values: &[i64]) -> QuadraticRoots {
    QuadraticRoots::Finite(
        values
            .iter()
            .copied()
            .map(|n| Rational::from(Integer::from(n)))
            .collect(),
    )
}

#[test]
fn all_quadratic_solution_cardinalities_are_exact() {
    let ctx = EvaluationContext::default();
    for (poly, expected) in [
        (p(&[]), QuadraticRoots::AllRationals),
        (p(&[3]), roots(&[])),
        (p(&[-3, 1]), roots(&[3])),
        (p(&[6, -5, 1]), roots(&[2, 3])),
        (p(&[4, -4, 1]), roots(&[2])),
        (p(&[-2, 0, 1]), roots(&[])),
        (p(&[1, 0, 1]), roots(&[])),
    ] {
        let result = ctx.solve_quadratic(&poly).unwrap();
        assert_eq!(result.roots(), &expected);
        assert_eq!(verify_quadratic_roots(&result), Ok(()));
    }
}

#[test]
fn fractional_rational_roots_and_coefficients_are_exact() {
    let one = Rational::from(Integer::one());
    let two = Rational::from(Integer::from(2));
    let half = Rational::new(Integer::one(), Integer::from(2)).unwrap();
    let poly = RationalPolynomial::new(vec![one, -&two, half]);
    let result = EvaluationContext::default().solve_quadratic(&poly).unwrap();
    // The discriminant is 2, so there are no rational roots.
    assert_eq!(result.roots(), &roots(&[]));
    let poly = RationalPolynomial::new(vec![
        Rational::new(Integer::from(1), Integer::from(8)).unwrap(),
        Rational::new(Integer::from(-3), Integer::from(4)).unwrap(),
        Rational::from(Integer::one()),
    ]);
    let result = EvaluationContext::default().solve_quadratic(&poly).unwrap();
    let r1 = Rational::new(Integer::from(1), Integer::from(4)).unwrap();
    let r2 = Rational::new(Integer::from(1), Integer::from(2)).unwrap();
    assert_eq!(result.roots(), &QuadraticRoots::Finite(vec![r1, r2]));
}

#[test]
fn deterministic_125_quadratic_coefficient_combinations() {
    let ctx = EvaluationContext::default();
    let mut n = 0;
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                let poly = p(&[a, b, c]);
                let one = ctx.solve_quadratic(&poly).unwrap();
                let two = ctx.solve_quadratic(&poly).unwrap();
                assert_eq!(one, two);
                assert_eq!(verify_quadratic_roots(&one), Ok(()));
                n += 1;
            }
        }
    }
    assert_eq!(n, 125);
}

#[test]
fn degree_and_work_limits_are_typed() {
    let ctx = EvaluationContext::default();
    assert_eq!(ctx.solve_quadratic(&p(&[1, 2, 3, 4])), Err(EvaluationError::Quadratic(QuadraticError::DegreeExceeded)));
    let tiny = EvaluationContext::new(ResourceLimits { max_work_units: 0, ..ResourceLimits::default() });
    assert_eq!(tiny.solve_quadratic(&p(&[1, -2, 1])), Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits)));
}
