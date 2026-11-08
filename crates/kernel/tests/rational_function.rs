use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalFunction, RationalFunctionError,
    RationalPolynomial, ResourceLimitKind, ResourceLimits,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&v| Rational::from(Integer::from(v)))
            .collect(),
    )
}

fn at(value: i64) -> Rational {
    Rational::from(Integer::from(value))
}

#[test]
fn checked_normalization_preserves_removable_holes_and_exact_values() {
    let ctx = EvaluationContext::default();
    let f = ctx
        .rational_function(&p(&[-1, 0, 1]), &p(&[-1, 1]))
        .unwrap();
    assert_eq!(f.numerator(), &p(&[1, 1]));
    assert_eq!(f.denominator(), &p(&[1]));
    assert_eq!(f.excluded_zero_sets(), &[p(&[-1, 1])]);
    assert_eq!(
        ctx.evaluate_rational_function(&f, &at(1)),
        Err(EvaluationError::RationalFunction(
            RationalFunctionError::UndefinedAtPoint
        ))
    );
    assert_eq!(ctx.evaluate_rational_function(&f, &at(2)), Ok(at(3)));
}

#[test]
fn all_checked_fraction_operations_match_trusted_exact_paths() {
    let ctx = EvaluationContext::default();
    let a = ctx.rational_function(&p(&[1, 1]), &p(&[-1, 1])).unwrap();
    let b = ctx.rational_function(&p(&[2, 1]), &p(&[0, 1])).unwrap();
    assert_eq!(ctx.add_rational_functions(&a, &b), Ok(a.add(&b).unwrap()));
    assert_eq!(
        ctx.subtract_rational_functions(&a, &b),
        Ok(a.subtract(&b).unwrap())
    );
    assert_eq!(
        ctx.multiply_rational_functions(&a, &b),
        Ok(a.multiply(&b).unwrap())
    );
    assert_eq!(
        ctx.divide_rational_functions(&a, &b),
        Ok(a.divide(&b).unwrap())
    );
    assert_eq!(
        ctx.differentiate_rational_function(&a),
        Ok(a.differentiate().unwrap())
    );
}

#[test]
fn zero_divisor_and_zero_denominator_fail_with_typed_errors() {
    let ctx = EvaluationContext::default();
    let a = RationalFunction::from_polynomial(p(&[1, 1]));
    let zero = RationalFunction::from_polynomial(p(&[]));
    assert_eq!(
        ctx.divide_rational_functions(&a, &zero),
        Err(EvaluationError::RationalFunction(
            RationalFunctionError::ZeroDivisor
        ))
    );
    assert_eq!(
        ctx.rational_function(&p(&[1]), &p(&[])),
        Err(EvaluationError::RationalFunction(
            RationalFunctionError::ZeroDenominator
        ))
    );
}

#[test]
fn checked_work_budget_and_intermediate_degree_are_enforced() {
    let short = EvaluationContext::new(ResourceLimits {
        max_work_units: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        short.rational_function(&p(&[1, 1]), &p(&[-1, 1])),
        Err(EvaluationError::Limit(ResourceLimitKind::WorkUnits))
    );
    let small = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 2,
        ..ResourceLimits::default()
    });
    let f = small
        .rational_function(&p(&[1, 1, 1]), &p(&[-1, 1]))
        .unwrap();
    assert_eq!(
        small.multiply_rational_functions(&f, &f),
        Err(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree))
    );
}

#[test]
fn checked_and_low_level_results_agree_on_125_dense_polynomial_pairs() {
    let ctx = EvaluationContext::default();
    let mut cases = 0;
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                let n = p(&[a, b, c]);
                let d = p(&[b, 1]);
                let trusted = RationalFunction::new(n.clone(), d.clone()).unwrap();
                let checked = ctx.rational_function(&n, &d).unwrap();
                assert_eq!(checked, trusted);
                assert_eq!(ctx.rational_function(&n, &d).unwrap(), checked);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 125);
}

#[test]
fn quotient_rule_preserves_a_canceled_domain_hole() {
    let ctx = EvaluationContext::default();
    let canceled_one = ctx.rational_function(&p(&[-1, 1]), &p(&[-1, 1])).unwrap();
    let derivative = ctx.differentiate_rational_function(&canceled_one).unwrap();
    assert!(derivative.is_zero());
    assert_eq!(derivative.excluded_zero_sets(), &[p(&[-1, 1])]);
    assert_eq!(
        ctx.evaluate_rational_function(&derivative, &at(1)),
        Err(EvaluationError::RationalFunction(
            RationalFunctionError::UndefinedAtPoint
        ))
    );
}
