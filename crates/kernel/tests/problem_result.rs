use mathema::{
    Condition, DomainExpression, Equation, Expr, Integer, Rational, ScalarDomain, Symbol,
};
use mathematishia_kernel::*;

fn variable(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

fn polynomial(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients
            .iter()
            .copied()
            .map(|value| Rational::from(Integer::from(value)))
            .collect(),
    )
}

fn request(
    operation: ProblemOperation,
    statement: ProblemStatement,
    domain: ScalarDomain,
) -> ProblemRequest {
    ProblemRequest {
        operation,
        statement,
        variables: vec![variable("x")],
        domain,
    }
}

#[test]
fn linear_and_quadratic_paths_agree_on_exact_rational_answer() {
    let context = EvaluationContext::default();
    let equation = Equation::new(Expr::from(variable("x")), Expr::from(2));
    let linear = request(
        ProblemOperation::SolveLinear,
        ProblemStatement::Equation(equation),
        ScalarDomain::Rational,
    );
    let polynomial_roots = request(
        ProblemOperation::RationalRoots,
        ProblemStatement::PolynomialZero(polynomial(&[-2, 1])),
        ScalarDomain::Rational,
    );
    let left = dispatch_problem(&context, &linear).unwrap();
    let right = dispatch_problem(&context, &polynomial_roots).unwrap();
    assert_eq!(left.outcome(), right.outcome());
    assert_eq!(
        left.outcome(),
        &ProblemOutcome::SolvedExact(ExactAnswer::RationalRoots(vec![Rational::from(
            Integer::from(2)
        )]))
    );
    assert_eq!(left.request(), &linear);
    assert_eq!(ProblemOperation::SolveLinear.stable_id(), "equation.linear");
    assert!(left.checked_by().is_some());
    assert!(right.checked_by().is_some());
    assert!(!left.steps().is_empty());
    assert!(right.steps().is_empty());
    assert_eq!(left.steps()[0].rule_id, "equation.collect_to_zero");
    assert_eq!(dispatch_problem(&context, &linear).unwrap(), left);
}

#[test]
fn exact_rational_root_absence_is_not_complex_root_absence() {
    let context = EvaluationContext::default();
    let p = polynomial(&[1, 0, 1]);
    let rationals = dispatch_problem(
        &context,
        &request(
            ProblemOperation::RationalRoots,
            ProblemStatement::PolynomialZero(p.clone()),
            ScalarDomain::Rational,
        ),
    )
    .unwrap();
    assert_eq!(rationals.outcome(), &ProblemOutcome::NoSolutions);
    assert_eq!(rationals.result_domain(), ScalarDomain::Rational);
    let complex = dispatch_problem(
        &context,
        &request(
            ProblemOperation::ComplexPolynomialRoots,
            ProblemStatement::PolynomialZero(p),
            ScalarDomain::Complex,
        ),
    )
    .unwrap();
    assert_eq!(complex.result_domain(), ScalarDomain::Complex);
    match complex.outcome() {
        ProblemOutcome::SolvedExact(ExactAnswer::ComplexRoots(values)) => {
            assert_eq!(values.len(), 2)
        }
        other => panic!("unexpected exact complex result: {other:?}"),
    }
}

#[test]
fn unsolved_complex_cofactor_is_not_a_no_solution_claim() {
    let context = EvaluationContext::default();
    let request = request(
        ProblemOperation::ComplexPolynomialRoots,
        ProblemStatement::PolynomialZero(polynomial(&[-2, 0, 0, 1])),
        ScalarDomain::Complex,
    );
    let result = dispatch_problem(&context, &request).unwrap();
    match result.outcome() {
        ProblemOutcome::Partial {
            known: ExactAnswer::ComplexRoots(roots),
            unresolved,
        } => {
            assert!(roots.is_empty());
            assert_eq!(unresolved.degree(), Some(3));
        }
        other => panic!("an unresolved cubic is not root-free: {other:?}"),
    }
    assert!(result.checked_by().is_some());
}

