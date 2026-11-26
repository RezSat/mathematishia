use mathema::{Integer, Rational, Symbol};
use mathematishia_kernel::step_records::division_trace::{
    DivisionReductionRule, record_ordered_division, verify_ordered_division,
};
use mathematishia_kernel::step_records::{StepLimits, StepRecordError};
use mathematishia_kernel::{MultivariateLimits, MultivariatePolynomial, divide_multivariate};

fn p(terms: &[(i32, i32, i64)]) -> MultivariatePolynomial {
    MultivariatePolynomial::try_new(
        vec![Symbol::new("x").unwrap(), Symbol::new("y").unwrap()],
        terms
            .iter()
            .map(|&(x, y, c)| (vec![x, y], Rational::from(Integer::from(c)))),
        MultivariateLimits::default(),
    )
    .unwrap()
}

fn example() -> (MultivariatePolynomial, Vec<MultivariatePolynomial>) {
    (
        p(&[(2, 0, 1), (1, 1, 1), (0, 2, 1), (0, 0, 1)]),
        vec![p(&[(1, 0, 1), (0, 1, 1)]), p(&[(1, 0, 1)])],
    )
}

#[test]
fn records_actual_priority_and_remainder_moves() {
    let (f, g) = example();
    let limits = StepLimits::default();
    let trace = record_ordered_division(&f, &g, limits).unwrap();
    assert_eq!(trace.steps.len(), 3);
    let first = &trace.steps[0];
    assert_eq!(first.before, f);
    assert_eq!(first.after, p(&[(0, 2, 1), (0, 0, 1)]));
    assert_eq!(first.factor, p(&[(1, 0, 1)]));
    assert_eq!(
        first.rule,
        DivisionReductionRule::Reduce { divisor_index: 0 }
    );
    assert_eq!(first.premise, None);
    assert_eq!(trace.steps[1].premise, Some(0));
    assert_eq!(trace.steps[1].rule, DivisionReductionRule::MoveToRemainder);
    assert_eq!(trace.steps[2].after, p(&[]));
    assert_eq!(trace.quotients, vec![p(&[(1, 0, 1)]), p(&[])]);
    assert_eq!(trace.remainder, p(&[(0, 2, 1), (0, 0, 1)]));
    verify_ordered_division(&f, &g, &trace, limits).unwrap();
    assert_eq!(trace, record_ordered_division(&f, &g, limits).unwrap());
    let original = divide_multivariate(&f, &g, MultivariateLimits::default()).unwrap();
    assert_eq!(trace.quotients, original.quotients());
    assert_eq!(&trace.remainder, original.remainder());
    assert_eq!(trace.steps.len(), original.steps());
}

#[test]
fn exact_fractional_univariate_reduction() {
    let variable = Symbol::new("x").unwrap();
    let build = |terms: &[(i32, i64, i64)]| {
        MultivariatePolynomial::try_new(
            vec![variable.clone()],
            terms.iter().map(|&(power, num, den)| {
                (
                    vec![power],
                    Rational::new(Integer::from(num), Integer::from(den)).unwrap(),
                )
            }),
            MultivariateLimits::default(),
        )
        .unwrap()
    };
    let f = build(&[(2, 1, 1)]);
    let g = [build(&[(1, 2, 1), (0, 1, 1)])];
    let limits = StepLimits::default();
    let trace = record_ordered_division(&f, &g, limits).unwrap();
    assert_eq!(trace.steps[0].factor, build(&[(1, 1, 2)]));
    assert_eq!(trace.steps[1].factor, build(&[(0, -1, 4)]));
    assert_eq!(trace.quotients, vec![build(&[(1, 1, 2), (0, -1, 4)])]);
    assert_eq!(trace.remainder, build(&[(0, 1, 4)]));
    verify_ordered_division(&f, &g, &trace, limits).unwrap();
}

