use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalPolynomial, ResourceLimitKind, ResourceLimits,
    RootIntervalOutcome, RootRefinementError, RootRefinementVerificationError,
    verify_root_refinement,
};

fn r(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn p(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(coefficients.iter().copied().map(r).collect())
}

fn verify(
    ctx: &EvaluationContext,
    polynomial: &RationalPolynomial,
    low: &Rational,
    high: &Rational,
    width: &Rational,
    steps: usize,
) -> mathematishia_kernel::RootRefinement {
    let result = ctx
        .refine_real_root_bracket(polynomial, low, high, width, steps)
        .unwrap();
    assert_eq!(
        verify_root_refinement(ctx, polynomial, low, high, width, steps, &result),
        Ok(())
    );
    assert_eq!(
        ctx.refine_real_root_bracket(polynomial, low, high, width, steps),
        Ok(result.clone())
    );
    result
}

#[test]
fn irrational_sign_change_and_exact_dyadic_endpoints() {
    let ctx = EvaluationContext::default();
    let poly = p(&[-2, 0, 1]);
    let result = verify(&ctx, &poly, &r(1), &r(2), &q(1, 16), 4);
    assert_eq!(result.bisections(), 4);
    assert!(result.precision_met());
    assert_eq!(
        result.outcome(),
        &RootIntervalOutcome::SignChange {
            lower: q(11, 8),
            upper: q(23, 16),
        }
    );

    let result = verify(&ctx, &poly, &r(1), &r(2), &q(1, 1024), 2);
    assert_eq!(result.bisections(), 2);
    assert!(!result.precision_met());
    assert!(matches!(
        result.outcome(),
        RootIntervalOutcome::SignChange { .. }
    ));

    let result = verify(&ctx, &p(&[-1, 1]), &r(1), &r(4), &q(1, 10), 10);
    assert_eq!(result.outcome(), &RootIntervalOutcome::ExactRoot(r(1)));
    assert_eq!(result.bisections(), 0);

    let result = verify(&ctx, &p(&[-4, 1]), &r(1), &r(4), &q(1, 10), 10);
    assert_eq!(result.outcome(), &RootIntervalOutcome::ExactRoot(r(4)));
    assert_eq!(result.bisections(), 0);

    let result = verify(&ctx, &p(&[-2, 1]), &r(1), &r(3), &q(1, 10), 10);
    assert_eq!(result.outcome(), &RootIntervalOutcome::ExactRoot(r(2)));
    assert_eq!(result.bisections(), 1);
    assert!(result.precision_met());
}

#[test]
fn even_multiplicity_and_same_sign_are_unresolved_not_absence_proofs() {
    let ctx = EvaluationContext::default();
    for poly in [p(&[1, -2, 1]), p(&[1, 0, 1]), p(&[1]), p(&[-1, 0, 1])] {
        let result = verify(&ctx, &poly, &r(-2), &r(2), &q(1, 100), 32);
        assert_eq!(
            result.outcome(),
            &RootIntervalOutcome::Unresolved {
                lower: r(-2),
                upper: r(2),
            }
        );
        assert_eq!(result.bisections(), 0);
        assert!(!result.precision_met());
    }
}

#[test]
fn odd_multiplicity_certifies_existence_not_uniqueness() {
    let ctx = EvaluationContext::default();
    let cubic = p(&[-2, 6, -6, 2]);
    let result = verify(&ctx, &cubic, &r(0), &r(3), &q(1, 4), 4);
    assert!(matches!(
        result.outcome(),
        RootIntervalOutcome::SignChange { .. }
    ));
    assert!(result.precision_met());
    // A product with three distinct real roots also carries only an existence claim.
    let result = verify(&ctx, &p(&[0, -4, 0, 1]), &r(-3), &r(3), &q(1, 32), 0);
    assert!(matches!(
        result.outcome(),
        RootIntervalOutcome::SignChange { .. }
    ));
}

#[test]
fn zero_step_budget_and_already_narrow_brackets() {
    let ctx = EvaluationContext::default();
    let poly = p(&[-2, 0, 1]);
    let result = verify(&ctx, &poly, &r(1), &r(2), &q(1, 8), 0);
    assert_eq!(result.bisections(), 0);
    assert!(!result.precision_met());
    let result = verify(&ctx, &poly, &r(1), &r(2), &r(1), 0);
    assert_eq!(result.bisections(), 0);
    assert!(result.precision_met());
}

#[test]
fn invalid_inputs_and_zero_polynomial() {
    let ctx = EvaluationContext::default();
    let poly = p(&[1, 1]);
    assert_eq!(
        ctx.refine_real_root_bracket(&p(&[]), &r(0), &r(1), &r(1), 5),
        Err(RootRefinementError::ZeroPolynomial)
    );
    for (a, b) in [(r(2), r(1)), (r(2), r(2))] {
        assert_eq!(
            ctx.refine_real_root_bracket(&poly, &a, &b, &r(1), 5),
            Err(RootRefinementError::InvalidInterval)
        );
    }
    for width in [r(0), r(-1)] {
        assert_eq!(
            ctx.refine_real_root_bracket(&poly, &r(0), &r(1), &width, 5),
            Err(RootRefinementError::NonpositiveTargetWidth)
        );
    }
}

#[test]
fn huge_exact_endpoints_are_supported_subject_to_explicit_limits() {
    let ctx = EvaluationContext::default();
    let big: Integer = "100000000000000000000000000000000000000000000000000"
        .parse()
        .unwrap();
    let lower = Rational::from(-&big);
    let upper = Rational::from(big);
    let result = verify(&ctx, &p(&[0, 1]), &lower, &upper, &q(1, 100), 1);
    assert_eq!(result.outcome(), &RootIntervalOutcome::ExactRoot(r(0)));
    assert_eq!(result.bisections(), 1);
    let limited = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 16,
        ..ResourceLimits::default()
    });
    assert_eq!(
        limited.refine_real_root_bracket(&p(&[0, 1]), &lower, &upper, &r(1), 1),
        Err(RootRefinementError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::ScalarBits
        )))
    );
}

