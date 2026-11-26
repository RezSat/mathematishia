use mathema::{Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    AbsoluteAffine, AbsoluteEquation, AbsoluteEquationError, AbsoluteExpression, AbsoluteInterval,
    AbsoluteSign, EvaluationContext, EvaluationError, ResourceLimitKind, ResourceLimits,
    solve_absolute_equation, verify_absolute_equation,
};

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn affine(slope: i64, constant: i64) -> AbsoluteAffine {
    AbsoluteAffine {
        slope: q(slope, 1),
        constant: q(constant, 1),
    }
}

fn expr(slope: i64, constant: i64, layers: &[(i64, i64)]) -> AbsoluteExpression {
    AbsoluteExpression {
        variable: Symbol::new("x").unwrap(),
        base: affine(slope, constant),
        layers: layers.iter().map(|&(a, b)| affine(a, b)).collect(),
    }
}

fn equation(left: AbsoluteExpression, right: AbsoluteExpression) -> AbsoluteEquation {
    AbsoluteEquation {
        domain: ScalarDomain::Real,
        left,
        right,
    }
}

fn point(value: Rational) -> AbsoluteInterval {
    AbsoluteInterval {
        lower: Some(value.clone()),
        upper: Some(value),
    }
}

fn points(values: &[i64]) -> Vec<AbsoluteInterval> {
    values.iter().map(|&value| point(q(value, 1))).collect()
}

fn assert_solutions(equation: &AbsoluteEquation, expected: Vec<AbsoluteInterval>) {
    let context = EvaluationContext::default();
    assert_eq!(equation.domain, ScalarDomain::Real);
    let result = solve_absolute_equation(&context, equation).unwrap();
    assert_eq!(result.solutions, expected);
    assert_eq!(
        verify_absolute_equation(&context, equation, &result),
        Ok(true)
    );
    assert_eq!(solve_absolute_equation(&context, equation), Ok(result));
}

#[test]
fn common_absolute_value_questions() {
    let abs = [(1, 0)];
    assert_solutions(
        &equation(expr(1, 0, &abs), expr(0, 3, &[])),
        points(&[-3, 3]),
    );
    assert_solutions(
        &equation(expr(2, -5, &abs), expr(0, 7, &[])),
        points(&[-1, 6]),
    );
    assert_solutions(
        &equation(expr(1, -1, &abs), expr(1, 3, &abs)),
        points(&[-1]),
    );
    assert_solutions(
        &equation(expr(1, 0, &[(1, -2), (1, 0)]), expr(0, 1, &[])),
        points(&[-3, -1, 1, 3]),
    );
    let mut half = expr(0, 0, &[]);
    half.base.constant = q(1, 2);
    assert_solutions(
        &equation(expr(3, 1, &abs), half),
        vec![point(q(-1, 2)), point(q(-1, 6))],
    );
    assert_solutions(
        &equation(expr(1, 0, &abs), expr(1, 1, &abs)),
        vec![point(q(-1, 2))],
    );
}

#[test]
fn zero_negative_targets_and_constant_cases() {
    let abs = [(1, 0)];
    assert_solutions(&equation(expr(1, 0, &abs), expr(0, -3, &[])), vec![]);
    assert_solutions(&equation(expr(1, 0, &abs), expr(0, 0, &[])), points(&[0]));
    assert_solutions(&equation(expr(0, -3, &abs), expr(0, 4, &[])), vec![]);
    assert_solutions(
        &equation(expr(0, -3, &abs), expr(0, 3, &[])),
        vec![AbsoluteInterval {
            lower: None,
            upper: None,
        }],
    );
    assert_solutions(
        &equation(expr(1, 0, &[(0, 2), (1, 0)]), expr(0, 2, &[])),
        vec![AbsoluteInterval {
            lower: None,
            upper: None,
        }],
    );
}

#[test]
fn identities_and_closed_rays() {
    let abs = [(1, 0)];
    assert_solutions(
        &equation(expr(1, 0, &abs), expr(-1, 0, &abs)),
        vec![AbsoluteInterval {
            lower: None,
            upper: None,
        }],
    );
    assert_solutions(
        &equation(expr(1, 0, &abs), expr(1, 0, &[])),
        vec![AbsoluteInterval {
            lower: Some(q(0, 1)),
            upper: None,
        }],
    );
    assert_solutions(
        &equation(expr(1, 0, &abs), expr(-1, 0, &[])),
        vec![AbsoluteInterval {
            lower: None,
            upper: Some(q(0, 1)),
        }],
    );
    assert_solutions(
        &equation(expr(1, 0, &[(1, -2), (1, 0)]), expr(1, -2, &[])),
        vec![AbsoluteInterval {
            lower: Some(q(2, 1)),
            upper: None,
        }],
    );
}

#[test]
fn bounded_interval_and_disjoint_rays() {
    assert_solutions(
        &equation(expr(1, 0, &[(1, -1), (1, 0)]), expr(1, 0, &[(-1, 1)])),
        vec![AbsoluteInterval {
            lower: Some(q(-1, 1)),
            upper: Some(q(1, 1)),
        }],
    );
    assert_solutions(
        &equation(expr(1, 0, &[(1, -1), (1, 0)]), expr(1, 0, &[(1, -1)])),
        vec![
            AbsoluteInterval {
                lower: None,
                upper: Some(q(-1, 1)),
            },
            AbsoluteInterval {
                lower: Some(q(1, 1)),
                upper: None,
            },
        ],
    );
}

