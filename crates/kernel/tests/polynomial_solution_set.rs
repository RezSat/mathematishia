use mathema::{Integer, Rational};
use mathematishia_kernel::{
    AlgebraicRootSet, EvaluationContext, EvaluationError, ExactQuadraticRoot,
    PartialPolynomialRootError, PartialPolynomialRootResult, PolynomialFactorRootEvidence,
    PolynomialRootMultiplicity, PolynomialZeroSet, RationalPolynomial, ResourceLimitKind,
    ResourceLimits, solve_partial_polynomial_roots, verify_partial_polynomial_roots,
};

fn p(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients
            .iter()
            .map(|&value| Rational::from(Integer::from(value)))
            .collect(),
    )
}

fn q(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn rational_root(value: Rational, multiplicity: usize) -> PolynomialRootMultiplicity {
    PolynomialRootMultiplicity::new(ExactQuadraticRoot::Rational(value), multiplicity)
}

fn checked(polynomial: &RationalPolynomial) -> PartialPolynomialRootResult {
    let context = EvaluationContext::default();
    let result = solve_partial_polynomial_roots(&context, polynomial).unwrap();
    assert!(
        verify_partial_polynomial_roots(&context, polynomial, &result).unwrap(),
        "the solver result must verify"
    );
    result
}

fn forge(
    result: &PartialPolynomialRootResult,
    set: PolynomialZeroSet,
) -> PartialPolynomialRootResult {
    PartialPolynomialRootResult::from_unverified_parts(
        result.original().clone(),
        set,
        result.factorization().cloned(),
        result.root_evidence().to_vec(),
    )
}

#[test]
fn zero_constant_and_linear_are_distinct_complete_sets() {
    let zero = checked(&p(&[]));
    assert_eq!(zero.zero_set(), &PolynomialZeroSet::AllComplexNumbers);
    assert!(zero.zero_set().is_complete());
    assert!(zero.factorization().is_none());

    for value in [-7, 1, 99] {
        let constant = checked(&p(&[value]));
        assert_eq!(
            constant.zero_set(),
            &PolynomialZeroSet::Finite {
                roots: vec![],
                unresolved_cofactor: None,
                complete: true,
            }
        );
        assert!(constant.factorization().is_none());
    }

    let linear = RationalPolynomial::new(vec![-q(1, 2), q(3, 1)]);
    let result = checked(&linear);
    assert_eq!(
        result.zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![rational_root(q(1, 6), 1)],
            unresolved_cofactor: None,
            complete: true,
        }
    );
    assert!(result.factorization().is_some());
}

#[test]
fn quadratic_complex_roots_are_exact_and_distinct() {
    for polynomial in [p(&[-2, 0, 1]), p(&[1, 0, 1])] {
        let result = checked(&polynomial);
        let PolynomialZeroSet::Finite {
            roots,
            unresolved_cofactor,
            complete,
        } = result.zero_set()
        else {
            panic!("nonzero quadratic has a finite zero set")
        };
        assert!(*complete);
        assert!(unresolved_cofactor.is_none());
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().all(|root| {
            root.multiplicity() == 1 && matches!(root.root(), ExactQuadraticRoot::Algebraic { .. })
        }));
        assert_ne!(roots[0].root(), roots[1].root());
        assert_eq!(result.root_evidence().len(), 1);
        assert!(matches!(
            result.root_evidence()[0].roots().roots(),
            AlgebraicRootSet::Finite(_)
        ));
    }
    let split = checked(&p(&[-1, 0, 1]));
    assert_eq!(
        split.zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![rational_root(q(-1, 1), 1), rational_root(q(1, 1), 1)],
            unresolved_cofactor: None,
            complete: true,
        }
    );
}