#[test]
fn rejects_forged_transitions_and_premises() {
    let (f, g) = example();
    let limits = StepLimits::default();
    let trace = record_ordered_division(&f, &g, limits).unwrap();
    let reject = |claim| assert!(verify_ordered_division(&f, &g, &claim, limits).is_err());

    let mut forged = trace.clone();
    forged.steps[0].rule = DivisionReductionRule::Reduce { divisor_index: 1 };
    reject(forged);
    let mut forged = trace.clone();
    forged.steps[0].factor = p(&[(1, 0, 2)]);
    reject(forged);
    let mut forged = trace.clone();
    forged.steps[0].after = p(&[]);
    reject(forged);
    let mut forged = trace.clone();
    forged.steps[1].premise = None;
    reject(forged);
    let mut forged = trace.clone();
    forged.steps.swap(1, 2);
    reject(forged);
    let mut forged = trace.clone();
    forged.steps.pop();
    reject(forged);
    let mut forged = trace.clone();
    forged.steps.push(trace.steps[2].clone());
    reject(forged);
    let mut forged = trace.clone();
    forged.remainder = p(&[(0, 0, 1)]);
    reject(forged);
    let mut forged = trace;
    forged.quotients[0] = p(&[(0, 0, 1)]);
    reject(forged);
}

#[test]
fn rejects_valid_recomposition_from_wrong_divisor_priority() {
    let f = p(&[(1, 0, 1)]);
    let g = vec![p(&[(1, 0, 1), (0, 1, 1)]), p(&[(1, 0, 1)])];
    let reversed = vec![g[1].clone(), g[0].clone()];
    let limits = StepLimits::default();
    let mut alternate = record_ordered_division(&f, &reversed, limits).unwrap();
    alternate.ordered_divisors = g.clone();
    alternate.quotients.swap(0, 1);
    alternate.steps[0].rule = DivisionReductionRule::Reduce { divisor_index: 1 };
    assert!(verify_ordered_division(&f, &g, &alternate, limits).is_err());
}

#[test]
fn rejects_source_substitution_even_with_same_final_values() {
    let (f, g) = example();
    let limits = StepLimits::default();
    let trace = record_ordered_division(&f, &g, limits).unwrap();
    let mut reversed = g.clone();
    reversed.reverse();
    assert!(verify_ordered_division(&f, &reversed, &trace, limits).is_err());
    assert!(verify_ordered_division(&p(&[]), &g, &trace, limits).is_err());
}

#[test]
fn handles_zero_and_refuses_invalid_or_excessive_inputs() {
    let (f, g) = example();
    let limits = StepLimits::default();
    let zero = p(&[]);
    let trace = record_ordered_division(&zero, &g, limits).unwrap();
    assert!(trace.steps.is_empty());
    verify_ordered_division(&zero, &g, &trace, limits).unwrap();
    assert!(record_ordered_division(&f, &[], limits).is_err());
    assert!(record_ordered_division(&f, &[zero], limits).is_err());
    assert!(record_ordered_division(&p(&[(33, 0, 1)]), &g, limits).is_err());
    let tiny = StepLimits {
        max_steps: 2,
        ..limits
    };
    assert_eq!(
        record_ordered_division(&f, &g, tiny),
        Err(StepRecordError::ResourceLimit)
    );
    let trace = record_ordered_division(&f, &g, limits).unwrap();
    assert!(verify_ordered_division(&f, &g, &trace, tiny).is_err());
    let tiny = StepLimits {
        max_work_units: 1,
        ..limits
    };
    assert!(record_ordered_division(&f, &g, tiny).is_err());
    assert!(verify_ordered_division(&f, &g, &trace, tiny).is_err());
}

#[test]
fn checked_json_rejects_forged_trace() {
    let (f, g) = example();
    let limits = StepLimits::default();
    let mut trace = record_ordered_division(&f, &g, limits).unwrap();
    let json = trace.to_checked_json(&f, &g, limits).unwrap();
    assert!(json.contains("\"kind\":\"checked_ordered_division\""));
    assert!(json.contains("\"rule_id\":\"polynomial.reduce_leading_term\""));
    trace.steps[0].premise = Some(0);
    assert!(trace.to_checked_json(&f, &g, limits).is_err());
}