#[test]
fn work_and_intermediate_limits_are_not_silent_inconclusive_results() {
    let poly = p(&[-2, 0, 1]);
    let work_limited = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        work_limited.refine_real_root_bracket(&poly, &r(1), &r(2), &q(1, 8), 8),
        Err(RootRefinementError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        )))
    );
    let bit_limited = EvaluationContext::new(ResourceLimits {
        max_intermediate_scalar_bits: 3,
        ..ResourceLimits::default()
    });
    assert_eq!(
        bit_limited.refine_real_root_bracket(&poly, &r(1), &r(2), &q(1, 256), 8),
        Err(RootRefinementError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::IntermediateScalarBits
        )))
    );
    let degree_limited = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        degree_limited.refine_real_root_bracket(&poly, &r(1), &r(2), &r(1), 1),
        Err(RootRefinementError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::PolynomialDegree
        )))
    );
}

#[test]
fn verifier_binds_polynomial_input_and_requested_budget() {
    let ctx = EvaluationContext::default();
    let original = p(&[-2, 0, 1]);
    let result = verify(&ctx, &original, &r(1), &r(2), &q(1, 16), 4);
    assert_eq!(
        verify_root_refinement(&ctx, &p(&[-3, 0, 1]), &r(1), &r(2), &q(1, 16), 4, &result),
        Err(RootRefinementVerificationError::InvalidEvidence)
    );
    assert_eq!(
        verify_root_refinement(&ctx, &original, &r(1), &r(2), &q(1, 4), 4, &result),
        Err(RootRefinementVerificationError::InvalidEvidence)
    );
    assert_eq!(
        verify_root_refinement(&ctx, &original, &r(1), &r(2), &q(1, 16), 3, &result),
        Err(RootRefinementVerificationError::InvalidEvidence)
    );
}