#[test]
fn rational_cancellation_preserves_excluded_points_and_undefined_evaluation() {
    let context = EvaluationContext::default();
    let factor = polynomial(&[-1, 1]);
    let canceled = context.rational_function(&factor, &factor).unwrap();
    let one = RationalFunction::from_polynomial(polynomial(&[1]));
    let equation = request(
        ProblemOperation::SolveRational,
        ProblemStatement::RationalEquation {
            left: canceled.clone(),
            right: one,
        },
        ScalarDomain::Rational,
    );
    let all = dispatch_problem(&context, &equation).unwrap();
    assert_eq!(all.outcome(), &ProblemOutcome::AllAdmissibleValues);
    assert_eq!(
        all.restrictions(),
        &[ProblemRestriction::PolynomialNonZero(factor)]
    );
    let at_hole = request(
        ProblemOperation::EvaluateRational,
        ProblemStatement::RationalEvaluation {
            function: canceled.clone(),
            at: Rational::from(Integer::from(1)),
        },
        ScalarDomain::Rational,
    );
    let undefined = dispatch_problem(&context, &at_hole).unwrap();
    assert_eq!(
        undefined.outcome(),
        &ProblemOutcome::Undefined(UndefinedReason::ExcludedPoint)
    );
    assert_eq!(undefined.checked_by(), None);
    assert!(!undefined.restrictions().is_empty());
    let allowed = request(
        ProblemOperation::EvaluateRational,
        ProblemStatement::RationalEvaluation {
            function: canceled,
            at: Rational::from(Integer::from(2)),
        },
        ScalarDomain::Rational,
    );
    assert_eq!(
        dispatch_problem(&context, &allowed).unwrap().outcome(),
        &ProblemOutcome::SolvedExact(ExactAnswer::Rational(Rational::from(Integer::from(1))))
    );
}

#[test]
fn real_derivatives_keep_unproved_domain_conditions_and_checked_step_ids() {
    let context = EvaluationContext::default();
    let x = Expr::from(variable("x"));
    let source =
        DomainExpression::from_expr_in_domain(Expr::log(x.clone()), ScalarDomain::Real, 64, 8)
            .unwrap();
    let request = request(
        ProblemOperation::Differentiate,
        ProblemStatement::Expression(source),
        ScalarDomain::Real,
    );
    let result = dispatch_problem(&context, &request).unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedConditional(ExactAnswer::Expression(Expr::pow(
            x.clone(),
            Expr::from(-1)
        )))
    );
    assert_eq!(
        result.restrictions(),
        &[ProblemRestriction::Condition(Condition::Positive(x))]
    );
    assert!(
        result
            .steps()
            .iter()
            .any(|step| step.rule_id == "calculus.log_chain")
    );
    assert!(result.checked_by().is_some());
}

#[test]
fn inherited_condition_on_constant_expression_is_not_dropped() {
    let context = EvaluationContext::default();
    let x = Expr::from(variable("x"));
    let source =
        DomainExpression::with_conditions(Expr::from(0), [Condition::NonZero(x.clone())], 64, 8)
            .unwrap();
    let result = dispatch_problem(
        &context,
        &request(
            ProblemOperation::Differentiate,
            ProblemStatement::Expression(source),
            ScalarDomain::Real,
        ),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedConditional(ExactAnswer::Expression(Expr::from(0)))
    );
    assert_eq!(
        result.restrictions(),
        &[ProblemRestriction::Condition(Condition::NonZero(x))]
    );
}

#[test]
fn contradictory_real_domain_and_complex_log_are_distinguished() {
    let context = EvaluationContext::default();
    let impossible =
        DomainExpression::from_expr_in_domain(Expr::log(Expr::from(-1)), ScalarDomain::Real, 32, 8)
            .unwrap();
    let result = dispatch_problem(
        &context,
        &request(
            ProblemOperation::Differentiate,
            ProblemStatement::Expression(impossible),
            ScalarDomain::Real,
        ),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::Undefined(UndefinedReason::ContradictedDomainCondition)
    );
    assert_eq!(result.checked_by(), None);
    let source = DomainExpression::from_expr_in_domain(
        Expr::log(Expr::from(variable("x"))),
        ScalarDomain::Complex,
        32,
        8,
    )
    .unwrap();
    let result = dispatch_problem(
        &context,
        &request(
            ProblemOperation::Differentiate,
            ProblemStatement::Expression(source),
            ScalarDomain::Complex,
        ),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::Unsupported(UnsupportedReason::ExpressionOutsideAlgorithm)
    );
    assert_eq!(result.checked_by(), None);
}

