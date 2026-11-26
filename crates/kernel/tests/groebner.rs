use mathema::{Condition, DomainExpression, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    GroebnerError, IdealMembership, MultivariateError, MultivariateLimits, MultivariatePolynomial,
    divide_multivariate, groebner_basis, groebner_basis_from_domain_expressions, ideal_membership,
    ideal_membership_from_domain_expressions, verify_groebner_basis,
    verify_groebner_basis_from_domain_expressions, verify_ideal_membership,
    verify_ideal_membership_from_domain_expressions,
};

fn s(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}
fn q(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}
fn vars() -> Vec<Symbol> {
    vec![s("x"), s("y")]
}
fn p(terms: &[(i32, i32, i64)]) -> MultivariatePolynomial {
    MultivariatePolynomial::try_new(
        vars(),
        terms.iter().map(|&(x, y, c)| (vec![x, y], q(c))),
        MultivariateLimits::default(),
    )
    .unwrap()
}

fn check_query(
    generators: &[MultivariatePolynomial],
    target: &MultivariatePolynomial,
    expected: bool,
) {
    let limits = MultivariateLimits::default();
    let variables = target.variables();
    let certificate = groebner_basis(variables, generators, limits).unwrap();
    assert!(verify_groebner_basis(variables, generators, &certificate, limits).unwrap());
    let result = ideal_membership(variables, generators, &certificate, target, limits).unwrap();
    assert_eq!(matches!(&result, IdealMembership::Member(_)), expected);
    assert!(
        verify_ideal_membership(variables, generators, &certificate, target, &result, limits)
        .unwrap()
    );
}

#[test]
fn eliminate_x_from_xy_minus_one_and_y_squared_minus_one() {
    let limits = MultivariateLimits::default();
    let generators = [p(&[(1, 1, 1), (0, 0, -1)]), p(&[(0, 2, 1), (0, 0, -1)])];
    let target = p(&[(1, 0, 1), (0, 1, -1)]);
    assert_eq!(divide_multivariate(&target, &generators, limits).unwrap().remainder(), &target);
    let certificate = groebner_basis(&vars(), &generators, limits).unwrap();
    assert_eq!(certificate.basis.len(), 3);
    assert_eq!(certificate.basis[2], target);
    assert_eq!(certificate.pairs.len(), 3);
    assert_eq!(certificate.pairs[0].lcm, vec![1, 2]);
    assert_eq!(certificate.representations[2], vec![p(&[(0, 1, 1)]), p(&[(1, 0, -1)])]);
    assert!(verify_groebner_basis(&vars(), &generators, &certificate, limits).unwrap());
    assert_eq!(certificate, groebner_basis(&[s("y"), s("x")], &generators, limits).unwrap());
    check_query(&generators, &target, true);
    check_query(&generators, &p(&[(0, 1, 1), (0, 0, -1)]), false);
}

#[test]
fn lexicographic_elimination_produces_y_cubed_minus_one() {
    let generators = [p(&[(2, 0, 1), (0, 1, -1)]), p(&[(1, 1, 1), (0, 0, -1)])];
    check_query(&generators, &p(&[(0, 3, 1), (0, 0, -1)]), true);
    check_query(&generators, &p(&[(0, 1, 1), (0, 0, -1)]), false);
}

#[test]
fn membership_is_not_radical_membership() {
    let generators = [p(&[(2, 0, 1)]), p(&[(0, 2, 1)])];
    check_query(&generators, &p(&[(1, 1, 1)]), false);
    check_query(&generators, &p(&[(3, 1, 2), (0, 2, -3)]), true);
}

#[test]
fn inconsistent_generators_produce_unit_ideal() {
    let generators = [p(&[(1, 0, 1)]), p(&[(1, 0, 1), (0, 0, -1)])];
    check_query(&generators, &p(&[(0, 0, 1)]), true);
    check_query(&generators, &p(&[(1, 2, 7), (0, 0, 11)]), true);
}

