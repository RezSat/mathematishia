use mathema::{Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    CertifiedSolutionSet, EvaluationContext, EvaluationError, ExactQuadraticRoot,
    PolynomialRootMultiplicity, RationalExpressionError, RationalFunction, RationalPolynomial,
    ResourceLimitKind, ResourceLimits, SolutionCertificateError, certify_solution_set,
    verify_solution_set,
};

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}

fn poly(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        values
            .iter()
            .map(|&v| Rational::from(Integer::from(v)))
            .collect(),
    )
}

fn domain(expr: Expr) -> DomainExpression {
    DomainExpression::from_expr(expr, 128).unwrap()
}

fn polynomial(values: &[i64]) -> DomainExpression {
    domain(poly(values).to_expr(&x()))
}

fn root(value: i64, multiplicity: usize) -> PolynomialRootMultiplicity {
    PolynomialRootMultiplicity::new(
        ExactQuadraticRoot::Rational(Rational::from(Integer::from(value))),
        multiplicity,
    )
}

#[test]
fn completeness_distinguishes_rational_real_and_complex_domains() {
    let ctx = EvaluationContext::default();
    let zero = polynomial(&[]);
    for (source, domain, count) in [
        (polynomial(&[-2, 0, 1]), ScalarDomain::Rational, 0),
        (polynomial(&[-2, 0, 1]), ScalarDomain::Real, 2),
        (polynomial(&[1, 0, 1]), ScalarDomain::Real, 0),
        (polynomial(&[1, 0, 1]), ScalarDomain::Complex, 2),
    ] {
        let result = certify_solution_set(&ctx, &source, &zero, &x(), domain, &[]).unwrap();
        match result.solutions {
            CertifiedSolutionSet::None => assert_eq!(count, 0),
            CertifiedSolutionSet::Finite(ref roots) => {
                assert_eq!(roots.len(), count);
                assert!(roots.iter().all(|r| r.multiplicity() == 1));
            }
            _ => panic!("expected complete set"),
        }
        assert!(verify_solution_set(&ctx, &source, &zero, &x(), domain, &result).unwrap());
    }
}

#[test]
fn original_operands_are_equal_at_exact_algebraic_roots() {
    let ctx = EvaluationContext::default();
    let symbol = Expr::from(x());
    let left = domain(Expr::pow(symbol.clone(), (-1).into()));
    let right = domain(Expr::mul([symbol, Expr::pow(2.into(), (-1).into())]));
    let result = certify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Real, &[]).unwrap();
    let CertifiedSolutionSet::Finite(ref roots) = result.solutions else {
        panic!("expected finite set")
    };
    assert_eq!(roots.len(), 2);
    assert_eq!(result.residual.excluded_zero_sets(), &[poly(&[0, 1])]);
    assert!(verify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Real, &result).unwrap());
}

#[test]
fn removed_rational_and_algebraic_poles_are_never_solutions() {
    let ctx = EvaluationContext::default();
    let symbol = Expr::from(x());
    let left = domain(Expr::mul([
        Expr::add([Expr::pow(symbol.clone(), 2.into()), (-1).into()]),
        Expr::pow(Expr::add([symbol, (-1).into()]), (-1).into()),
    ]));
    let right = polynomial(&[2]);
    let result =
        certify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Rational, &[]).unwrap();
    assert_eq!(result.solutions, CertifiedSolutionSet::None);
    assert_eq!(result.residual.excluded_zero_sets(), &[poly(&[-1, 1])]);

    let quadratic = poly(&[-2, 0, 1]).to_expr(&x());
    let restricted = DomainExpression::with_conditions(
        quadratic.clone(),
        [Condition::NonZero(quadratic)],
        128,
        8,
    )
    .unwrap();
    let zero = polynomial(&[]);
    let result =
        certify_solution_set(&ctx, &restricted, &zero, &x(), ScalarDomain::Complex, &[]).unwrap();
    assert_eq!(result.solutions, CertifiedSolutionSet::None);
    assert_eq!(result.residual.excluded_zero_sets(), &[poly(&[-2, 0, 1])]);
}

