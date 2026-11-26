use mathema::{Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    CertifiedSolutionSet, EvaluationContext, EvaluationError, ExactQuadraticRoot,
    PolynomialEquationError, PolynomialEquationResult, PolynomialRootMultiplicity, QuadraticBranch,
    RationalPolynomial, ResourceLimitKind, ResourceLimits, certify_solution_set,
    solve_polynomial_equation, verify_polynomial_equation,
};

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}

fn p(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients
            .iter()
            .map(|&value| Rational::from(Integer::from(value)))
            .collect(),
    )
}

fn rational(value: i64, multiplicity: usize) -> PolynomialRootMultiplicity {
    PolynomialRootMultiplicity::new(
        ExactQuadraticRoot::Rational(Rational::from(Integer::from(value))),
        multiplicity,
    )
}

fn algebraic(
    coefficients: &[i64],
    branch: QuadraticBranch,
    multiplicity: usize,
) -> PolynomialRootMultiplicity {
    PolynomialRootMultiplicity::new(
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial: p(coefficients),
            branch,
        },
        multiplicity,
    )
}

fn checked(original: &RationalPolynomial, domain: ScalarDomain) -> PolynomialEquationResult {
    let context = EvaluationContext::default();
    let result = solve_polynomial_equation(&context, original, &x(), domain).unwrap();
    assert!(verify_polynomial_equation(&context, original, &x(), domain, &result).unwrap());
    assert_eq!(&result.original, original);
    assert_eq!(result.certificate.domain, domain);
    assert!(result.certificate.residual.excluded_zero_sets().is_empty());
    result
}

#[test]
fn linear_and_quadratic_dispatch_without_factor_search() {
    let linear = checked(&p(&[-1, 2]), ScalarDomain::Rational);
    assert_eq!(
        linear.solutions(),
        &CertifiedSolutionSet::Finite(vec![PolynomialRootMultiplicity::new(
            ExactQuadraticRoot::Rational(Rational::new(Integer::one(), Integer::from(2)).unwrap()),
            1,
        )])
    );
    assert!(linear.complex_search.is_none());
    let quadratic = checked(&p(&[-2, 0, 1]), ScalarDomain::Real);
    assert_eq!(
        quadratic.solutions(),
        &CertifiedSolutionSet::Finite(vec![
            algebraic(&[-2, 0, 1], QuadraticBranch::LowerReal, 1),
            algebraic(&[-2, 0, 1], QuadraticBranch::UpperReal, 1),
        ])
    );
    assert!(quadratic.complex_search.is_none());
    assert_eq!(
        checked(&p(&[-2, 0, 1]), ScalarDomain::Rational).solutions(),
        &CertifiedSolutionSet::None
    );
}

#[test]
fn cubic_roots_of_unity() {
    let polynomial = p(&[-1, 0, 0, 1]);
    let expected = vec![
        rational(1, 1),
        algebraic(&[1, 1, 1], QuadraticBranch::NegativeImaginary, 1),
        algebraic(&[1, 1, 1], QuadraticBranch::PositiveImaginary, 1),
    ];
    let complex = checked(&polynomial, ScalarDomain::Complex);
    assert!(complex.is_complete());
    assert_eq!(complex.solutions(), &CertifiedSolutionSet::Finite(expected));
    for domain in [ScalarDomain::Rational, ScalarDomain::Real] {
        let result = checked(&polynomial, domain);
        assert!(result.is_complete());
        assert_eq!(
            result.solutions(),
            &CertifiedSolutionSet::Finite(vec![rational(1, 1)])
        );
    }
}

#[test]
fn quartic_roots_of_unity() {
    let polynomial = p(&[-1, 0, 0, 0, 1]);
    let result = checked(&polynomial, ScalarDomain::Complex);
    assert_eq!(
        result.solutions(),
        &CertifiedSolutionSet::Finite(vec![
            rational(-1, 1),
            rational(1, 1),
            algebraic(&[1, 0, 1], QuadraticBranch::NegativeImaginary, 1),
            algebraic(&[1, 0, 1], QuadraticBranch::PositiveImaginary, 1),
        ])
    );
    assert_eq!(
        checked(&polynomial, ScalarDomain::Real).solutions(),
        &CertifiedSolutionSet::Finite(vec![rational(-1, 1), rational(1, 1)])
    );
}

#[test]
fn repeated_rational_and_algebraic_roots() {
    let repeated = checked(&p(&[0, 1, -2, 1]), ScalarDomain::Real);
    assert_eq!(
        repeated.solutions(),
        &CertifiedSolutionSet::Finite(vec![rational(0, 1), rational(1, 2)])
    );

    let polynomial = p(&[-3, 1]).multiply(&p(&[2, 0, 1]).multiply(&p(&[2, 0, 1])));
    assert_eq!(
        checked(&polynomial, ScalarDomain::Complex).solutions(),
        &CertifiedSolutionSet::Finite(vec![
            rational(3, 1),
            algebraic(&[2, 0, 1], QuadraticBranch::NegativeImaginary, 2),
            algebraic(&[2, 0, 1], QuadraticBranch::PositiveImaginary, 2),
        ])
    );
    assert_eq!(
        checked(&polynomial, ScalarDomain::Real).solutions(),
        &CertifiedSolutionSet::Finite(vec![rational(3, 1)])
    );
}