#[test]
fn forged_and_mismatched_certificates_are_rejected() {
    let context = EvaluationContext::default();
    let original = Equation::new(Expr::from(variable("x")), Expr::from(2));
    let different = Equation::new(Expr::from(variable("x")), Expr::from(3));
    let evidence = context.solve_linear(&different, &variable("x")).unwrap();
    let input = request(
        ProblemOperation::SolveLinear,
        ProblemStatement::Equation(original),
        ScalarDomain::Rational,
    );
    assert_eq!(
        adapt_verified_problem(&context, &input, ProblemEvidence::Linear(&evidence)),
        Err(ProblemError::InvalidEvidence)
    );
    let poly = polynomial(&[-1, 1]);
    let verified = context.partial_polynomial_roots(&poly).unwrap();
    let forged = PartialPolynomialRootResult::from_unverified_parts(
        poly.clone(),
        PolynomialZeroSet::Finite {
            roots: Vec::new(),
            unresolved_cofactor: None,
            complete: true,
        },
        verified.factorization().cloned(),
        verified.root_evidence().to_vec(),
    );
    let polynomial_input = request(
        ProblemOperation::ComplexPolynomialRoots,
        ProblemStatement::PolynomialZero(poly),
        ScalarDomain::Complex,
    );
    assert_eq!(
        adapt_verified_problem(
            &context,
            &polynomial_input,
            ProblemEvidence::ComplexRoots(&forged)
        ),
        Err(ProblemError::InvalidEvidence)
    );
    let function = RationalFunction::from_polynomial(polynomial(&[1]));
    let at = request(
        ProblemOperation::EvaluateRational,
        ProblemStatement::RationalEvaluation {
            function,
            at: Rational::from(Integer::from(4)),
        },
        ScalarDomain::Rational,
    );
    assert_eq!(
        adapt_verified_problem(
            &context,
            &at,
            ProblemEvidence::RationalEvaluation(&Rational::from(Integer::from(42)))
        ),
        Err(ProblemError::InvalidEvidence)
    );
}

#[test]
fn invalid_requests_unsupported_operations_and_degree_limits_are_typed() {
    let context = EvaluationContext::default();
    let mismatched = request(
        ProblemOperation::SolveLinear,
        ProblemStatement::PolynomialZero(polynomial(&[1, 2])),
        ScalarDomain::Rational,
    );
    assert_eq!(
        dispatch_problem(&context, &mismatched),
        Err(ProblemError::StatementMismatch)
    );
    let mut missing = mismatched.clone();
    missing.variables.clear();
    assert_eq!(
        dispatch_problem(&context, &missing),
        Err(ProblemError::MissingVariable)
    );
    let mut many = mismatched.clone();
    many.variables.push(variable("y"));
    assert_eq!(
        dispatch_problem(&context, &many).unwrap().outcome(),
        &ProblemOutcome::Unsupported(UnsupportedReason::SeveralVariables)
    );
    let integral = request(
        ProblemOperation::Integrate,
        ProblemStatement::Expression(
            DomainExpression::from_expr(Expr::from(variable("x")), 32).unwrap(),
        ),
        ScalarDomain::Real,
    );
    assert_eq!(
        dispatch_problem(&context, &integral).unwrap().outcome(),
        &ProblemOutcome::SolvedExact(ExactAnswer::Expression(Expr::mul([
            Expr::from(Rational::new(Integer::from(1), Integer::from(2)).unwrap()),
            Expr::pow(Expr::from(variable("x")), Expr::from(2))
        ])))
    );
    let high = request(
        ProblemOperation::RationalRoots,
        ProblemStatement::PolynomialZero(polynomial(&[1, 0, 0, 1])),
        ScalarDomain::Rational,
    );
    assert_eq!(
        dispatch_problem(&context, &high).unwrap().outcome(),
        &ProblemOutcome::Unsupported(UnsupportedReason::DegreeOutsideAlgorithm)
    );
    let wrong_domain = request(
        ProblemOperation::RationalRoots,
        ProblemStatement::PolynomialZero(polynomial(&[-1, 1])),
        ScalarDomain::Complex,
    );
    assert_eq!(
        dispatch_problem(&context, &wrong_domain).unwrap().outcome(),
        &ProblemOutcome::Unsupported(UnsupportedReason::Domain)
    );
}

