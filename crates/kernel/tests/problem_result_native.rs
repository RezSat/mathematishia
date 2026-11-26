use mathema::{DomainExpression, Equation, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::step_records::{StepEvidence, StepLimits, record_steps};
use mathematishia_kernel::*;

fn q(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}

fn x() -> Symbol {
    Symbol::new("x").unwrap()
}

fn affine(slope: i64, constant: i64) -> AbsoluteAffine {
    AbsoluteAffine {
        slope: q(slope, 1),
        constant: q(constant, 1),
    }
}

fn absolute(right: i64) -> ProblemRequest {
    let equation = AbsoluteEquation {
        domain: ScalarDomain::Real,
        left: AbsoluteExpression {
            variable: x(),
            base: affine(1, 0),
            layers: vec![affine(1, 0)],
        },
        right: AbsoluteExpression {
            variable: x(),
            base: affine(0, right),
            layers: vec![],
        },
    };
    ProblemRequest {
        operation: ProblemOperation::SolveAbsolute,
        statement: ProblemStatement::Absolute(Box::new(equation)),
        variables: vec![x()],
        domain: ScalarDomain::Real,
    }
}

fn special(function: TrigFunction, n: i64, d: i64) -> ProblemRequest {
    ProblemRequest {
        operation: ProblemOperation::EvaluateSpecialAngle,
        statement: ProblemStatement::SpecialAngle {
            angle: PiMultiple::new(q(n, d)),
            function,
        },
        variables: vec![],
        domain: ScalarDomain::Real,
    }
}

fn zeros(function: TrigFunction) -> ProblemRequest {
    ProblemRequest {
        operation: ProblemOperation::TrigZeros,
        statement: ProblemStatement::TrigZeros(function),
        variables: vec![x()],
        domain: ScalarDomain::Real,
    }
}

fn source(expression: Expr) -> DomainExpression {
    DomainExpression::from_expr(expression, 32).unwrap()
}

fn ideal(target: Option<DomainExpression>) -> ProblemRequest {
    ProblemRequest {
        operation: if target.is_some() {
            ProblemOperation::IdealMembership
        } else {
            ProblemOperation::GroebnerBasis
        },
        statement: ProblemStatement::Ideal {
            generators: vec![source(Expr::pow(x().into(), 2.into()))],
            target,
        },
        variables: vec![x()],
        domain: ScalarDomain::Rational,
    }
}

fn stable(request: &ProblemRequest) -> ProblemResult {
    let context = EvaluationContext::default();
    let checked = dispatch_problem(&context, request).unwrap();
    let json = checked.to_json().unwrap();
    assert_eq!(
        json,
        dispatch_problem(&context, request)
            .unwrap()
            .to_json()
            .unwrap()
    );
    assert!(json.starts_with(request.to_json().unwrap().strip_suffix('}').unwrap()));
    checked
}

#[test]
fn absolute_union_retains_sign_cases() {
    let checked = stable(&absolute(3));
    let ProblemOutcome::SolvedExact(ExactAnswer::Absolute(answer)) = checked.outcome() else {
        panic!("expected complete absolute solution union");
    };
    assert_eq!(answer.solutions.len(), 2);
    assert_eq!(answer.solutions[0].lower, Some(q(-3, 1)));
    assert_eq!(answer.solutions[1].upper, Some(q(3, 1)));
    assert_eq!(checked.checked_by(), Some(CheckedBy::AbsoluteSignCases));
    assert!(checked.to_json().unwrap().contains("\"signs\""));
}

#[test]
fn absolute_empty_union_retains_evidence() {
    let checked = stable(&absolute(-1));
    assert_eq!(checked.outcome(), &ProblemOutcome::NoSolutions);
    assert!(matches!(
        checked.evidence_detail(),
        Some(ExactAnswer::Absolute(answer)) if answer.solutions.is_empty()
    ));
}

#[test]
fn absolute_forged_solution_is_rejected() {
    let context = EvaluationContext::default();
    let request = absolute(3);
    let ProblemStatement::Absolute(equation) = &request.statement else {
        unreachable!()
    };
    let mut candidate = solve_absolute_equation(&context, equation).unwrap();
    candidate.solutions.clear();
    assert_eq!(
        adapt_verified_problem(&context, &request, ProblemEvidence::Absolute(&candidate)),
        Err(ProblemError::InvalidEvidence)
    );
}

#[test]
fn special_angle_retains_exact_algebraic_branch() {
    let checked = stable(&special(TrigFunction::Cos, 5, 6));
    let ProblemOutcome::SolvedExact(ExactAnswer::SpecialAngle(certificate)) = checked.outcome()
    else {
        panic!("expected exact angle");
    };
    assert!(matches!(
        certificate.outcome,
        SpecialAngleOutcome::Exact(ExactQuadraticRoot::Algebraic {
            branch: QuadraticBranch::LowerReal,
            ..
        })
    ));
    assert!(
        checked
            .to_json()
            .unwrap()
            .contains("\"minimal_polynomial\"")
    );
}

#[test]
fn tangent_pole_is_checked_undefined_not_a_numeric_answer() {
    let checked = stable(&special(TrigFunction::Tan, 1, 2));
    assert_eq!(
        checked.outcome(),
        &ProblemOutcome::Undefined(UndefinedReason::TangentPole)
    );
    assert!(
        checked
            .to_json()
            .unwrap()
            .contains("\"undefined_tangent_pole\"")
    );
    assert!(checked.evidence_detail().is_some());
}

#[test]
fn special_angle_wrong_source_and_forged_tick_are_rejected() {
    let context = EvaluationContext::default();
    let request = special(TrigFunction::Sin, 1, 6);
    let mut candidate = evaluate_special_angle(
        &PiMultiple::new(q(13, 6)),
        TrigFunction::Sin,
        ScalarDomain::Real,
        TrigLimits::default(),
    )
    .unwrap();
    assert_eq!(
        adapt_verified_problem(
            &context,
            &request,
            ProblemEvidence::SpecialAngle(&candidate)
        ),
        Err(ProblemError::InvalidEvidence)
    );
    candidate.angle = PiMultiple::new(q(1, 6));
    candidate.tick = 0;
    assert_eq!(
        adapt_verified_problem(
            &context,
            &request,
            ProblemEvidence::SpecialAngle(&candidate)
        ),
        Err(ProblemError::InvalidEvidence)
    );
}

#[test]
fn trig_zeros_are_partial_and_exact() {
    let checked = stable(&zeros(TrigFunction::Cos));
    assert!(matches!(
        checked.outcome(),
        ProblemOutcome::PartialScope {
            known: ExactAnswer::TrigZeros(certificates),
            reason: UnsupportedReason::PeriodicCompletenessUnavailable,
        } if certificates.len() == 2
    ));
    assert!(
        checked
            .to_json()
            .unwrap()
            .contains("\"status\":\"partial\"")
    );
}

#[test]
fn incomplete_trig_evidence_and_unsupported_angles_are_not_complete() {
    let context = EvaluationContext::default();
    let request = zeros(TrigFunction::Sin);
    let proof =
        partial_trig_zeros(TrigFunction::Sin, ScalarDomain::Real, TrigLimits::default()).unwrap();
    assert_eq!(
        adapt_verified_problem(
            &context,
            &request,
            ProblemEvidence::TrigZeros(&proof.known_zeros()[..1])
        ),
        Err(ProblemError::InvalidEvidence)
    );
    assert!(matches!(
        stable(&special(TrigFunction::Sin, 1, 5)).outcome(),
        ProblemOutcome::Unsupported(_)
    ));
}

#[test]
fn groebner_wire_preserves_certificate_and_membership_is_not_radical_membership() {
    let basis = stable(&ideal(None));
    assert!(matches!(
        basis.outcome(),
        ProblemOutcome::SolvedExact(ExactAnswer::Groebner(_))
    ));
    assert!(basis.to_json().unwrap().contains("\"representations\""));
    let member = stable(&ideal(Some(source(Expr::pow(x().into(), 3.into())))));
    let nonmember = stable(&ideal(Some(source(x().into()))));
    assert!(member.to_json().unwrap().contains("\"member\":true"));
    assert!(nonmember.to_json().unwrap().contains("\"member\":false"));
}

#[test]
fn forged_ideal_membership_status_cannot_be_published() {
    let context = EvaluationContext::default();
    let request = ideal(Some(source(x().into())));
    let checked = dispatch_problem(&context, &request).unwrap();
    let ProblemOutcome::SolvedExact(ExactAnswer::Membership { basis, membership }) =
        checked.outcome()
    else {
        panic!("expected ideal membership query");
    };
    let IdealMembership::NonMember(reduction) = membership.as_ref() else {
        panic!("x is not in the ideal generated by x squared");
    };
    let forged = IdealMembership::Member(reduction.clone());
    assert_eq!(
        adapt_verified_problem(
            &context,
            &request,
            ProblemEvidence::Membership(basis, &forged)
        ),
        Err(ProblemError::InvalidEvidence)
    );
}

#[test]
fn restricted_ideal_keeps_history_and_refuses_unconditional_membership() {
    let reciprocal = source(Expr::pow(x().into(), (-1).into()));
    let historical = DomainExpression::multiply([source(0.into()), reciprocal], 16).unwrap();
    let checked = stable(&ideal(Some(historical)));
    assert!(matches!(checked.outcome(), ProblemOutcome::Unsupported(_)));
    assert_eq!(checked.restrictions().len(), 1);
    assert_eq!(checked.checked_by(), None);
}

#[test]
fn resource_refusal_does_not_publish_evidence() {
    let context = EvaluationContext::new(ResourceLimits {
        max_work_units: 0,
        ..ResourceLimits::default()
    });
    let checked = dispatch_problem(&context, &special(TrigFunction::Cos, 1, 3)).unwrap();
    assert_eq!(
        checked.outcome(),
        &ProblemOutcome::ResourceRefusal(ResourceLimitKind::WorkUnits)
    );
    assert_eq!(checked.checked_by(), None);
}

#[test]
fn structural_steps_are_checked_before_serialization() {
    let context = EvaluationContext::default();
    let left = source(Expr::mul([2.into(), x().into()]));
    let right = source(6.into());
    let equation = Equation::new(left.expression().clone(), right.expression().clone());
    let variable = x();
    let solution = context.solve_linear(&equation, &variable).unwrap();
    let evidence = StepEvidence::Linear {
        left: &left,
        right: &right,
        variable: &variable,
        result: &solution,
    };
    let limits = StepLimits::default();
    let mut records = record_steps(evidence, limits).unwrap();
    let json = records.to_checked_json(evidence, limits).unwrap();
    assert!(json.contains("\"kind\":\"equivalent\""));
    assert!(json.contains("\"kind\":\"proven_nonzero\""));
    assert_eq!(json, records.to_checked_json(evidence, limits).unwrap());
    records.steps[0].premises.push(0);
    assert!(matches!(
        records.to_checked_json(evidence, limits),
        Err(ProblemWireError::StepVerification(_))
    ));
}