#[test]
fn multiplicities_and_distinct_roots_survive_factorization() {
    let linear = p(&[-3, 1]);
    let quadratic = p(&[2, 0, 1]);
    let polynomial = linear.multiply(&quadratic).multiply(&quadratic);
    let result = checked(&polynomial);
    let PolynomialZeroSet::Finite {
        roots,
        unresolved_cofactor,
        complete,
    } = result.zero_set()
    else {
        panic!("expected finite roots")
    };
    assert!(*complete);
    assert!(unresolved_cofactor.is_none());
    assert_eq!(roots.len(), 3);
    assert_eq!(roots[0], rational_root(q(3, 1), 1));
    assert_eq!(roots[1].multiplicity(), 2);
    assert_eq!(roots[2].multiplicity(), 2);
    assert_ne!(roots[1].root(), roots[2].root());
    assert_eq!(result.root_evidence().len(), 2);

    let repeated = checked(&p(&[8, -12, 6, -1]));
    assert_eq!(
        repeated.zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![rational_root(q(2, 1), 3)],
            unresolved_cofactor: None,
            complete: true,
        }
    );
}

#[test]
fn reducible_cubic_and_quartic_are_complete() {
    let cubic = p(&[-1, 1]).multiply(&p(&[1, 0, 1]));
    let result = checked(&cubic);
    let PolynomialZeroSet::Finite {
        roots,
        unresolved_cofactor,
        complete,
    } = result.zero_set()
    else {
        panic!("cubic expected finite roots")
    };
    assert!(*complete);
    assert!(unresolved_cofactor.is_none());
    assert_eq!(roots.len(), 3);

    let quartic = p(&[-1, 1]).multiply(&p(&[1, 1])).multiply(&p(&[1, 0, 1]));
    let result = checked(&quartic);
    let PolynomialZeroSet::Finite {
        roots,
        complete,
        unresolved_cofactor,
    } = result.zero_set()
    else {
        panic!("quartic expected finite roots")
    };
    assert!(*complete);
    assert!(unresolved_cofactor.is_none());
    assert_eq!(roots.len(), 4);
    assert_eq!(
        roots.iter().map(|root| root.multiplicity()).sum::<usize>(),
        4
    );
}

#[test]
fn certified_irrreducible_cubic_does_not_imply_complete_roots() {
    let cubic = p(&[-2, 0, 0, 1]);
    let result = checked(&cubic);
    assert!(result.factorization().unwrap().is_complete());
    assert_eq!(
        result.zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: Some(cubic),
            complete: false,
        }
    );
}

#[test]
fn partial_roots_preserve_high_degree_residual() {
    let cubic = p(&[-2, 0, 0, 1]);
    let polynomial = p(&[-1, 1]).multiply(&cubic);
    let result = checked(&polynomial);
    assert_eq!(
        result.zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![rational_root(q(1, 1), 1)],
            unresolved_cofactor: Some(cubic),
            complete: false,
        }
    );

    // The rational factorizer itself deliberately leaves this large
    // primitive quartic unresolved; neither zero roots nor completeness
    // may be inferred from that bounded search.
    let quartic = p(&[2_000_000, 0, 3_000, 0, 1]);
    let result = checked(&quartic);
    assert!(!result.factorization().unwrap().is_complete());
    assert_eq!(
        result.zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: Some(quartic),
            complete: false,
        }
    );
}

#[test]
fn reject_forged_roots_completeness_multiplicity_and_source() {
    let context = EvaluationContext::default();
    let polynomial = p(&[-2, 0, 0, 1]);
    let result = checked(&polynomial);
    for set in [
        PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: Some(polynomial.clone()),
            complete: true,
        },
        PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: None,
            complete: true,
        },
        PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: Some(p(&[1, 1])),
            complete: false,
        },
        PolynomialZeroSet::Finite {
            roots: vec![rational_root(q(0, 1), 1)],
            unresolved_cofactor: Some(polynomial.clone()),
            complete: false,
        },
        PolynomialZeroSet::AllComplexNumbers,
    ] {
        assert!(
            !verify_partial_polynomial_roots(&context, &polynomial, &forge(&result, set)).unwrap()
        );
    }

    let repeated = p(&[4, -4, 1]);
    let correct = checked(&repeated);
    let wrong_multiplicity = PolynomialZeroSet::Finite {
        roots: vec![rational_root(q(2, 1), 1)],
        unresolved_cofactor: None,
        complete: true,
    };
    assert!(
        !verify_partial_polynomial_roots(&context, &repeated, &forge(&correct, wrong_multiplicity))
            .unwrap()
    );

    let wrong_source = PartialPolynomialRootResult::from_unverified_parts(
        p(&[1, 1]),
        correct.zero_set().clone(),
        correct.factorization().cloned(),
        correct.root_evidence().to_vec(),
    );
    assert!(!verify_partial_polynomial_roots(&context, &repeated, &wrong_source).unwrap());

    let duplicate = PolynomialZeroSet::Finite {
        roots: vec![rational_root(q(2, 1), 2), rational_root(q(2, 1), 2)],
        unresolved_cofactor: None,
        complete: true,
    };
    assert!(
        !verify_partial_polynomial_roots(&context, &repeated, &forge(&correct, duplicate)).unwrap()
    );
}

