use mathema::{
    Condition, DomainExpression, Equation, Expr, Integer, Rational, ScalarDomain, Symbol,
};
use mathematishia_kernel::{
    Assumptions, ConditionStatus, DerivationRenderError, EvaluationContext, ExactMatrix,
    ExactQuadraticRoot, GuardedRewriteOutcome, MultivariateLimits, PartialPolynomialRootResult,
    PolynomialExpansionOutcome, PolynomialFactorRootEvidence, PolynomialRootMultiplicity,
    PolynomialZeroSet, ProblemError, ProblemEvidence, ProblemOperation, ProblemRequest,
    ProblemStatement, ProofGraph, ProofNodeId, RationalFunction, RationalPolynomial,
    adapt_verified_problem, cancel_symbol_inverse_once, distribute_product_once, expand_polynomial,
    render_proof_graph, verify_linear_system_vector, verify_matrix_inverse,
    verify_partial_polynomial_roots, verify_polynomial_expansion, verify_proof_graph,
};

fn q(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}

fn poly(values: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(values.iter().copied().map(q).collect())
}

fn symbol(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

fn roots_with(
    original: &PartialPolynomialRootResult,
    zero_set: PolynomialZeroSet,
) -> PartialPolynomialRootResult {
    PartialPolynomialRootResult::from_unverified_parts(
        original.original().clone(),
        zero_set,
        original.factorization().cloned(),
        original.root_evidence().to_vec(),
    )
}

#[test]
fn forged_completeness_and_missing_unresolved_factors_cannot_certify_cubic_roots() {
    let context = EvaluationContext::default();
    let cubic = poly(&[-2, 0, 0, 1]);
    let sound = context.partial_polynomial_roots(&cubic).unwrap();
    assert_eq!(
        verify_partial_polynomial_roots(&context, &cubic, &sound),
        Ok(true)
    );
    assert!(!sound.zero_set().is_complete());

    let false_sets = [
        PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: Some(cubic.clone()),
            complete: true,
        },
        PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: None,
            complete: true,
        },
        PolynomialZeroSet::Finite {
            roots: vec![],
            unresolved_cofactor: Some(poly(&[1, 1])),
            complete: false,
        },
        PolynomialZeroSet::Finite {
            roots: vec![PolynomialRootMultiplicity::new(
                ExactQuadraticRoot::Rational(q(0)),
                1,
            )],
            unresolved_cofactor: Some(cubic.clone()),
            complete: false,
        },
        PolynomialZeroSet::AllComplexNumbers,
    ];
    for zero_set in false_sets {
        let forged = roots_with(&sound, zero_set);
        assert_eq!(
            verify_partial_polynomial_roots(&context, &cubic, &forged),
            Ok(false)
        );
    }

    let foreign_source = PartialPolynomialRootResult::from_unverified_parts(
        poly(&[1, 1]),
        sound.zero_set().clone(),
        sound.factorization().cloned(),
        sound.root_evidence().to_vec(),
    );
    assert_eq!(
        verify_partial_polynomial_roots(&context, &cubic, &foreign_source),
        Ok(false)
    );
}