#[test]
fn identity_preserves_conditions_and_nonzero_constant_is_empty() {
    let ctx = EvaluationContext::default();
    let zero = polynomial(&[]);
    let restricted =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(Expr::from(x()))], 128, 8)
            .unwrap();
    let identity =
        certify_solution_set(&ctx, &restricted, &zero, &x(), ScalarDomain::Complex, &[]).unwrap();
    assert_eq!(identity.solutions, CertifiedSolutionSet::AllAdmissible);
    assert_eq!(identity.residual.excluded_zero_sets(), &[poly(&[0, 1])]);
    assert!(
        verify_solution_set(
            &ctx,
            &restricted,
            &zero,
            &x(),
            ScalarDomain::Complex,
            &identity
        )
        .unwrap()
    );
    let constant = certify_solution_set(
        &ctx,
        &polynomial(&[1]),
        &zero,
        &x(),
        ScalarDomain::Complex,
        &[],
    )
    .unwrap();
    assert_eq!(constant.solutions, CertifiedSolutionSet::None);
}

#[test]
fn repeated_roots_and_supported_factor_products_have_exact_multiplicity() {
    let ctx = EvaluationContext::default();
    let zero = polynomial(&[]);
    let repeated = polynomial(&[1, -2, 1]);
    let result =
        certify_solution_set(&ctx, &repeated, &zero, &x(), ScalarDomain::Real, &[]).unwrap();
    assert_eq!(
        result.solutions,
        CertifiedSolutionSet::Finite(vec![root(1, 2)])
    );

    // (x-1)^4(x+2) = x^5-2x^4-2x^3+8x^2-7x+2
    let source = polynomial(&[2, -7, 8, -2, -2, 1]);
    let result = certify_solution_set(
        &ctx,
        &source,
        &zero,
        &x(),
        ScalarDomain::Real,
        &[poly(&[-1, 1])],
    )
    .unwrap();
    assert_eq!(
        result.solutions,
        CertifiedSolutionSet::Finite(vec![root(-2, 1), root(1, 4)])
    );
    assert!(verify_solution_set(&ctx, &source, &zero, &x(), ScalarDomain::Real, &result).unwrap());
}

#[test]
fn higher_degree_cofactor_remains_partial_even_without_known_roots() {
    let ctx = EvaluationContext::default();
    let zero = polynomial(&[]);
    let cubic = polynomial(&[-2, 0, 0, 1]);
    let unresolved =
        certify_solution_set(&ctx, &cubic, &zero, &x(), ScalarDomain::Rational, &[]).unwrap();
    assert_eq!(
        unresolved.solutions,
        CertifiedSolutionSet::Partial {
            roots: vec![],
            unresolved_cofactor: poly(&[-2, 0, 0, 1])
        }
    );
    let product = polynomial(&[2, -2, 0, -1, 1]);
    let result = certify_solution_set(
        &ctx,
        &product,
        &zero,
        &x(),
        ScalarDomain::Real,
        &[poly(&[-1, 1])],
    )
    .unwrap();
    assert_eq!(
        result.solutions,
        CertifiedSolutionSet::Partial {
            roots: vec![root(1, 1)],
            unresolved_cofactor: poly(&[-2, 0, 0, 1])
        }
    );
    assert!(verify_solution_set(&ctx, &product, &zero, &x(), ScalarDomain::Real, &result).unwrap());
}

#[test]
fn verification_rejects_altered_source_domain_roots_multiplicity_and_links() {
    let ctx = EvaluationContext::default();
    let left = DomainExpression::with_conditions(
        poly(&[2, -3, 1]).to_expr(&x()),
        [Condition::NonZero(poly(&[-1, 1]).to_expr(&x()))],
        128,
        8,
    )
    .unwrap();
    let right = polynomial(&[]);
    let domain = ScalarDomain::Real;
    let result = certify_solution_set(&ctx, &left, &right, &x(), domain, &[]).unwrap();
    assert_eq!(
        result.solutions,
        CertifiedSolutionSet::Finite(vec![root(2, 1)])
    );
    assert!(
        !verify_solution_set(
            &ctx,
            &polynomial(&[2, -3, 1]),
            &right,
            &x(),
            domain,
            &result
        )
        .unwrap()
    );
    assert!(
        !verify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Complex, &result).unwrap()
    );

    for roots in [
        vec![root(1, 1), root(2, 1)],
        vec![root(2, 2)],
        vec![root(3, 1)],
        vec![],
    ] {
        let mut forged = result.clone();
        forged.solutions = CertifiedSolutionSet::Finite(roots);
        assert!(!verify_solution_set(&ctx, &left, &right, &x(), domain, &forged).unwrap());
    }
    let mut forged = result.clone();
    forged.residual = RationalFunction::from_polynomial(poly(&[2, -3, 1]));
    assert!(!verify_solution_set(&ctx, &left, &right, &x(), domain, &forged).unwrap());
    let mut forged = result.clone();
    forged.factors[0] = ctx.algebraic_roots(&poly(&[-3, 1])).unwrap();
    assert!(!verify_solution_set(&ctx, &left, &right, &x(), domain, &forged).unwrap());
    let mut forged = result;
    forged.factors.clear();
    forged.solutions = CertifiedSolutionSet::None;
    assert!(!verify_solution_set(&ctx, &left, &right, &x(), domain, &forged).unwrap());
}

