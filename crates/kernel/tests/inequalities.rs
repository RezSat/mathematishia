use mathema::{Integer, Rational};
use mathematishia_kernel::{
    ComparisonOperator as Op, EvaluationContext, EvaluationError, ExactQuadraticRoot,
    InequalityError, RationalFunction, RationalPolynomial, RealBound, RealInterval,
    ResourceLimitKind, ResourceLimits, solve_rational_inequality, verify_inequality_result,
};

fn p(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&value| Rational::from(Integer::from(value)))
            .collect(),
    )
}

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn finite(value: i64) -> RealBound {
    RealBound::Finite(ExactQuadraticRoot::Rational(q(value, 1)))
}

fn interval(lower: RealBound, upper: RealBound, left: bool, right: bool) -> RealInterval {
    RealInterval {
        lower,
        upper,
        lower_closed: left,
        upper_closed: right,
    }
}

fn solve(n: &[i64], d: &[i64], operator: Op) -> Vec<RealInterval> {
    let context = EvaluationContext::default();
    let function = context.rational_function(&p(n), &p(d)).unwrap();
    let result = context
        .solve_rational_inequality(&function, operator)
        .unwrap();
    assert_eq!(verify_inequality_result(&context, &result), Ok(true));
    assert_eq!(
        context
            .solve_rational_inequality(&function, operator)
            .unwrap(),
        result
    );
    result.intervals().to_vec()
}

#[test]
fn five_common_questions_have_exact_interval_answers() {
    use RealBound::{NegativeInfinity as Neg, PositiveInfinity as Pos};
    assert_eq!(
        solve(&[-4, 0, 1], &[1], Op::Greater),
        vec![
            interval(Neg, finite(-2), false, false),
            interval(finite(2), Pos, false, false),
        ],
    );
    assert_eq!(
        solve(&[0, 0, 1], &[1], Op::LessOrEqual),
        vec![interval(finite(0), finite(0), true, true)],
    );
    assert_eq!(
        solve(&[-1, 1], &[2, 1], Op::LessOrEqual),
        vec![interval(finite(-2), finite(1), false, true)],
    );
    assert_eq!(
        solve(&[-3, 1], &[1], Op::GreaterOrEqual),
        vec![interval(finite(3), Pos, true, false)],
    );
    assert_eq!(
        solve(&[6, -5, 1], &[1], Op::Less),
        vec![interval(finite(2), finite(3), false, false)],
    );
}

#[test]
fn equality_boundaries_and_real_domain_are_exact() {
    use RealBound::{NegativeInfinity as Neg, PositiveInfinity as Pos};
    assert_eq!(
        solve(&[0, 0, 1], &[1], Op::GreaterOrEqual),
        vec![interval(Neg, Pos, false, false)],
    );
    assert_eq!(solve(&[1, 0, 1], &[1], Op::LessOrEqual), vec![],);
    assert_eq!(
        solve(&[1, 0, 1], &[1], Op::Greater),
        vec![interval(Neg, Pos, false, false)],
    );
    assert_eq!(
        solve(&[0], &[1], Op::LessOrEqual),
        vec![interval(Neg, Pos, false, false)],
    );
    assert_eq!(solve(&[0], &[1], Op::Greater), vec![]);
    assert_eq!(solve(&[1], &[1], Op::Less), vec![],);
}

#[test]
fn removable_holes_remain_excluded_after_cancellation() {
    use RealBound::{NegativeInfinity as Neg, PositiveInfinity as Pos};
    // (x^2 - 1)/(x - 1) = x + 1, but x = 1 is undefined.
    assert_eq!(
        solve(&[-1, 0, 1], &[-1, 1], Op::Greater),
        vec![
            interval(finite(-1), finite(1), false, false),
            interval(finite(1), Pos, false, false),
        ],
    );
    assert_eq!(
        solve(&[-1, 1], &[-1, 1], Op::GreaterOrEqual),
        vec![
            interval(Neg, finite(1), false, false),
            interval(finite(1), Pos, false, false),
        ],
    );
    assert_eq!(
        solve(&[0], &[0, 1], Op::LessOrEqual),
        vec![
            interval(Neg, finite(0), false, false),
            interval(finite(0), Pos, false, false),
        ],
    );
}