#[test]
fn reject_forged_factorization_and_root_evidence() {
    let context = EvaluationContext::default();
    let polynomial = p(&[2, 0, 1]);
    let result = checked(&polynomial);
    let foreign_factorization = context.factor_rational_polynomial(&p(&[3, 0, 1])).unwrap();
    let forged = PartialPolynomialRootResult::from_unverified_parts(
        polynomial.clone(),
        result.zero_set().clone(),
        Some(foreign_factorization),
        result.root_evidence().to_vec(),
    );
    assert!(!verify_partial_polynomial_roots(&context, &polynomial, &forged).unwrap());

    let foreign_evidence = context.algebraic_roots(&p(&[3, 0, 1])).unwrap();
    let forged = PartialPolynomialRootResult::from_unverified_parts(
        polynomial.clone(),
        result.zero_set().clone(),
        result.factorization().cloned(),
        vec![PolynomialFactorRootEvidence::from_parts(
            polynomial.clone(),
            1,
            foreign_evidence,
        )],
    );
    assert!(!verify_partial_polynomial_roots(&context, &polynomial, &forged).unwrap());
    let forged = PartialPolynomialRootResult::from_unverified_parts(
        polynomial.clone(),
        result.zero_set().clone(),
        result.factorization().cloned(),
        vec![],
    );
    assert!(!verify_partial_polynomial_roots(&context, &polynomial, &forged).unwrap());
}

#[test]
fn boundaries_large_coefficients_and_resource_exhaustion() {
    let context = EvaluationContext::default();
    let degree_32 = RationalPolynomial::new(
        (0..32)
            .map(|_| q(0, 1))
            .chain(std::iter::once(q(1, 1)))
            .collect(),
    );
    assert_eq!(
        checked(&degree_32).zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![rational_root(q(0, 1), 32)],
            unresolved_cofactor: None,
            complete: true,
        }
    );

    let degree_33 = RationalPolynomial::new(
        (0..33)
            .map(|_| q(0, 1))
            .chain(std::iter::once(q(1, 1)))
            .collect(),
    );
    assert_eq!(
        solve_partial_polynomial_roots(&context, &degree_33),
        Err(PartialPolynomialRootError::Evaluation(
            EvaluationError::Limit(ResourceLimitKind::PolynomialDegree)
        ))
    );
    let small = EvaluationContext::new(ResourceLimits {
        max_scalar_bits: 8,
        ..ResourceLimits::default()
    });
    assert_eq!(
        solve_partial_polynomial_roots(&small, &p(&[1_000, 1])),
        Err(PartialPolynomialRootError::Evaluation(
            EvaluationError::Limit(ResourceLimitKind::ScalarBits)
        ))
    );
    let no_work = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    assert_eq!(
        solve_partial_polynomial_roots(&no_work, &p(&[])),
        Err(PartialPolynomialRootError::Evaluation(
            EvaluationError::Limit(ResourceLimitKind::WorkUnits)
        ))
    );

    let large = p(&[-100_003, 0, 0, 1]);
    let result = checked(&large);
    assert!(!result.zero_set().is_complete());
    assert_eq!(
        result.zero_set(),
        &PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: Some(large),
            complete: false,
        }
    );
}

#[test]
fn deterministic_repeat_and_context_method() {
    let context = EvaluationContext::default();
    let polynomial = p(&[-1, 1]).multiply(&p(&[2, 0, 1]));
    let reference = context.partial_polynomial_roots(&polynomial).unwrap();
    for _ in 0..10 {
        assert_eq!(
            reference,
            solve_partial_polynomial_roots(&context, &polynomial).unwrap()
        );
        assert!(verify_partial_polynomial_roots(&context, &polynomial, &reference).unwrap());
    }
}