#[test]
fn incomplete_proof_cannot_claim_completeness() {
    let ctx = EvaluationContext::default();
    let left = polynomial(&[2, -2, 0, -1, 1]);
    let right = polynomial(&[]);
    let mut result = certify_solution_set(
        &ctx,
        &left,
        &right,
        &x(),
        ScalarDomain::Complex,
        &[poly(&[-1, 1])],
    )
    .unwrap();
    result.solutions = CertifiedSolutionSet::Finite(vec![root(1, 1)]);
    assert!(
        !verify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Complex, &result).unwrap()
    );
    result.solutions = CertifiedSolutionSet::Partial {
        roots: vec![root(1, 1)],
        unresolved_cofactor: poly(&[1, 0, 0, 1]),
    };
    assert!(
        !verify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Complex, &result).unwrap()
    );
}

#[test]
fn refuses_unsupported_conditions_invalid_factors_and_exhausted_budgets() {
    let ctx = EvaluationContext::default();
    let left = polynomial(&[-2, 0, 1]);
    let zero = polynomial(&[]);
    let restricted =
        DomainExpression::with_conditions(0.into(), [Condition::Positive(Expr::from(x()))], 128, 8)
            .unwrap();
    assert_eq!(
        certify_solution_set(&ctx, &restricted, &zero, &x(), ScalarDomain::Real, &[]),
        Err(SolutionCertificateError::Expression(
            RationalExpressionError::UnsupportedCondition
        ))
    );
    assert_eq!(
        certify_solution_set(&ctx, &left, &zero, &x(), ScalarDomain::Integer, &[]),
        Err(SolutionCertificateError::UnsupportedDomain)
    );
    assert_eq!(
        certify_solution_set(
            &ctx,
            &left,
            &zero,
            &x(),
            ScalarDomain::Real,
            &[poly(&[-3, 1])]
        ),
        Err(SolutionCertificateError::IncorrectCertificate)
    );
    for (limits, kind) in [
        (
            ResourceLimits {
                max_polynomial_degree: 1,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::PolynomialDegree,
        ),
        (
            ResourceLimits {
                max_work_units: 0,
                ..ResourceLimits::default()
            },
            ResourceLimitKind::WorkUnits,
        ),
    ] {
        let limited = EvaluationContext::new(limits);
        let result = certify_solution_set(&limited, &left, &zero, &x(), ScalarDomain::Real, &[]);
        // The shared converter reports its own exponent ceiling before polynomial admission.
        assert!(
            result
                == Err(SolutionCertificateError::Evaluation(
                    EvaluationError::Limit(kind)
                ))
                || (kind == ResourceLimitKind::PolynomialDegree
                    && result
                        == Err(SolutionCertificateError::Expression(
                            RationalExpressionError::ExponentLimitExceeded
                        )))
        );
    }
}

#[test]
fn rejects_unnecessary_partial_claim_for_supported_quadratic() {
    let ctx = EvaluationContext::default();
    let left = polynomial(&[-2, 0, 1]);
    let right = polynomial(&[]);
    let mut result =
        certify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Real, &[]).unwrap();
    result.factors.clear();
    result.solutions = CertifiedSolutionSet::Partial {
        roots: vec![],
        unresolved_cofactor: poly(&[-2, 0, 1]),
    };
    assert!(!verify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Real, &result).unwrap());
}