#[test]
fn three_variables_and_rational_coefficients() {
    let limits = MultivariateLimits::default();
    let variables = [s("x"), s("y"), s("z")];
    let x = Expr::from(s("x"));
    let y = Expr::from(s("y"));
    let z = Expr::from(s("z"));
    let minus = |e: Expr| Expr::mul([Expr::from(-1), e]);
    let half = Expr::from(Rational::new(Integer::one(), Integer::from(2)).unwrap());
    let convert = |e: Expr| MultivariatePolynomial::try_from_expr(&e, &variables, limits).unwrap();
    let generators = [
        convert(Expr::add([x.clone(), minus(y.clone())])),
        convert(Expr::add([y, minus(z.clone())])),
        convert(Expr::add([z, minus(half.clone())])),
    ];
    check_query(&generators, &convert(Expr::add([x.clone(), minus(half)])), true);
    check_query(&generators, &convert(x), false);
}

#[test]
fn zero_ideal_zero_generators_and_duplicate_generators() {
    let zero = p(&[]);
    let x = p(&[(1, 0, 1)]);
    check_query(&[], &zero, true);
    check_query(&[], &x, false);
    check_query(&[zero.clone(), zero.clone()], &x, false);
    check_query(&[zero, x.clone(), x.clone()], &x, true);
}

#[test]
fn an_ideal_containing_the_generators_is_not_necessarily_the_same_ideal() {
    let limits = MultivariateLimits::default();
    let x = p(&[(1, 0, 1)]);
    let one = p(&[(0, 0, 1)]);
    let mut certificate = groebner_basis(&vars(), std::slice::from_ref(&one), limits).unwrap();
    // The forged basis [1] reduces x to zero, and has no S-pairs, but 1 is not in (x).
    let reduction = &mut certificate.generator_reductions[0];
    reduction.quotients[0] = x.clone();
    assert!(
        !verify_groebner_basis(&vars(), std::slice::from_ref(&x), &certificate, limits).unwrap()
    );
    assert_eq!(
        ideal_membership(&vars(), &[x], &certificate, &one, limits),
        Err(GroebnerError::InvalidCertificate)
    );
}

#[test]
fn missing_altered_and_reordered_pairs_are_rejected() {
    let limits = MultivariateLimits::default();
    let generators = [p(&[(1, 1, 1), (0, 0, -1)]), p(&[(0, 2, 1), (0, 0, -1)])];
    let certificate = groebner_basis(&vars(), &generators, limits).unwrap();
    let mut forged = certificate.clone();
    forged.pairs.pop();
    assert!(!verify_groebner_basis(&vars(), &generators, &forged, limits).unwrap());
    let mut forged = certificate.clone();
    forged.pairs[0].lcm[0] += 1;
    assert!(!verify_groebner_basis(&vars(), &generators, &forged, limits).unwrap());
    let mut forged = certificate.clone();
    forged.pairs.swap(0, 1);
    assert!(!verify_groebner_basis(&vars(), &generators, &forged, limits).unwrap());
    let mut forged = certificate.clone();
    forged.pairs[0].reduction.quotients[0] = p(&[(0, 0, 99)]);
    assert!(!verify_groebner_basis(&vars(), &generators, &forged, limits).unwrap());
    let mut forged = certificate.clone();
    forged.generator_reductions[0].steps += 1;
    assert!(!verify_groebner_basis(&vars(), &generators, &forged, limits).unwrap());
    let mut forged = certificate;
    forged.representations[2][0] = p(&[]);
    assert!(!verify_groebner_basis(&vars(), &generators, &forged, limits).unwrap());
}

#[test]
fn recomposition_alone_does_not_prove_ordered_reduction() {
    let limits = MultivariateLimits::default();
    let x = p(&[(1, 0, 1)]);
    let generators = [x.clone(), x.clone()];
    let certificate = groebner_basis(&vars(), &generators, limits).unwrap();
    let IdealMembership::Member(mut reduction) =
        ideal_membership(&vars(), &generators, &certificate, &x, limits).unwrap()
    else {
        panic!("x is a generator");
    };
    reduction.quotients.swap(0, 1);
    assert!(
        !verify_ideal_membership(
            &vars(),
            &generators,
            &certificate,
            &x,
            &IdealMembership::Member(reduction),
            limits
        )
        .unwrap()
    );
    let target = p(&[(0, 1, 1)]);
    let IdealMembership::NonMember(reduction) =
        ideal_membership(&vars(), &generators, &certificate, &target, limits).unwrap()
    else {
        panic!("y is outside (x)");
    };
    assert!(
        !verify_ideal_membership(
            &vars(),
            &generators,
            &certificate,
            &target,
            &IdealMembership::Member(reduction),
            limits
        )
        .unwrap()
    );
}