#[test]
fn even_multiplicity_poles_and_multiple_exclusions() {
    use RealBound::{NegativeInfinity as Neg, PositiveInfinity as Pos};
    assert_eq!(
        solve(&[1], &[1, -2, 1], Op::Greater),
        vec![
            interval(Neg, finite(1), false, false),
            interval(finite(1), Pos, false, false),
        ],
    );
    let context = EvaluationContext::default();
    let f = RationalFunction::from_polynomial(p(&[1]))
        .restrict_nonzero(p(&[-1, 1]))
        .unwrap()
        .restrict_nonzero(p(&[2, 1]))
        .unwrap();
    let answer = context.solve_rational_inequality(&f, Op::Greater).unwrap();
    assert_eq!(
        answer.intervals(),
        &[
            interval(Neg, finite(-2), false, false),
            interval(finite(-2), finite(1), false, false),
            interval(finite(1), Pos, false, false),
        ],
    );
    assert_eq!(answer.excluded_zero_sets().len(), 2);
}

#[test]
fn irrational_quadratic_boundaries_are_ordered_without_floats() {
    use RealBound::{NegativeInfinity as Neg, PositiveInfinity as Pos};
    let ctx = EvaluationContext::default();
    let f = RationalFunction::from_polynomial(p(&[-2, 0, 1]));
    let solution = ctx.solve_rational_inequality(&f, Op::Less).unwrap();
    assert_eq!(solution.intervals().len(), 1);
    let bound = &solution.intervals()[0];
    assert!(!bound.lower_closed && !bound.upper_closed);
    let roots = ctx.algebraic_roots(&p(&[-2, 0, 1])).unwrap();
    let mathematishia_kernel::AlgebraicRootSet::Finite(values) = roots.roots() else {
        panic!("expected finite quadratic roots");
    };
    assert_eq!(bound.lower, RealBound::Finite(values[0].clone()));
    assert_eq!(bound.upper, RealBound::Finite(values[1].clone()));
    let denominator = p(&[-3, 0, 1]);
    let f = ctx
        .rational_function(&p(&[-2, 0, 1]), &denominator)
        .unwrap();
    let answer = ctx.solve_rational_inequality(&f, Op::Greater).unwrap();
    let numerator_roots = ctx.algebraic_roots(&p(&[-2, 0, 1])).unwrap();
    let denominator_roots = ctx.algebraic_roots(&denominator).unwrap();
    let (
        mathematishia_kernel::AlgebraicRootSet::Finite(numerator_values),
        mathematishia_kernel::AlgebraicRootSet::Finite(denominator_values),
    ) = (numerator_roots.roots(), denominator_roots.roots())
    else {
        panic!("expected real roots");
    };
    assert_eq!(
        answer.intervals(),
        &[
            interval(
                Neg,
                RealBound::Finite(denominator_values[0].clone()),
                false,
                false,
            ),
            interval(
                RealBound::Finite(numerator_values[0].clone()),
                RealBound::Finite(numerator_values[1].clone()),
                false,
                false,
            ),
            interval(
                RealBound::Finite(denominator_values[1].clone()),
                Pos,
                false,
                false,
            ),
        ]
    );
    assert_eq!(verify_inequality_result(&ctx, &answer), Ok(true));
}

#[test]
fn shared_algebraic_zeros_are_deduplicated_and_excluded() {
    use RealBound::{NegativeInfinity as Neg, PositiveInfinity as Pos};
    let ctx = EvaluationContext::default();
    let f = ctx
        .rational_function(&p(&[-2, 0, 1]), &p(&[-2, 0, 1]))
        .unwrap();
    let result = ctx.solve_rational_inequality(&f, Op::Greater).unwrap();
    assert_eq!(result.critical_points().len(), 2);
    assert!(
        result
            .critical_points()
            .iter()
            .all(|item| item.is_excluded())
    );
    assert_eq!(result.intervals().len(), 3);
    assert!(matches!(result.intervals()[0].lower, Neg));
    assert!(matches!(result.intervals()[2].upper, Pos));
}

