use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, EvaluationError, RationalPolynomial, ResourceLimitKind, ResourceLimits,
    SturmCountCertificate, SturmCountError, SturmSign, SturmVerificationError, verify_sturm_count,
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

fn certified(
    ctx: &EvaluationContext,
    polynomial: &RationalPolynomial,
    a: &Rational,
    b: &Rational,
    expected_count: usize,
) -> SturmCountCertificate {
    let first = ctx.count_real_roots_sturm(polynomial, a, b).unwrap();
    assert_eq!(first.root_count(), expected_count);
    assert_eq!(verify_sturm_count(ctx, polynomial, a, b, &first), Ok(()));
    assert_eq!(
        ctx.count_real_roots_sturm(polynomial, a, b),
        Ok(first.clone())
    );
    first
}

#[test]
fn exact_sturm_chain_and_variations_for_two_irrational_roots() {
    let ctx = EvaluationContext::default();
    let polynomial = p(&[-2, 0, 1]);
    let result = certified(&ctx, &polynomial, &r(-2), &r(2), 2);
    assert_eq!(result.polynomial(), &polynomial);
    assert_eq!(result.lower(), &r(-2));
    assert_eq!(result.upper(), &r(2));
    assert_eq!(result.sequence(), &[p(&[-2, 0, 1]), p(&[0, 2]), p(&[2])]);
    assert_eq!(result.divisions().len(), 2);
    assert_eq!(
        result.divisions()[0].quotient(),
        &RationalPolynomial::new(vec![r(0), q(1, 2)])
    );
    assert_eq!(result.divisions()[0].remainder(), &p(&[-2]));
    assert_eq!(result.divisions()[1].quotient(), &p(&[0, 1]));
    assert!(result.divisions()[1].remainder().is_zero());
    assert_eq!(
        result.lower_signs(),
        &[SturmSign::Positive, SturmSign::Negative, SturmSign::Positive]
    );
    assert_eq!(
        result.upper_signs(),
        &[SturmSign::Positive, SturmSign::Positive, SturmSign::Positive]
    );
    assert_eq!(result.lower_variations(), 2);
    assert_eq!(result.upper_variations(), 0);
}

#[test]
fn same_sign_interval_endpoints_can_enclose_two_roots() {
    let ctx = EvaluationContext::default();
    certified(&ctx, &p(&[-1, 0, 1]), &r(-2), &r(2), 2);
    certified(&ctx, &p(&[-1, 0, 1]), &r(2), &r(4), 0);
    certified(&ctx, &p(&[-2, 0, 1]), &r(-2), &r(0), 1);
    certified(&ctx, &p(&[-2, 0, 1]), &r(0), &r(2), 1);
}

#[test]
fn zero_signs_of_internal_sturm_polynomials_are_skipped() {
    let ctx = EvaluationContext::default();
    let result = certified(&ctx, &p(&[-2, 0, 1]), &r(0), &r(2), 1);
    assert_eq!(
        result.lower_signs(),
        &[SturmSign::Negative, SturmSign::Zero, SturmSign::Positive]
    );
    assert_eq!(result.lower_variations(), 1);
    assert_eq!(result.upper_variations(), 0);
}

#[test]
fn several_rational_roots_and_polynomials_with_one_real_root() {
    let ctx = EvaluationContext::default();
    // x * (x - 2) * (x + 1) has three distinct real roots.
    let cubic = p(&[0, -2, -1, 1]);
    certified(&ctx, &cubic, &r(-2), &r(3), 3);
    certified(&ctx, &cubic, &r(-2), &q(-1, 2), 1);
    certified(&ctx, &cubic, &q(-1, 2), &r(1), 1);
    certified(&ctx, &cubic, &r(1), &r(3), 1);
    // x^3 + x has only the rational real root zero.
    certified(&ctx, &p(&[0, 1, 0, 1]), &r(-10), &r(10), 1);
    certified(&ctx, &p(&[0, 1, 0, 1]), &r(1), &r(10), 0);
}

#[test]
fn nonreal_quadratics_nonzero_constants_and_negative_units() {
    let ctx = EvaluationContext::default();
    for polynomial in [p(&[1, 0, 1]), p(&[2, 0, 3]), p(&[7]), p(&[-7])] {
        let result = certified(&ctx, &polynomial, &r(-2), &r(3), 0);
        if polynomial.degree() == Some(0) {
            assert_eq!(result.sequence(), &[polynomial]);
            assert!(result.divisions().is_empty());
            assert_eq!(result.lower_variations(), 0);
            assert_eq!(result.upper_variations(), 0);
        }
    }
    // Scaling the original by a nonzero negative constant does not change roots.
    certified(&ctx, &p(&[2, 0, -1]), &r(-2), &r(2), 2);
}