#[test]
fn rejects_wrong_ring_even_for_the_zero_ideal() {
    let limits = MultivariateLimits::default();
    let certificate = groebner_basis(&vars(), &[], limits).unwrap();
    assert!(!verify_groebner_basis(&[s("z")], &[], &certificate, limits).unwrap());
    assert_eq!(
        groebner_basis(&[s("z")], &[p(&[])], limits),
        Err(GroebnerError::Polynomial(MultivariateError::VariableMismatch))
    );
    assert_eq!(
        groebner_basis(&[s("x"), s("x")], &[], limits),
        Err(GroebnerError::Polynomial(MultivariateError::DuplicateVariable))
    );
}

#[test]
fn domain_input_and_target_are_bound_to_original_conditions() {
    let limits = MultivariateLimits::default();
    let x = Expr::from(s("x"));
    let zero = DomainExpression::from_expr(Expr::from(0), 32).unwrap();
    let reciprocal = DomainExpression::from_expr(Expr::pow(x.clone(), Expr::from(-1)), 32).unwrap();
    let historical = DomainExpression::multiply([zero, reciprocal], 16).unwrap();
    assert_eq!(historical.expression(), &Expr::from(0));
    assert_eq!(historical.conditions(), &[Condition::NonZero(x.clone())]);
    let generator = DomainExpression::from_expr(x.clone(), 32).unwrap();
    let target = DomainExpression::from_expr(Expr::pow(x, Expr::from(2)), 32).unwrap();
    let generators = [generator];
    let certificate = groebner_basis_from_domain_expressions(&vars(), &generators, limits).unwrap();
    assert!(
        verify_groebner_basis_from_domain_expressions(&vars(), &generators, &certificate, limits)
        .unwrap()
    );
    let result =
        ideal_membership_from_domain_expressions(&vars(), &generators, &certificate, &target, limits)
            .unwrap();
    assert!(matches!(&result, IdealMembership::Member(_)));
    assert!(
        verify_ideal_membership_from_domain_expressions(
            &vars(),
            &generators,
            &certificate,
            &target,
            &result,
            limits
        )
        .unwrap()
    );
    assert_eq!(
        groebner_basis_from_domain_expressions(&vars(), std::slice::from_ref(&historical), limits),
        Err(GroebnerError::RestrictedDomain { input: 0 })
    );
    assert_eq!(
        ideal_membership_from_domain_expressions(
            &vars(),
            &generators,
            &certificate,
            &historical,
            limits
        ),
        Err(GroebnerError::RestrictedDomain { input: 1 })
    );
    assert_eq!(
        verify_ideal_membership_from_domain_expressions(
            &vars(),
            &generators,
            &certificate,
            &historical,
            &result,
            limits
        ),
        Err(GroebnerError::RestrictedDomain { input: 1 })
    );
}

#[test]
fn unsupported_expressions_are_not_polynomialized() {
    let limits = MultivariateLimits::default();
    let x = Expr::from(s("x"));
    let analytic =
        DomainExpression::from_expr_in_domain(Expr::sin(x), ScalarDomain::Real, 100, 10).unwrap();
    let other = DomainExpression::from_expr(Expr::from(s("z")), 32).unwrap();
    for expr in [analytic, other] {
        assert!(
            matches!(
                groebner_basis_from_domain_expressions(&vars(), &[expr], limits),
                Err(GroebnerError::Expression(_))
            )
        );
    }
}