#[test]
fn unsupported_and_resource_exhaustion_are_typed() {
    let ctx = EvaluationContext::default();
    assert_eq!(
        ctx.solve_rational_inequality(
            &RationalFunction::from_polynomial(p(&[1, 0, 0, 1])),
            Op::Greater
        ),
        Err(InequalityError::UnsupportedDegree),
    );
    let f = RationalFunction::from_polynomial(p(&[1]))
        .restrict_nonzero(p(&[1, 0, 0, 1]))
        .unwrap();
    assert_eq!(
        ctx.solve_rational_inequality(&f, Op::Greater),
        Err(InequalityError::UnsupportedDegree),
    );
    let tiny = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        solve_rational_inequality(
            &tiny,
            &RationalFunction::from_polynomial(p(&[1])),
            Op::Greater
        ),
        Err(InequalityError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        ))),
    );
    let small = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        small
            .solve_rational_inequality(&RationalFunction::from_polynomial(p(&[1, 0, 1])), Op::Less),
        Err(InequalityError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::PolynomialDegree
        ))),
    );
    let few_conditions = EvaluationContext::new(ResourceLimits {
        max_rational_conditions: 0,
        ..ResourceLimits::default()
    });
    let excluded = RationalFunction::from_polynomial(p(&[1]))
        .restrict_nonzero(p(&[-1, 1]))
        .unwrap();
    assert_eq!(
        few_conditions.solve_rational_inequality(&excluded, Op::Greater),
        Err(InequalityError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::RationalConditions
        ))),
    );
}

#[test]
fn exhaustive_small_integer_linear_and_quadratic_signs() {
    let ctx = EvaluationContext::default();
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                let poly = p(&[a, b, c]);
                let function = RationalFunction::from_polynomial(poly.clone());
                for relation in [Op::Less, Op::LessOrEqual, Op::Greater, Op::GreaterOrEqual] {
                    let result = ctx.solve_rational_inequality(&function, relation).unwrap();
                    assert_eq!(verify_inequality_result(&ctx, &result), Ok(true));
                }
            }
        }
    }
}

#[test]
fn constant_nonzero_denominators_preserve_sign() {
    use RealBound::{NegativeInfinity as Neg, PositiveInfinity as Pos};
    assert_eq!(
        solve(&[1], &[-2], Op::Less),
        vec![interval(Neg, Pos, false, false)]
    );
    assert_eq!(
        solve(&[-1, 1], &[-1], Op::LessOrEqual),
        vec![interval(finite(1), Pos, true, false)]
    );
}

#[test]
fn sign_chart_matches_direct_exact_rational_evaluation() {
    let context = EvaluationContext::default();
    let contains = |interval: &RealInterval, x: &Rational| {
        let left = match &interval.lower {
            RealBound::NegativeInfinity => true,
            RealBound::Finite(ExactQuadraticRoot::Rational(value)) => {
                x > value || (interval.lower_closed && x == value)
            }
            _ => panic!("expected rational lower boundary"),
        };
        let right = match &interval.upper {
            RealBound::PositiveInfinity => true,
            RealBound::Finite(ExactQuadraticRoot::Rational(value)) => {
                x < value || (interval.upper_closed && x == value)
            }
            _ => panic!("expected rational upper boundary"),
        };
        left && right
    };
    for a in -1..=1 {
        for b in -1..=1 {
            for c in -1..=1 {
                for d in -1..=1 {
                    let numerator = p(&[a * b, -a - b, 1]);
                    let denominator = p(&[c * d, -c - d, 1]);
                    let function = context.rational_function(&numerator, &denominator).unwrap();
                    for operator in [
                        Op::Less,
                        Op::LessOrEqual,
                        Op::Greater,
                        Op::GreaterOrEqual,
                    ] {
                        let answer = context
                            .solve_rational_inequality(&function, operator)
                            .unwrap();
                        for sample in -3..=3 {
                            let x = q(sample, 1);
                            let actual = function.evaluate(&x);
                            let expected = actual.is_ok_and(|value| match operator {
                                Op::Less => value < q(0, 1),
                                Op::LessOrEqual => value <= q(0, 1),
                                Op::Greater => value > q(0, 1),
                                Op::GreaterOrEqual => value >= q(0, 1),
                            });
                            assert_eq!(
                                answer.intervals().iter().any(|item| contains(item, &x)),
                                expected,
                                "n={numerator:?}, d={denominator:?}, x={sample}, op={operator:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