#[test]
fn resource_exhaustion_is_not_misreported_as_unsolved() {
    let limits = ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    };
    let context = EvaluationContext::new(limits);
    let input = request(
        ProblemOperation::RationalRoots,
        ProblemStatement::PolynomialZero(polynomial(&[-1, 0, 1])),
        ScalarDomain::Rational,
    );
    let result = dispatch_problem(&context, &input).unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::ResourceRefusal(ResourceLimitKind::WorkUnits)
    );
    assert_eq!(result.checked_by(), None);
    assert!(result.steps().is_empty());
}

fn source(expression: Expr) -> DomainExpression {
    DomainExpression::from_expr_in_domain(expression, ScalarDomain::Real, 2048, 32).unwrap()
}

fn q(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}

#[test]
fn shared_integration_binds_certificate_to_original_source() {
    let context = EvaluationContext::default();
    let x = Expr::from(variable("x"));
    let input = source(Expr::add([x.clone(), Expr::sin(x)]));
    let proof = context
        .integrate_expression_dispatch(&input, &variable("x"), ScalarDomain::Real)
        .unwrap();
    let req = request(
        ProblemOperation::Integrate,
        ProblemStatement::Expression(input),
        ScalarDomain::Real,
    );
    let result = dispatch_problem(&context, &req).unwrap();
    assert_eq!(result.request(), &req);
    assert_eq!(result.checked_by(), Some(CheckedBy::IntegrationTerms));
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedExact(ExactAnswer::Expression(
            proof.primitive().expression().clone()
        ))
    );
    let wrong = request(
        ProblemOperation::Integrate,
        ProblemStatement::Expression(source(Expr::from(2))),
        ScalarDomain::Real,
    );
    assert!(
        adapt_verified_problem(&context, &wrong, ProblemEvidence::Integration(&proof)).is_err()
    );
}

#[test]
fn shared_expansion_keeps_conditions_and_explicit_ring() {
    let context = EvaluationContext::default();
    let x = Expr::from(variable("x"));
    let y = Expr::from(variable("y"));
    let condition = Condition::NonZero(x.clone());
    let input = DomainExpression::with_conditions(
        Expr::pow(Expr::add([x, y]), Expr::from(2)),
        [condition.clone()],
        128,
        8,
    )
    .unwrap();
    let variables = vec![variable("x"), variable("y")];
    let req = ProblemRequest {
        operation: ProblemOperation::Expand,
        statement: ProblemStatement::Expression(input.clone()),
        variables: variables.clone(),
        domain: ScalarDomain::Real,
    };
    let PolynomialExpansionOutcome::Exact(proof) =
        expand_polynomial(&input, &variables, MultivariateLimits::default())
    else {
        panic!("expected expansion")
    };
    let result = dispatch_problem(&context, &req).unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedConditional(ExactAnswer::Expression(
            proof.expanded().expression().clone()
        ))
    );
    assert!(
        result
            .restrictions()
            .contains(&ProblemRestriction::Condition(condition))
    );
    assert_eq!(result.checked_by(), Some(CheckedBy::PolynomialIdentity));
    let wrong = ProblemRequest {
        variables: vec![variable("y"), variable("x")],
        ..req
    };
    assert!(adapt_verified_problem(&context, &wrong, ProblemEvidence::Expansion(&proof)).is_err());
}

