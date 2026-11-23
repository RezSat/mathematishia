use mathema::{Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, ExactMatrix, LinearSystemSolution, MultivariateError,
    MultivariateExprError, MultivariateLimits, PolynomialExpansionOutcome,
    RationalEquationSolution, RationalFunction, RationalFunctionError, RationalPolynomial,
    ResourceLimitKind, ResourceLimits, expand_polynomial, verify_linear_system_vector,
    verify_matrix_inverse, verify_polynomial_expansion, verify_rational_equation,
};

fn q(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn poly(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(coefficients.iter().map(|&value| q(value, 1)).collect())
}

fn symbol(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

fn next(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 32
}

fn generated_polynomial(seed: &mut u64) -> RationalPolynomial {
    let count = (next(seed) % 5 + 1) as usize;
    RationalPolynomial::new(
        (0..count)
            .map(|_| q((next(seed) % 7) as i64 - 3, (next(seed) % 4 + 1) as i64))
            .collect(),
    )
}

fn independent_value(polynomial: &RationalPolynomial, at: &Rational) -> Rational {
    let mut sum = q(0, 1);
    let mut power = q(1, 1);
    for coefficient in polynomial.coefficients() {
        sum = &sum + &(coefficient * &power);
        power = &power * at;
    }
    sum
}

#[test]
fn seeded_exact_polynomial_ring_and_evaluation_identities() {
    let mut seed = 0x5a17_4c2d_902e_18abu64;
    for case in 0..96 {
        let a = generated_polynomial(&mut seed);
        let b = generated_polynomial(&mut seed);
        let c = generated_polynomial(&mut seed);
        let point = q(
            (next(&mut seed) % 9) as i64 - 4,
            (next(&mut seed) % 4 + 1) as i64,
        );
        let ab = a.multiply(&b);
        assert_eq!(a.add(&b), b.add(&a), "addition, case {case}");
        assert_eq!(ab, b.multiply(&a), "multiplication, case {case}");
        assert_eq!(
            a.add(&b).add(&c),
            a.add(&b.add(&c)),
            "add associativity, case {case}"
        );
        assert_eq!(
            ab.multiply(&c),
            a.multiply(&b.multiply(&c)),
            "associativity, case {case}"
        );
        assert_eq!(
            a.multiply(&b.add(&c)),
            ab.add(&a.multiply(&c)),
            "distributivity, case {case}"
        );
        assert_eq!(a.subtract(&a), RationalPolynomial::zero());
        assert_eq!(a.evaluate(&point), independent_value(&a, &point));
        assert_eq!(
            ab.evaluate(&point),
            &independent_value(&a, &point) * &independent_value(&b, &point)
        );
        assert_eq!(
            a.add(&b).evaluate(&point),
            &independent_value(&a, &point) + &independent_value(&b, &point)
        );
    }
}

#[test]
fn seeded_division_reconstructs_inputs_with_smaller_remainders() {
    let context = EvaluationContext::default();
    let mut seed = 0x39ed_2c61_a593_77b1u64;
    for case in 0..80 {
        let dividend = generated_polynomial(&mut seed);
        let mut divisor = generated_polynomial(&mut seed);
        if divisor.is_zero() {
            divisor = poly(&[1]);
        }
        let (quotient, remainder) = context.divide(&dividend, &divisor).unwrap();
        assert_eq!(
            divisor.multiply(&quotient).add(&remainder),
            dividend,
            "division reconstruction, case {case}"
        );
        assert!(
            remainder.is_zero() || remainder.degree() < divisor.degree(),
            "non-reduced remainder, case {case}"
        );
        for point in [q(-3, 2), q(0, 1), q(5, 3)] {
            let left = independent_value(&dividend, &point);
            let right = &(&independent_value(&divisor, &point)
                * &independent_value(&quotient, &point))
                + &independent_value(&remainder, &point);
            assert_eq!(left, right, "division evaluation, case {case}");
        }
        assert_eq!(
            context.divide(&dividend, &divisor).unwrap(),
            (quotient, remainder)
        );
    }
}

#[test]
fn removed_poles_remain_undefined_under_cancellation_and_arithmetic() {
    for excluded in -3..=3 {
        let hole = poly(&[-excluded, 1]);
        let numerator = poly(&[1, 2]).multiply(&hole);
        let canceled = RationalFunction::new(numerator, hole.clone()).unwrap();
        let filled = RationalFunction::from_polynomial(poly(&[1, 2]));
        assert!(canceled.algebraically_equivalent(&filled));
        assert_ne!(canceled, filled);
        assert_eq!(canceled.excluded_zero_sets(), &[hole]);
        assert_eq!(
            canceled.evaluate(&q(excluded, 1)),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        let doubled = canceled.add(&canceled).unwrap();
        let zero = canceled.subtract(&canceled).unwrap();
        for at in -4..=4 {
            let x = q(at, 1);
            if at == excluded {
                assert_eq!(
                    doubled.evaluate(&x),
                    Err(RationalFunctionError::UndefinedAtPoint)
                );
                assert_eq!(
                    zero.evaluate(&x),
                    Err(RationalFunctionError::UndefinedAtPoint)
                );
            } else {
                assert_eq!(canceled.evaluate(&x), Ok(q(1 + 2 * at, 1)));
                assert_eq!(doubled.evaluate(&x), Ok(q(2 + 4 * at, 1)));
                assert_eq!(zero.evaluate(&x), Ok(q(0, 1)));
            }
        }
    }
    assert_eq!(
        RationalFunction::new(poly(&[1]), RationalPolynomial::zero()),
        Err(RationalFunctionError::ZeroDenominator)
    );
    assert_eq!(
        RationalFunction::from_polynomial(poly(&[1]))
            .divide(&RationalFunction::from_polynomial(poly(&[]))),
        Err(RationalFunctionError::ZeroDivisor)
    );
}

#[test]
fn rational_linear_solutions_satisfy_original_equations() {
    let context = EvaluationContext::default();
    let zero = RationalFunction::from_polynomial(poly(&[]));
    for slope in -3..=3 {
        for constant in -3..=3 {
            let input = poly(&[constant, slope]);
            let left = RationalFunction::from_polynomial(input.clone());
            let result = context.solve_rational_equation(&left, &zero).unwrap();
            assert_eq!(verify_rational_equation(&result), Ok(()));
            match (slope, constant) {
                (0, 0) => assert_eq!(
                    result.solution(),
                    &RationalEquationSolution::AllAdmissibleRationals
                ),
                (0, _) => assert_eq!(result.solution(), &RationalEquationSolution::NoSolution),
                (_, _) => {
                    let root = q(-constant, slope);
                    assert_eq!(
                        result.solution(),
                        &RationalEquationSolution::Unique(root.clone())
                    );
                    assert_eq!(independent_value(&input, &root), q(0, 1));
                    assert_eq!(left.evaluate(&root), zero.evaluate(&root));
                }
            }
            assert_eq!(
                context.solve_rational_equation(&left, &zero).unwrap(),
                result
            );
        }
    }
}

#[test]
fn generated_two_by_two_matrices_match_independent_exact_adjugate() {
    let context = EvaluationContext::default();
    let mut seed = 0x63da_10bc_527e_13e7u64;
    for case in 0..80 {
        let [a, b, c, d] = std::array::from_fn(|_| (next(&mut seed) % 9) as i64 - 4);
        let matrix = ExactMatrix::new(
            &context,
            vec![vec![q(a, 1), q(b, 1)], vec![q(c, 1), q(d, 1)]],
        )
        .unwrap();
        let det = a * d - b * c;
        assert_eq!(
            matrix.determinant(&context).unwrap(),
            q(det, 1),
            "case {case}"
        );
        let inverse = matrix.inverse(&context).unwrap();
        assert_eq!(inverse.is_some(), det != 0);
        if let Some(inverse) = inverse {
            assert!(verify_matrix_inverse(&context, &matrix, &inverse).unwrap());
            let rhs = [q(1, 2), q(-2, 3)];
            let LinearSystemSolution::Unique(solution) = matrix.solve(&context, &rhs).unwrap()
            else {
                panic!("invertible matrix must have a unique solution, case {case}");
            };
            assert_eq!(solution[0], &q(d, 2 * det) + &q(2 * b, 3 * det));
            assert_eq!(solution[1], &q(-c, 2 * det) + &q(-2 * a, 3 * det));
            assert!(verify_linear_system_vector(&context, &matrix, &rhs, &solution).unwrap());
        }
    }
}

#[test]
fn power_expansion_preserves_independently_calculated_values() {
    let limits = MultivariateLimits::default();
    let x = Expr::from(symbol("x"));
    let ring = [symbol("x")];
    for a in -3..=3 {
        for b in -3..=3 {
            let base = Expr::add([
                Expr::mul([Expr::from(q(a, 2)), x.clone()]),
                Expr::from(q(b, 3)),
            ]);
            let source = DomainExpression::from_expr(Expr::pow(base, Expr::from(2)), 64).unwrap();
            let PolynomialExpansionOutcome::Exact(result) =
                expand_polynomial(&source, &ring, limits)
            else {
                panic!("small rational binomial must expand");
            };
            assert_eq!(
                verify_polynomial_expansion(&source, &result, &ring, limits),
                Ok(true)
            );
            for point in [q(-2, 1), q(0, 1), q(1, 2), q(3, 1)] {
                let base = &(&q(a, 2) * &point) + &q(b, 3);
                let expected = &base * &base;
                assert_eq!(result.polynomial().evaluate(&[point], limits), Ok(expected));
            }
            assert_eq!(
                expand_polynomial(&source, &ring, limits),
                PolynomialExpansionOutcome::Exact(result)
            );
        }
    }
}

#[test]
fn inherited_nonzero_obligations_are_not_erased_by_zero_multiplication() {
    let x = Expr::from(symbol("x"));
    let zero = DomainExpression::from_expr(Expr::from(0), 64).unwrap();
    let reciprocal = DomainExpression::from_expr(Expr::pow(x.clone(), Expr::from(-1)), 64).unwrap();
    let source = DomainExpression::multiply([zero, reciprocal], 32).unwrap();
    let PolynomialExpansionOutcome::Exact(result) =
        expand_polynomial(&source, &[symbol("x")], MultivariateLimits::default())
    else {
        panic!("zero with inherited hole must still be representable");
    };
    assert!(result.polynomial().is_zero());
    assert_eq!(result.expanded().conditions(), &[Condition::NonZero(x)]);
    assert_eq!(
        verify_polynomial_expansion(
            &source,
            &result,
            &[symbol("x")],
            MultivariateLimits::default()
        ),
        Ok(true)
    );
}

#[test]
fn unsupported_operations_and_tight_multivariate_budgets_refuse_without_partial_answers() {
    let x = Expr::from(symbol("x"));
    let y = Expr::from(symbol("y"));
    let vars = [symbol("x"), symbol("y")];
    let source =
        DomainExpression::from_expr(Expr::pow(Expr::add([x.clone(), y]), Expr::from(4)), 64)
            .unwrap();
    for limits in [
        MultivariateLimits {
            max_terms: 2,
            ..MultivariateLimits::default()
        },
        MultivariateLimits {
            max_work_units: 1,
            ..MultivariateLimits::default()
        },
        MultivariateLimits {
            max_total_degree: 2,
            ..MultivariateLimits::default()
        },
    ] {
        assert!(
            matches!(
                expand_polynomial(&source, &vars, limits),
                PolynomialExpansionOutcome::ResourceLimited(_)
            ),
            "tight limit must reject complete expansion: {limits:?}"
        );
    }
    let huge = DomainExpression::from_expr(Expr::from(1_000_000), 64).unwrap();
    assert!(matches!(
        expand_polynomial(
            &huge,
            &[],
            MultivariateLimits {
                max_scalar_bits: 2,
                ..MultivariateLimits::default()
            }
        ),
        PolynomialExpansionOutcome::ResourceLimited(MultivariateExprError::Polynomial(
            MultivariateError::ScalarBitLimit
        ))
    ));
    let analytic =
        DomainExpression::from_expr_in_domain(Expr::sin(x.clone()), ScalarDomain::Real, 64, 8)
            .unwrap();
    assert_eq!(
        expand_polynomial(&analytic, &vars, MultivariateLimits::default()),
        PolynomialExpansionOutcome::Unsupported(MultivariateExprError::UnsupportedAnalyticFunction)
    );
    assert_eq!(
        expand_polynomial(
            &DomainExpression::from_expr(Expr::pow(x, Expr::from(-2)), 64).unwrap(),
            &vars,
            MultivariateLimits::default()
        ),
        PolynomialExpansionOutcome::Unsupported(MultivariateExprError::NegativeExponent)
    );
}

#[test]
fn checked_kernel_enforces_work_scalar_degree_and_expression_depth_limits() {
    let sample = poly(&[1, 2, 3]);
    let point = q(2, 1);
    for (limits, expected) in [
        (
            ResourceLimits {
                max_work_units: 0,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::WorkUnits,
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
                max_polynomial_degree: 1,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::PolynomialDegree,
        ),
    ] {
        assert_eq!(
            EvaluationContext::new(limits).evaluate_polynomial(&sample, &point),
            Err(EvaluationError::Limit(expected))
        );
    }
    let x = Expr::from(symbol("x"));
    let nested = Expr::pow(Expr::add([x, Expr::from(1)]), Expr::from(2));
    let depth = EvaluationContext::new(ResourceLimits {
        max_expression_depth: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        depth.polynomial(&nested, &symbol("x")),
        Err(EvaluationError::Limit(ResourceLimitKind::ExpressionDepth))
    );
}