fn evaluate(expression: &AbsoluteExpression, x: &Rational) -> Rational {
    let mut value = &expression.base.slope * x + expression.base.constant.clone();
    for layer in &expression.layers {
        if value.numerator() < &Integer::zero() {
            value = -value;
        }
        value = &layer.slope * &value + layer.constant.clone();
    }
    value
}

#[test]
fn exact_substitution_and_side_swap_agree_on_small_integer_grid() {
    let context = EvaluationContext::default();
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -1..=1 {
                let source = equation(expr(a, b, &[(1, -1), (1, 0)]), expr(c, 1, &[(1, 0)]));
                let result = solve_absolute_equation(&context, &source).unwrap();
                let reversed = equation(source.right.clone(), source.left.clone());
                let swapped = solve_absolute_equation(&context, &reversed).unwrap();
                assert_eq!(result.solutions, swapped.solutions);
                for n in -12..=12 {
                    let x = q(n, 3);
                    let included = result.solutions.iter().any(|interval| {
                        interval.lower.as_ref().is_none_or(|lower| lower <= &x)
                            && interval.upper.as_ref().is_none_or(|upper| &x <= upper)
                    });
                    assert_eq!(
                        included,
                        evaluate(&source.left, &x) == evaluate(&source.right, &x)
                    );
                }
            }
        }
    }
}

#[test]
fn forged_certificates_fail_against_original_equation() {
    let context = EvaluationContext::default();
    let source = equation(expr(1, 0, &[(1, 0)]), expr(0, 3, &[]));
    let result = solve_absolute_equation(&context, &source).unwrap();
    let mut forged = result.clone();
    forged.cases.pop();
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );

    let mut forged = result.clone();
    forged.cases[0].interval.upper = Some(q(-1, 1));
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );

    let mut forged = result.clone();
    forged.cases[0].signs[0] = AbsoluteSign::Nonnegative;
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );

    let mut forged = result.clone();
    forged.cases[0].solution = None;
    forged.solutions.remove(0);
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );

    let mut forged = result.clone();
    forged.cases[0].solution = Some(point(q(-2, 1)));
    forged.solutions[0] = point(q(-2, 1));
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );

    let mut forged = result.clone();
    forged.solutions.reverse();
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );

    let negative_target = equation(expr(1, 0, &[(1, 0)]), expr(0, -3, &[]));
    assert_eq!(
        verify_absolute_equation(&context, &negative_target, &result),
        Ok(false)
    );
}

#[test]
fn forged_identity_and_overlapping_partition_fail() {
    let context = EvaluationContext::default();
    let source = equation(expr(1, 0, &[(1, 0)]), expr(1, 0, &[]));
    let result = solve_absolute_equation(&context, &source).unwrap();
    let mut forged = result.clone();
    forged.cases[1].solution = Some(point(q(0, 1)));
    forged.solutions = points(&[0]);
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );

    let mut forged = result;
    forged.cases[0].interval.upper = Some(q(1, 1));
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );
}

#[test]
fn unsupported_domains_and_variables_are_not_empty_answers() {
    let context = EvaluationContext::default();
    let source = equation(expr(1, 0, &[(1, 0)]), expr(0, 3, &[]));
    for domain in [
        ScalarDomain::Complex,
        ScalarDomain::Rational,
        ScalarDomain::Integer,
    ] {
        let mut input = source.clone();
        input.domain = domain;
        assert_eq!(
            solve_absolute_equation(&context, &input),
            Err(AbsoluteEquationError::UnsupportedDomain)
        );
    }
    let mut input = source;
    input.right.variable = Symbol::new("y").unwrap();
    assert_eq!(
        solve_absolute_equation(&context, &input),
        Err(AbsoluteEquationError::MultipleVariables)
    );
    assert_eq!(
        solve_absolute_equation(&context, &equation(expr(1, 0, &[]), expr(0, 1, &[]))),
        Err(AbsoluteEquationError::MissingAbsoluteValue)
    );
}

#[test]
fn resource_limits_apply_to_generation_and_untrusted_evidence() {
    let source = equation(expr(1, 0, &[(1, 0)]), expr(0, 3, &[]));
    let context = EvaluationContext::default();
    let result = solve_absolute_equation(&context, &source).unwrap();
    for (limits, kind) in [
        (
            ResourceLimits {
                max_work_units: 0,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::WorkUnits,
        ),
        (
            ResourceLimits {
                max_expression_nodes: 2,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::ExpressionNodes,
        ),
        (
            ResourceLimits {
                max_expression_depth: 1,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::ExpressionDepth,
        ),
        (
            ResourceLimits {
                max_scalar_bits: 1,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::ScalarBits,
        ),
        (
            ResourceLimits {
                max_intermediate_scalar_bits: 1,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::IntermediateScalarBits,
        ),
    ] {
        let context = EvaluationContext::new(limits);
        let error = AbsoluteEquationError::Evaluation(EvaluationError::Limit(kind));
        assert_eq!(solve_absolute_equation(&context, &source), Err(error));
        assert_eq!(
            verify_absolute_equation(&context, &source, &result),
            Err(error)
        );
    }
    let over_limit = equation(expr(1, 0, &[(1, 0); 9]), expr(0, 1, &[]));
    assert_eq!(
        solve_absolute_equation(&context, &over_limit),
        Err(AbsoluteEquationError::CaseLimit)
    );
    let mut forged = result;
    forged.cases.push(forged.cases[0].clone());
    assert_eq!(
        verify_absolute_equation(&context, &source, &forged),
        Ok(false)
    );
}