#[test]
fn forged_multiplicities_duplicates_and_root_evidence_are_rejected() {
    let context = EvaluationContext::default();
    let repeated = poly(&[4, -4, 1]);
    let valid = context.partial_polynomial_roots(&repeated).unwrap();
    assert_eq!(
        verify_partial_polynomial_roots(&context, &repeated, &valid),
        Ok(true)
    );
    for roots in [
        vec![PolynomialRootMultiplicity::new(
            ExactQuadraticRoot::Rational(q(2)),
            1,
        )],
        vec![
            PolynomialRootMultiplicity::new(ExactQuadraticRoot::Rational(q(2)), 2),
            PolynomialRootMultiplicity::new(ExactQuadraticRoot::Rational(q(2)), 2),
        ],
        vec![PolynomialRootMultiplicity::new(
            ExactQuadraticRoot::Rational(q(3)),
            2,
        )],
    ] {
        let forged = roots_with(
            &valid,
            PolynomialZeroSet::Finite {
                roots,
                unresolved_cofactor: None,
                complete: true,
            },
        );
        assert_eq!(
            verify_partial_polynomial_roots(&context, &repeated, &forged),
            Ok(false)
        );
    }

    let quadratic = poly(&[2, 0, 1]);
    let certified = context.partial_polynomial_roots(&quadratic).unwrap();
    assert_eq!(
        verify_partial_polynomial_roots(&context, &quadratic, &certified),
        Ok(true)
    );
    let foreign_factorization = context
        .factor_rational_polynomial(&poly(&[3, 0, 1]))
        .unwrap();
    let replaced_factorization = PartialPolynomialRootResult::from_unverified_parts(
        quadratic.clone(),
        certified.zero_set().clone(),
        Some(foreign_factorization),
        certified.root_evidence().to_vec(),
    );
    assert_eq!(
        verify_partial_polynomial_roots(&context, &quadratic, &replaced_factorization),
        Ok(false)
    );

    let foreign_roots = context.algebraic_roots(&poly(&[3, 0, 1])).unwrap();
    let factor = &certified.root_evidence()[0];
    let replaced_evidence = PartialPolynomialRootResult::from_unverified_parts(
        quadratic.clone(),
        certified.zero_set().clone(),
        certified.factorization().cloned(),
        vec![PolynomialFactorRootEvidence::from_parts(
            factor.factor().clone(),
            factor.multiplicity(),
            foreign_roots,
        )],
    );
    assert_eq!(
        verify_partial_polynomial_roots(&context, &quadratic, &replaced_evidence),
        Ok(false)
    );
    let missing_evidence = PartialPolynomialRootResult::from_unverified_parts(
        quadratic.clone(),
        certified.zero_set().clone(),
        certified.factorization().cloned(),
        vec![],
    );
    assert_eq!(
        verify_partial_polynomial_roots(&context, &quadratic, &missing_evidence),
        Ok(false)
    );
}

#[test]
fn proof_graph_verification_rejects_changed_sources_and_reordered_steps() {
    let x = Expr::from(symbol("x"));
    let y = Expr::from(symbol("y"));
    let z = Expr::from(symbol("z"));
    let original = Expr::mul([x.clone(), Expr::add([y.clone(), Expr::from(1)])]);
    let step = distribute_product_once(&original).unwrap();

    let mut valid = ProofGraph::new(original.clone());
    let end = valid
        .add_distribution(ProofNodeId::new(0), step.clone())
        .unwrap();
    valid.select_conclusion(end);
    assert_eq!(
        verify_proof_graph(&original, &Assumptions::new(), &valid),
        Ok(())
    );
    assert!(render_proof_graph(&original, &Assumptions::new(), &valid).is_ok());
    assert!(verify_proof_graph(&z, &Assumptions::new(), &valid).is_err());
    assert!(matches!(
        render_proof_graph(&z, &Assumptions::new(), &valid),
        Err(DerivationRenderError::Proof(_))
    ));

    let unrelated = Expr::mul([x, Expr::add([z, Expr::from(1)])]);
    let mut wrong_step = ProofGraph::new(unrelated.clone());
    let conclusion = wrong_step
        .add_distribution(ProofNodeId::new(0), step)
        .unwrap();
    wrong_step.select_conclusion(conclusion);
    assert!(verify_proof_graph(&unrelated, &Assumptions::new(), &wrong_step).is_err());
    assert!(render_proof_graph(&unrelated, &Assumptions::new(), &wrong_step).is_err());

    let mut bad_parent = ProofGraph::new(original.clone());
    let cyclic = bad_parent
        .add_distribution(
            ProofNodeId::new(1),
            distribute_product_once(&original).unwrap(),
        )
        .unwrap();
    bad_parent.select_conclusion(cyclic);
    assert!(verify_proof_graph(&original, &Assumptions::new(), &bad_parent).is_err());
}

#[test]
fn forged_assumptions_and_excluded_point_claims_cannot_prove_cancellation() {
    let x = Expr::from(symbol("x"));
    let y = Expr::from(symbol("y"));
    let original = Expr::mul([x.clone(), Expr::pow(x.clone(), Expr::from(-1)), y.clone()]);
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x.clone())]).unwrap();
    let GuardedRewriteOutcome::Applied(step) = cancel_symbol_inverse_once(&original, &assumptions)
    else {
        panic!("the nonzero assumption should permit guarded cancellation");
    };
    let mut graph = ProofGraph::new(original.clone());
    let premise = graph
        .add_condition(Condition::NonZero(x.clone()), ConditionStatus::Assumed)
        .unwrap();
    let conclusion = graph
        .add_guarded_cancellation(ProofNodeId::new(0), premise, step)
        .unwrap();
    graph.select_conclusion(conclusion);

    assert_eq!(verify_proof_graph(&original, &assumptions, &graph), Ok(()));
    assert!(render_proof_graph(&original, &assumptions, &graph).is_ok());
    assert!(verify_proof_graph(&original, &Assumptions::new(), &graph).is_err());
    assert!(render_proof_graph(&original, &Assumptions::new(), &graph).is_err());

    let canceled = RationalFunction::new(poly(&[-1, 1]), poly(&[-1, 1])).unwrap();
    let filled = RationalFunction::from_polynomial(poly(&[1]));
    assert!(canceled.algebraically_equivalent(&filled));
    assert_ne!(canceled, filled);
    assert!(canceled.evaluate(&q(1)).is_err());
    assert_eq!(filled.evaluate(&q(1)), Ok(q(1)));
}