#[test]
fn near_rational_endpoints_and_multiple_nearby_roots() {
    let ctx = EvaluationContext::default();
    // Roots 1/1024 and 1/512, with exact rational coefficients.
    let polynomial = p(&[1, -1536, 524288]);
    certified(&ctx, &polynomial, &r(0), &q(1, 100), 2);
    certified(&ctx, &polynomial, &q(1, 10000), &q(1, 1000), 1);
    certified(&ctx, &polynomial, &q(1, 1000), &q(1, 500), 1);
    certified(&ctx, &p(&[-1, 3]), &q(333332, 1000000), &q(333334, 1000000), 1);
    certified(&ctx, &p(&[-1, 3]), &q(1, 2), &q(2, 3), 0);
}

#[test]
fn repeated_roots_are_explicitly_unsupported_even_when_outside_interval() {
    let ctx = EvaluationContext::default();
    for polynomial in [p(&[1, -2, 1]), p(&[1, 0, 2, 0, 1]), p(&[0, 0, 1])] {
        assert_eq!(
            ctx.count_real_roots_sturm(&polynomial, &r(2), &r(3)),
            Err(SturmCountError::NonSquareFree)
        );
    }
}

#[test]
fn zero_polynomial_invalid_order_and_exact_endpoint_roots_are_rejected() {
    let ctx = EvaluationContext::default();
    assert_eq!(
        ctx.count_real_roots_sturm(&p(&[]), &r(-1), &r(1)),
        Err(SturmCountError::ZeroPolynomial)
    );
    for (a, b) in [(r(0), r(0)), (r(3), r(1))] {
        assert_eq!(
            ctx.count_real_roots_sturm(&p(&[-1, 1]), &a, &b),
            Err(SturmCountError::InvalidInterval)
        );
    }
    for (a, b) in [(r(1), r(2)), (r(0), r(1)), (r(-2), r(1))] {
        assert_eq!(
            ctx.count_real_roots_sturm(&p(&[-1, 0, 1]), &a, &b),
            Err(SturmCountError::RootAtEndpoint)
        );
    }
}

#[test]
fn large_exact_coefficients_and_huge_endpoints() {
    let ctx = EvaluationContext::default();
    let huge: Integer = "100000000000000000000000000000000000000000000000000000000"
        .parse()
        .unwrap();
    let bound = Rational::from(huge.clone());
    let minus_bound = Rational::from(-&huge);
    certified(&ctx, &p(&[0, 1]), &minus_bound, &bound, 1);
    let positive = RationalPolynomial::new(vec![Rational::from(huge.clone()), r(0), r(1)]);
    certified(&ctx, &positive, &r(-2), &r(2), 0);
    let big_leading = RationalPolynomial::new(vec![r(-1), r(0), Rational::from(huge)]);
    certified(&ctx, &big_leading, &r(-1), &r(1), 2);
}

#[test]
fn deterministic_degree_scalar_intermediate_and_work_refusals() {
    let polynomial = p(&[-2, 0, 1]);
    let ctx = EvaluationContext::new(ResourceLimits {
        max_polynomial_degree: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.count_real_roots_sturm(&polynomial, &r(-2), &r(2)),
        Err(SturmCountError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::PolynomialDegree
        )))
    );

    let ctx = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.count_real_roots_sturm(&p(&[9, 1]), &r(-1), &r(1)),
        Err(SturmCountError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::ScalarBits
        )))
    );

    let ctx = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.count_real_roots_sturm(&polynomial, &r(-2), &r(2)),
        Err(SturmCountError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        )))
    );

    let ctx = EvaluationContext::new(ResourceLimits {
        max_intermediate_scalar_bits: 2,
        ..ResourceLimits::default()
    });
    assert_eq!(
        ctx.count_real_roots_sturm(&polynomial, &r(-2), &r(2)),
        Err(SturmCountError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::IntermediateScalarBits
        )))
    );
}

#[test]
fn verifier_rejects_wrong_original_bounds_and_fails_closed_on_small_budget() {
    let ctx = EvaluationContext::default();
    let polynomial = p(&[-2, 0, 1]);
    let certificate = certified(&ctx, &polynomial, &r(-2), &r(2), 2);
    assert_eq!(
        verify_sturm_count(&ctx, &p(&[-3, 0, 1]), &r(-2), &r(2), &certificate),
        Err(SturmVerificationError::InvalidEvidence)
    );
    assert_eq!(
        verify_sturm_count(&ctx, &polynomial, &r(-3), &r(2), &certificate),
        Err(SturmVerificationError::InvalidEvidence)
    );
    assert_eq!(
        verify_sturm_count(&ctx, &polynomial, &r(-2), &r(3), &certificate),
        Err(SturmVerificationError::InvalidEvidence)
    );
    let limited = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        verify_sturm_count(&limited, &polynomial, &r(-2), &r(2), &certificate),
        Err(SturmVerificationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        )))
    );
}