#[test]
fn shared_limit_preserves_hole_and_rejects_wrong_point_or_direction() {
    let context = EvaluationContext::default();
    let condition = Condition::NonZero(Expr::add([Expr::from(variable("x")), Expr::from(-1)]));
    let input =
        DomainExpression::with_conditions(Expr::from(1), [condition.clone()], 128, 8).unwrap();
    let proof = context
        .rational_expression_limit(&input, &variable("x"), &q(1), LimitDirection::TwoSided)
        .unwrap();
    let req = request(
        ProblemOperation::Limit,
        ProblemStatement::Limit {
            source: input.clone(),
            at: q(1),
            direction: LimitDirection::TwoSided,
        },
        ScalarDomain::Real,
    );
    let result = dispatch_problem(&context, &req).unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedExact(ExactAnswer::Limit(LimitValue::Finite(q(1))))
    );
    assert!(
        result
            .restrictions()
            .contains(&ProblemRestriction::Condition(condition))
    );
    assert!(
        result
            .restrictions()
            .contains(&ProblemRestriction::PolynomialNonZero(polynomial(&[-1, 1])))
    );
    for (at, direction) in [
        (q(2), LimitDirection::TwoSided),
        (q(1), LimitDirection::Left),
    ] {
        let wrong = ProblemRequest {
            statement: ProblemStatement::Limit {
                source: input.clone(),
                at,
                direction,
            },
            ..req.clone()
        };
        assert!(adapt_verified_problem(&context, &wrong, ProblemEvidence::Limit(&proof)).is_err());
    }
    let pole = source(Expr::pow(Expr::from(variable("x")), Expr::from(-1)));
    for (direction, value) in [
        (LimitDirection::Left, LimitValue::NegativeInfinity),
        (LimitDirection::TwoSided, LimitValue::DoesNotExist),
    ] {
        let req = request(
            ProblemOperation::Limit,
            ProblemStatement::Limit {
                source: pole.clone(),
                at: q(0),
                direction,
            },
            ScalarDomain::Real,
        );
        assert_eq!(
            dispatch_problem(&context, &req).unwrap().outcome(),
            &ProblemOutcome::SolvedExact(ExactAnswer::Limit(value))
        );
    }
}

#[test]
fn shared_inequality_preserves_algebraic_bounds_and_empty_set() {
    let context = EvaluationContext::default();
    let left = source(Expr::pow(Expr::from(variable("x")), Expr::from(2)));
    let right = source(Expr::from(2));
    let proof = context
        .solve_expression_inequality(&left, &right, &variable("x"), ComparisonOperator::Less)
        .unwrap();
    let req = request(
        ProblemOperation::Inequality,
        ProblemStatement::Comparison {
            left: left.clone(),
            right: right.clone(),
            operator: ComparisonOperator::Less,
        },
        ScalarDomain::Real,
    );
    let result = dispatch_problem(&context, &req).unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedExact(ExactAnswer::Intervals(proof.intervals().to_vec()))
    );
    assert_eq!(proof.intervals().len(), 1);
    let json = result.to_json().unwrap();
    assert!(json.contains("algebraic"));
    assert!(json.contains("minimal_polynomial"));
    let wrong = ProblemRequest {
        statement: ProblemStatement::Comparison {
            left: left.clone(),
            right,
            operator: ComparisonOperator::Greater,
        },
        ..req.clone()
    };
    assert!(adapt_verified_problem(&context, &wrong, ProblemEvidence::Inequality(&proof)).is_err());
    let impossible = ProblemRequest {
        statement: ProblemStatement::Comparison {
            left,
            right: source(Expr::from(0)),
            operator: ComparisonOperator::Less,
        },
        ..req
    };
    let empty = dispatch_problem(&context, &impossible).unwrap();
    assert_eq!(empty.outcome(), &ProblemOutcome::NoSolutions);
    assert_eq!(empty.checked_by(), Some(CheckedBy::RationalInequalitySigns));
}

#[test]
fn automatic_equation_dispatch_never_promotes_rational_completeness_to_complex() {
    let context = EvaluationContext::default();
    let x = Expr::from(variable("x"));
    let condition = Condition::NonZero(Expr::add([x.clone(), Expr::from(-1)]));
    let left = DomainExpression::with_conditions(x.clone(), [condition.clone()], 128, 8).unwrap();
    let statement = ProblemStatement::SourceEquation {
        left,
        right: DomainExpression::from_expr(Expr::from(1), 128).unwrap(),
    };
    let req = request(
        ProblemOperation::SolveAuto,
        statement,
        ScalarDomain::Rational,
    );
    let result = dispatch_problem(&context, &req).unwrap();
    assert_eq!(result.outcome(), &ProblemOutcome::NoSolutions);
    assert_eq!(result.request(), &req);
    let complex = ProblemRequest {
        domain: ScalarDomain::Complex,
        ..req
    };
    let result = dispatch_problem(&context, &complex).unwrap();
    assert_eq!(result.request(), &complex);
    assert_eq!(result.result_domain(), ScalarDomain::Rational);
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::PartialScope {
            known: ExactAnswer::RationalRoots(vec![]),
            reason: UnsupportedReason::Domain
        }
    );
    assert!(
        result
            .restrictions()
            .contains(&ProblemRestriction::Condition(condition))
    );
    let unrestricted = ProblemRequest {
        statement: ProblemStatement::SourceEquation {
            left: DomainExpression::from_expr(
                Expr::add([Expr::pow(x, Expr::from(2)), Expr::from(1)]),
                128,
            )
            .unwrap(),
            right: DomainExpression::from_expr(Expr::from(0), 128).unwrap(),
        },
        ..complex
    };
    let result = dispatch_problem(&context, &unrestricted).unwrap();
    assert_eq!(result.request(), &unrestricted);
    assert_eq!(result.result_domain(), ScalarDomain::Complex);
    assert!(
        matches!(result.outcome(), ProblemOutcome::SolvedExact(ExactAnswer::ComplexRoots(roots)) if roots.len() == 2)
    );
}