#[test]
fn expansion_verifier_detects_changed_source_ring_and_domain_history() {
    let x = Expr::from(symbol("x"));
    let vars = [symbol("x")];
    let limits = MultivariateLimits::default();
    let source = DomainExpression::from_expr(
        Expr::pow(Expr::add([x.clone(), Expr::from(1)]), Expr::from(2)),
        64,
    )
    .unwrap();
    let PolynomialExpansionOutcome::Exact(correct) = expand_polynomial(&source, &vars, limits)
    else {
        panic!("small polynomial expansion must be exact");
    };
    assert_eq!(
        verify_polynomial_expansion(&source, &correct, &vars, limits),
        Ok(true)
    );
    let changed = DomainExpression::from_expr(
        Expr::pow(Expr::add([x.clone(), Expr::from(2)]), Expr::from(2)),
        64,
    )
    .unwrap();
    assert_eq!(
        verify_polynomial_expansion(&changed, &correct, &vars, limits),
        Ok(false)
    );
    assert_eq!(
        verify_polynomial_expansion(&source, &correct, &[symbol("x"), symbol("y")], limits),
        Ok(false)
    );

    let history =
        DomainExpression::with_conditions(Expr::from(0), [Condition::NonZero(x)], 64, 8).unwrap();
    let plain = DomainExpression::from_expr(Expr::from(0), 64).unwrap();
    let PolynomialExpansionOutcome::Exact(with_hole) = expand_polynomial(&history, &vars, limits)
    else {
        panic!("constant with domain history remains polynomial");
    };
    assert_eq!(
        verify_polynomial_expansion(&history, &with_hole, &vars, limits),
        Ok(true)
    );
    assert_eq!(
        verify_polynomial_expansion(&plain, &with_hole, &vars, limits),
        Ok(false)
    );
    assert_ne!(history.conditions(), plain.conditions());
}

#[test]
fn typed_problem_adapter_rejects_foreign_equation_and_fabricated_scalar_evidence() {
    let context = EvaluationContext::default();
    let x = Expr::from(symbol("x"));
    let expected = Equation::new(x.clone(), Expr::from(2));
    let foreign = Equation::new(x, Expr::from(3));
    let certificate = context.solve_linear(&foreign, &symbol("x")).unwrap();
    let request = ProblemRequest {
        operation: ProblemOperation::SolveLinear,
        statement: ProblemStatement::Equation(expected),
        variables: vec![symbol("x")],
        domain: ScalarDomain::Rational,
    };
    assert_eq!(
        adapt_verified_problem(&context, &request, ProblemEvidence::Linear(&certificate)),
        Err(ProblemError::InvalidEvidence)
    );

    let evaluation = ProblemRequest {
        operation: ProblemOperation::EvaluateRational,
        statement: ProblemStatement::RationalEvaluation {
            function: RationalFunction::from_polynomial(poly(&[7])),
            at: q(4),
        },
        variables: vec![symbol("x")],
        domain: ScalarDomain::Rational,
    };
    assert_eq!(
        adapt_verified_problem(
            &context,
            &evaluation,
            ProblemEvidence::RationalEvaluation(&q(6))
        ),
        Err(ProblemError::InvalidEvidence)
    );
}

#[test]
fn independent_matrix_verifiers_reject_altered_inverse_and_solution() {
    let context = EvaluationContext::default();
    let matrix = ExactMatrix::new(&context, vec![vec![q(2), q(1)], vec![q(5), q(3)]]).unwrap();
    let actual = matrix.inverse(&context).unwrap().unwrap();
    assert!(verify_matrix_inverse(&context, &matrix, &actual).unwrap());
    let forged = ExactMatrix::new(&context, vec![vec![q(3), q(-1)], vec![q(-5), q(3)]]).unwrap();
    assert!(!verify_matrix_inverse(&context, &matrix, &forged).unwrap());
    assert!(
        !verify_linear_system_vector(&context, &matrix, &[q(5), q(13)], &[q(2), q(2)]).unwrap()
    );
}