#[test]
fn complex_conjugate_multiplicity_and_original_nonzero_conditions() {
    let ctx = EvaluationContext::default();
    let left = polynomial(&[1, 0, 2, 0, 1]);
    let zero = polynomial(&[]);
    let result = certify_solution_set(
        &ctx,
        &left,
        &zero,
        &x(),
        ScalarDomain::Complex,
        &[poly(&[1, 0, 1])],
    )
    .unwrap();
    let CertifiedSolutionSet::Finite(ref roots) = result.solutions else {
        panic!("expected complete complex pair")
    };
    assert_eq!(roots.len(), 2);
    assert!(roots.iter().all(|r| r.multiplicity() == 2));
    assert!(verify_solution_set(&ctx, &left, &zero, &x(), ScalarDomain::Complex, &result).unwrap());

    let limited = EvaluationContext::new(ResourceLimits {
        max_work_units: 1,
        ..ResourceLimits::default()
    });
    assert_eq!(
        verify_solution_set(&limited, &left, &zero, &x(), ScalarDomain::Complex, &result),
        Err(SolutionCertificateError::Evaluation(
            EvaluationError::Limit(ResourceLimitKind::WorkUnits)
        ))
    );
}

fn quotient(numerator: &[i64], denominator: &[i64]) -> DomainExpression {
    polynomial(numerator)
        .divide(polynomial(denominator), 16)
        .unwrap()
}

#[test]
fn canceled_linear_root_is_removed_over_every_admitted_domain() {
    let ctx = EvaluationContext::default();
    let left = quotient(&[-1, 0, 1], &[-1, 1]);
    for scalar in [
        ScalarDomain::Rational,
        ScalarDomain::Real,
        ScalarDomain::Complex,
    ] {
        for (constant, expected) in [
            (2, CertifiedSolutionSet::None),
            (3, CertifiedSolutionSet::Finite(vec![root(2, 1)])),
        ] {
            let right = polynomial(&[constant]);
            let result = certify_solution_set(&ctx, &left, &right, &x(), scalar, &[]).unwrap();
            assert_eq!(result.solutions, expected);
            assert_eq!(result.residual.excluded_zero_sets(), &[poly(&[-1, 1])]);
            assert!(verify_solution_set(&ctx, &left, &right, &x(), scalar, &result).unwrap());
        }
    }
}

#[test]
fn reciprocal_has_no_zero_despite_its_pole() {
    let ctx = EvaluationContext::default();
    let left = quotient(&[1], &[-1, 1]);
    let right = polynomial(&[]);
    let scalar = ScalarDomain::Real;
    let result = certify_solution_set(&ctx, &left, &right, &x(), scalar, &[]).unwrap();
    assert_eq!(result.solutions, CertifiedSolutionSet::None);
    assert_eq!(result.residual.excluded_zero_sets(), &[poly(&[-1, 1])]);
    assert!(verify_solution_set(&ctx, &left, &right, &x(), scalar, &result).unwrap());
}

#[test]
fn zero_times_inverse_is_an_identity_only_on_its_original_domain() {
    let ctx = EvaluationContext::default();
    let left =
        DomainExpression::multiply([polynomial(&[]), quotient(&[1], &[0, 1])], 16).unwrap();
    let right = polynomial(&[]);
    let scalar = ScalarDomain::Real;
    let result = certify_solution_set(&ctx, &left, &right, &x(), scalar, &[]).unwrap();
    assert_eq!(result.solutions, CertifiedSolutionSet::AllAdmissible);
    assert_eq!(result.residual.excluded_zero_sets(), &[poly(&[0, 1])]);
    assert!(verify_solution_set(&ctx, &left, &right, &x(), scalar, &result).unwrap());
}

#[test]
fn cancellation_across_operands_keeps_both_independent_holes() {
    let ctx = EvaluationContext::default();
    let left = quotient(&[0, -1, 1], &[-1, 1]);
    let right = quotient(&[0, 1, 1], &[1, 1]);
    let scalar = ScalarDomain::Real;
    let result = certify_solution_set(&ctx, &left, &right, &x(), scalar, &[]).unwrap();
    assert_eq!(result.solutions, CertifiedSolutionSet::AllAdmissible);
    assert_eq!(
        result.residual.excluded_zero_sets(),
        &[poly(&[-1, 1]), poly(&[1, 1])]
    );
    assert!(verify_solution_set(&ctx, &left, &right, &x(), scalar, &result).unwrap());
}

#[test]
fn quadratic_roots_are_excluded_by_a_polynomial_on_the_other_operand() {
    let ctx = EvaluationContext::default();
    let left = polynomial(&[-2, 0, 1]);
    let right = DomainExpression::with_conditions(
        0.into(),
        [Condition::NonZero(poly(&[4, 0, -4, 0, 1]).to_expr(&x()))],
        128,
        8,
    )
    .unwrap();
    for scalar in [ScalarDomain::Real, ScalarDomain::Complex] {
        let result = certify_solution_set(&ctx, &left, &right, &x(), scalar, &[]).unwrap();
        assert_eq!(result.solutions, CertifiedSolutionSet::None);
        assert!(verify_solution_set(&ctx, &left, &right, &x(), scalar, &result).unwrap());
    }
}