fn exact_request(expression: Expr) -> ProblemRequest {
    ProblemRequest {
        operation: ProblemOperation::EvaluateExact,
        statement: ProblemStatement::Expression(source(expression)),
        variables: vec![],
        domain: ScalarDomain::Real,
    }
}

#[test]
fn exact_constant_dispatch_distinguishes_rewrite_partial_and_refusal() {
    let context = EvaluationContext::default();
    let result = dispatch_problem(
        &context,
        &exact_request(Expr::add([
            Expr::sin(Expr::from(0)),
            Expr::cos(Expr::from(0)),
            Expr::exp(Expr::from(0)),
        ])),
    )
    .unwrap();
    assert_eq!(
        result.outcome(),
        &ProblemOutcome::SolvedExact(ExactAnswer::Rational(q(2)))
    );
    assert_eq!(result.checked_by(), Some(CheckedBy::AnalyticConstantRules));
    assert_eq!(result.steps().len(), 3);
    let partial = dispatch_problem(
        &context,
        &exact_request(Expr::add([
            Expr::cos(Expr::from(0)),
            Expr::sin(Expr::from(1)),
        ])),
    )
    .unwrap();
    assert!(matches!(
        partial.outcome(),
        ProblemOutcome::PartialScope { .. }
    ));
    assert_eq!(partial.steps().len(), 1);
    let unsupported = dispatch_problem(&context, &exact_request(Expr::sin(Expr::from(1)))).unwrap();
    assert!(matches!(
        unsupported.outcome(),
        ProblemOutcome::Unsupported(_)
    ));
    assert_eq!(unsupported.checked_by(), None);
    let limited = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    let refusal = dispatch_problem(&limited, &exact_request(Expr::sin(Expr::from(0)))).unwrap();
    assert!(matches!(
        refusal.outcome(),
        ProblemOutcome::ResourceRefusal(_)
    ));
    assert_eq!(refusal.checked_by(), None);
}

#[test]
fn canonical_json_is_exact_deterministic_and_bounded() {
    let context = EvaluationContext::default();
    let half = Rational::new(Integer::from(1), Integer::from(2)).unwrap();
    let req = exact_request(Expr::from(half));
    let result = dispatch_problem(&context, &req).unwrap();
    let json = result.to_json().unwrap();
    assert_eq!(
        json,
        dispatch_problem(&context, &req).unwrap().to_json().unwrap()
    );
    assert!(json.contains(r#""schema_version":1"#));
    assert!(json.contains(r#""numerator":"1","denominator":"2""#));
    assert!(json.contains(r#""status":"complete""#));
    let request_json = req.to_json().unwrap();
    assert!(json.starts_with(request_json.strip_suffix('}').unwrap()));
    let unusual = request(
        ProblemOperation::SolveLinear,
        ProblemStatement::Equation(Equation::new(
            Expr::from(variable("a\"\\\n\u{0001}")),
            Expr::from(0),
        )),
        ScalarDomain::Rational,
    );
    assert!(unusual.to_json().unwrap().contains(r#"a\"\\\n\u0001"#));
    let large = request(
        ProblemOperation::SolveLinear,
        ProblemStatement::Equation(Equation::new(
            Expr::from(variable(&"x".repeat(2 * 1024 * 1024))),
            Expr::from(0),
        )),
        ScalarDomain::Rational,
    );
    assert_eq!(large.to_json(), Err(ProblemWireError::OutputLimit));
}