#[test]
fn resource_limits_cover_intermediate_lcm_and_aggregate_work() {
    let limits = MultivariateLimits::default();
    let generators = [p(&[(2, 0, 1)]), p(&[(0, 2, 1)])];
    let low_degree = MultivariateLimits { max_total_degree: 2, ..limits };
    assert_eq!(
        groebner_basis(&vars(), &generators, low_degree),
        Err(GroebnerError::Polynomial(MultivariateError::TotalDegreeLimit))
    );
    let low_work = MultivariateLimits { max_work_units: 12, ..limits };
    assert_eq!(
        groebner_basis(&vars(), &generators, low_work),
        Err(GroebnerError::Polynomial(MultivariateError::WorkLimit))
    );
    let certificate = groebner_basis(&vars(), &generators, limits).unwrap();
    assert!(
        matches!(
            verify_groebner_basis(&vars(), &generators, &certificate, low_work),
            Err(GroebnerError::Polynomial(MultivariateError::WorkLimit))
        )
    );
    assert_eq!(
        groebner_basis(
            &vars(),
            &[p(&[(1, 0, 9)])],
            MultivariateLimits { max_scalar_bits: 3, ..limits }
        ),
        Err(GroebnerError::Polynomial(MultivariateError::ScalarBitLimit))
    );
    assert_eq!(
        groebner_basis(
            &vars(),
            &[p(&[(1, 0, 1), (0, 1, 1)])],
            MultivariateLimits { max_terms: 1, ..limits }
        ),
        Err(GroebnerError::Polynomial(MultivariateError::TermLimit))
    );
    assert_eq!(
        groebner_basis(&[s("a"), s("b"), s("c"), s("d")], &[], limits),
        Err(GroebnerError::Polynomial(MultivariateError::VariableLimit))
    );
    let too_many = vec![p(&[]); 9];
    assert_eq!(
        groebner_basis(&vars(), &too_many, limits),
        Err(GroebnerError::GeneratorLimit)
    );
}

#[test]
fn a_generating_set_with_an_unresolved_s_pair_cannot_certify_nonmembership() {
    let limits = MultivariateLimits::default();
    let generators = [p(&[(1, 1, 1), (0, 0, -1)]), p(&[(0, 2, 1), (0, 0, -1)])];
    let mut forged = groebner_basis(&vars(), &generators, limits).unwrap();
    forged.basis.truncate(2);
    forged.representations.truncate(2);
    for reduction in &mut forged.generator_reductions {
        reduction.quotients.truncate(2);
    }
    forged.pairs.truncate(1);
    forged.pairs[0].reduction.quotients.truncate(2);
    assert!(!verify_groebner_basis(&vars(), &generators, &forged, limits).unwrap());
    assert_eq!(
        ideal_membership(&vars(), &generators, &forged, &p(&[(1, 0, 1), (0, 1, -1)]), limits),
        Err(GroebnerError::InvalidCertificate)
    );
}

#[test]
fn intermediate_scalar_growth_and_oversized_evidence_are_refused() {
    let limits = MultivariateLimits::default();
    let generators = [p(&[(1, 0, 7)]), p(&[(0, 1, 1)])];
    assert_eq!(
        groebner_basis(&vars(), &generators, MultivariateLimits { max_scalar_bits: 3, ..limits }),
        Err(GroebnerError::Polynomial(MultivariateError::ScalarBitLimit))
    );
    let mut forged = groebner_basis(&vars(), &generators, limits).unwrap();
    forged.basis = vec![generators[0].clone(); 13];
    assert_eq!(
        verify_groebner_basis(&vars(), &generators, &forged, limits),
        Err(GroebnerError::BasisLimit)
    );
}

#[test]
fn nonmonic_generators_produce_exact_rational_remainders() {
    let limits = MultivariateLimits::default();
    let generators = [p(&[(1, 0, 2), (0, 1, -1)]), p(&[(0, 1, 3), (0, 0, -1)])];
    check_query(&generators, &p(&[(1, 0, 6), (0, 0, -1)]), true);
    let target = p(&[(1, 0, 1)]);
    let certificate = groebner_basis(&vars(), &generators, limits).unwrap();
    let IdealMembership::NonMember(reduction) =
        ideal_membership(&vars(), &generators, &certificate, &target, limits).unwrap()
    else {
        panic!("x has a nonzero rational normal form");
    };
    assert_eq!(
        reduction.remainder,
        MultivariatePolynomial::constant(
            vars(),
            Rational::new(Integer::one(), Integer::from(6)).unwrap(),
            limits,
        )
        .unwrap()
    );
}