#[test]
fn bound_real_source_can_be_certified_in_its_own_domain() {
    let ctx = EvaluationContext::default();
    let left = DomainExpression::from_expr_in_domain(
        poly(&[-2, 0, 1]).to_expr(&x()),
        ScalarDomain::Real,
        128,
        8,
    )
    .unwrap();
    let right = polynomial(&[]);
    let result =
        certify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Real, &[]).unwrap();
    let CertifiedSolutionSet::Finite(ref roots) = result.solutions else {
        panic!("expected exact real pair");
    };
    assert_eq!(roots.len(), 2);
    assert!(verify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Real, &result).unwrap());
}

#[test]
fn refuses_erased_complex_history_when_solving_over_real_or_rational_scalars() {
    let ctx = EvaluationContext::default();
    let logarithm =
        DomainExpression::from_expr_in_domain(Expr::log((-1).into()), ScalarDomain::Complex, 128, 8)
            .unwrap();
    let erased = DomainExpression::multiply([polynomial(&[]), logarithm], 16).unwrap();
    let zero = polynomial(&[]);
    for scalar in [ScalarDomain::Real, ScalarDomain::Rational] {
        assert_eq!(
            certify_solution_set(&ctx, &erased, &zero, &x(), scalar, &[]),
            Err(SolutionCertificateError::UnsupportedDomain)
        );
        assert_eq!(
            certify_solution_set(&ctx, &zero, &erased, &x(), scalar, &[]),
            Err(SolutionCertificateError::UnsupportedDomain)
        );
    }
}

#[test]
fn verifier_refuses_relabeling_a_bound_certificate() {
    let ctx = EvaluationContext::default();
    let left = DomainExpression::from_expr_in_domain(
        poly(&[-2, 0, 1]).to_expr(&x()),
        ScalarDomain::Real,
        128,
        8,
    )
    .unwrap();
    let right = polynomial(&[]);
    let mut result =
        certify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Real, &[]).unwrap();
    result.domain = ScalarDomain::Complex;
    assert_eq!(
        verify_solution_set(&ctx, &left, &right, &x(), ScalarDomain::Complex, &result),
        Err(SolutionCertificateError::UnsupportedDomain)
    );
}

#[test]
fn verifier_rejects_identity_with_either_original_hole_removed() {
    let ctx = EvaluationContext::default();
    let left = quotient(&[0, -1, 1], &[-1, 1]);
    let right = quotient(&[0, 1, 1], &[1, 1]);
    let scalar = ScalarDomain::Real;
    let result = certify_solution_set(&ctx, &left, &right, &x(), scalar, &[]).unwrap();
    for exclusion in [poly(&[-1, 1]), poly(&[1, 1])] {
        let mut forged = result.clone();
        forged.residual = RationalFunction::from_polynomial(poly(&[]))
            .restrict_nonzero(exclusion)
            .unwrap();
        assert!(!verify_solution_set(&ctx, &left, &right, &x(), scalar, &forged).unwrap());
    }
}

#[test]
fn verifier_rejects_empty_identity_and_all_admissible_nonzero_constant() {
    let ctx = EvaluationContext::default();
    let zero = polynomial(&[]);
    let one = polynomial(&[1]);
    let scalar = ScalarDomain::Real;
    for (left, false_claim) in [
        (&zero, CertifiedSolutionSet::None),
        (&one, CertifiedSolutionSet::AllAdmissible),
    ] {
        let mut forged = certify_solution_set(&ctx, left, &zero, &x(), scalar, &[]).unwrap();
        forged.solutions = false_claim;
        assert!(!verify_solution_set(&ctx, left, &zero, &x(), scalar, &forged).unwrap());
    }
}

#[test]
fn refuses_branch_selective_nonpolynomial_restrictions() {
    let ctx = EvaluationContext::default();
    let left = DomainExpression::with_conditions(
        poly(&[-2, 0, 1]).to_expr(&x()),
        [Condition::Positive(Expr::from(x()))],
        128,
        8,
    )
    .unwrap();
    assert_eq!(
        certify_solution_set(&ctx, &left, &polynomial(&[]), &x(), ScalarDomain::Real, &[]),
        Err(SolutionCertificateError::Expression(
            RationalExpressionError::UnsupportedCondition
        ))
    );
}