#[test]
fn quartic_without_rational_roots() {
    let polynomial = p(&[6, 0, 5, 0, 1]);
    assert_eq!(
        checked(&polynomial, ScalarDomain::Complex).solutions(),
        &CertifiedSolutionSet::Finite(vec![
            algebraic(&[2, 0, 1], QuadraticBranch::NegativeImaginary, 1),
            algebraic(&[2, 0, 1], QuadraticBranch::PositiveImaginary, 1),
            algebraic(&[3, 0, 1], QuadraticBranch::NegativeImaginary, 1),
            algebraic(&[3, 0, 1], QuadraticBranch::PositiveImaginary, 1),
        ])
    );
    let real = checked(&polynomial, ScalarDomain::Real);
    assert!(real.is_complete());
    assert_eq!(real.solutions(), &CertifiedSolutionSet::None);
}

#[test]
fn irreducible_and_unresolved_cofactors_are_not_empty_sets() {
    let cubic = p(&[-2, 0, 0, 1]);
    for domain in [
        ScalarDomain::Rational,
        ScalarDomain::Real,
        ScalarDomain::Complex,
    ] {
        let result = checked(&cubic, domain);
        assert!(!result.is_complete());
        assert_eq!(
            result.solutions(),
            &CertifiedSolutionSet::Partial {
                roots: vec![],
                unresolved_cofactor: cubic.clone(),
            }
        );
    }
    let scaled = p(&[-3, 3]).multiply(&cubic);
    assert_eq!(
        checked(&scaled, ScalarDomain::Complex).solutions(),
        &CertifiedSolutionSet::Partial {
            roots: vec![rational(1, 1)],
            unresolved_cofactor: p(&[-6, 0, 0, 3]),
        }
    );
    let quartic = p(&[2_000_000, 0, 3_000, 0, 1]);
    assert!(!checked(&quartic, ScalarDomain::Real).is_complete());
}

#[test]
fn zero_and_nonzero_constants() {
    for domain in [
        ScalarDomain::Rational,
        ScalarDomain::Real,
        ScalarDomain::Complex,
    ] {
        let identity = checked(&p(&[]), domain);
        assert!(identity.is_complete());
        assert_eq!(identity.solutions(), &CertifiedSolutionSet::AllAdmissible);
        let contradiction = checked(&p(&[7]), domain);
        assert!(contradiction.is_complete());
        assert_eq!(contradiction.solutions(), &CertifiedSolutionSet::None);
    }
}

#[test]
fn rejects_forged_completeness_roots_and_source() {
    let context = EvaluationContext::default();
    let original = p(&[2, -2, 0, -1, 1]);
    let domain = ScalarDomain::Complex;
    let result = checked(&original, domain);
    for solutions in [
        CertifiedSolutionSet::Finite(vec![rational(1, 1)]),
        CertifiedSolutionSet::Partial {
            roots: vec![rational(1, 2)],
            unresolved_cofactor: p(&[-2, 0, 0, 1]),
        },
        CertifiedSolutionSet::Partial {
            roots: vec![rational(1, 1)],
            unresolved_cofactor: p(&[-3, 0, 0, 1]),
        },
        CertifiedSolutionSet::None,
    ] {
        let mut forged = result.clone();
        forged.certificate.solutions = solutions;
        assert!(!verify_polynomial_equation(&context, &original, &x(), domain, &forged).unwrap());
    }
    assert!(!verify_polynomial_equation(&context, &p(&[1, 1]), &x(), domain, &result).unwrap());
    assert!(
        !verify_polynomial_equation(&context, &original, &x(), ScalarDomain::Real, &result)
            .unwrap()
    );
    let mut forged = result.clone();
    forged.complex_search = None;
    assert!(!verify_polynomial_equation(&context, &original, &x(), domain, &forged).unwrap());
    let mut forged = result;
    forged.certificate = certify_solution_set(
        &context,
        &forged.certificate.left,
        &forged.certificate.right,
        &x(),
        domain,
        &[],
    )
    .unwrap();
    // This is a valid but weaker W4 certificate. The dispatch must reject it
    // because its supported factor search has already found x = 1.
    assert!(!verify_polynomial_equation(&context, &original, &x(), domain, &forged).unwrap());
}

#[test]
fn field_and_resource_refusals_are_typed() {
    let context = EvaluationContext::default();
    let polynomial = p(&[-1, 0, 0, 1]);
    assert_eq!(
        solve_polynomial_equation(&context, &polynomial, &x(), ScalarDomain::Integer),
        Err(PolynomialEquationError::UnsupportedDomain)
    );
    for (limits, original, kind) in [
        (
            ResourceLimits {
                max_polynomial_degree: 2,
                ..ResourceLimits::default()
            },
            polynomial.clone(),
            ResourceLimitKind::PolynomialDegree,
        ),
        (
            ResourceLimits {
                max_scalar_bits: 8,
                ..ResourceLimits::default()
            },
            p(&[1_000, 1]),
            ResourceLimitKind::ScalarBits,
        ),
        (
            ResourceLimits {
                max_work_units: 0,
                ..ResourceLimits::default()
            },
            polynomial.clone(),
            ResourceLimitKind::WorkUnits,
        ),
    ] {
        let limited = EvaluationContext::new(limits);
        assert_eq!(
            solve_polynomial_equation(&limited, &original, &x(), ScalarDomain::Complex),
            Err(PolynomialEquationError::Evaluation(EvaluationError::Limit(
                kind
            )))
        );
    }
}

#[test]
fn deterministic_output_and_verifier_budget() {
    let context = EvaluationContext::default();
    let polynomial = p(&[-1, 0, 0, 1]);
    let result = checked(&polynomial, ScalarDomain::Complex);
    for _ in 0..3 {
        assert_eq!(
            solve_polynomial_equation(&context, &polynomial, &x(), ScalarDomain::Complex).unwrap(),
            result
        );
    }
    let limited = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        verify_polynomial_equation(&limited, &polynomial, &x(), ScalarDomain::Complex, &result),
        Err(PolynomialEquationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits
        )))
    );
}
