use mathema::{Condition, DomainExpression, Equation, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::step_records::{
    StepClaim, StepConclusion, StepEvidence, StepEvidenceSource, StepLimits, StepRecordError,
    record_steps, verify_step_records,
};
use mathematishia_kernel::{
    EvaluationContext, LinearSolution, MultivariateLimits, MultivariatePolynomial,
    RationalPolynomial, Relation, divide_multivariate,
};

fn symbol(name: &str) -> Symbol {
    Symbol::new(name).unwrap()
}

fn source(expression: Expr) -> DomainExpression {
    DomainExpression::from_expr(expression, 128).unwrap()
}

fn polynomial(coefficients: &[i64]) -> RationalPolynomial {
    RationalPolynomial::new(
        coefficients
            .iter()
            .map(|value| Rational::from(Integer::from(*value)))
            .collect(),
    )
}

#[test]
fn linear_equation_records_actual_equivalences_and_nonzero_condition() {
    let x = symbol("x");
    let left = source(Expr::add([
        Expr::mul([2.into(), x.clone().into()]),
        4.into(),
    ]));
    let right = source(10.into());
    let equation = Equation::new(left.expression().clone(), right.expression().clone());
    let result = EvaluationContext::default()
        .solve_linear(&equation, &x)
        .unwrap();
    let evidence = StepEvidence::Linear {
        left: &left,
        right: &right,
        variable: &x,
        result: &result,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    assert_eq!(records.domain, ScalarDomain::Rational);
    assert_eq!(records.steps.len(), result.steps().len());
    assert_eq!(
        records.steps.last().unwrap().claim.relation(),
        Relation::Equivalent
    );
    assert_eq!(
        records.steps.last().unwrap().rule.stable_id(),
        "equation.divide_by_nonzero_coefficient"
    );
    assert_eq!(records.steps.last().unwrap().conditions.len(), 1);
    assert!(matches!(
        records.conclusion,
        StepConclusion::Linear { solution: LinearSolution::Unique(ref root), .. }
            if root == &Rational::from(Integer::from(3))
    ));
    assert_eq!(
        record_steps(evidence, StepLimits::default()).unwrap(),
        records
    );
    verify_step_records(evidence, &records, StepLimits::default()).unwrap();

    let mut forged = records.clone();
    forged.steps.last_mut().unwrap().conditions.clear();
    assert_eq!(
        verify_step_records(evidence, &forged, StepLimits::default()),
        Err(StepRecordError::InvalidRecord)
    );
    let mut forged = records.clone();
    forged.steps[0].premises.push(0);
    assert_eq!(
        verify_step_records(evidence, &forged, StepLimits::default()),
        Err(StepRecordError::InvalidRecord)
    );
    let mut forged = records;
    forged.steps.reverse();
    assert_eq!(
        verify_step_records(evidence, &forged, StepLimits::default()),
        Err(StepRecordError::InvalidRecord)
    );
}

#[test]
fn linear_identity_keeps_classification_even_without_steps() {
    let x = symbol("x");
    let left = source(0.into());
    let right = source(0.into());
    let result = EvaluationContext::default()
        .solve_linear(&Equation::new(0.into(), 0.into()), &x)
        .unwrap();
    let evidence = StepEvidence::Linear {
        left: &left,
        right: &right,
        variable: &x,
        result: &result,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    assert!(records.steps.is_empty());
    assert!(matches!(
        records.conclusion,
        StepConclusion::Linear { solution: LinearSolution::AllRationals, .. }
    ));
}

#[test]
fn linear_evidence_cannot_erase_a_historical_exclusion() {
    let x = symbol("x");
    let left = DomainExpression::with_conditions(
        Expr::from(0),
        [Condition::NonZero(x.clone().into())],
        128,
        32,
    )
    .unwrap();
    let right = source(0.into());
    let result = EvaluationContext::default()
        .solve_linear(&Equation::new(0.into(), 0.into()), &x)
        .unwrap();
    assert_eq!(
        record_steps(
            StepEvidence::Linear {
                left: &left,
                right: &right,
                variable: &x,
                result: &result,
            },
            StepLimits::default()
        ),
        Err(StepRecordError::UnsupportedDomain)
    );
}

#[test]
fn distribution_retains_original_conditions_and_partial_status() {
    let x = Expr::from(symbol("x"));
    let expression = Expr::mul([
        Expr::add([x.clone(), 1.into()]),
        Expr::add([x.clone(), 2.into()]),
    ]);
    let source = DomainExpression::with_conditions(
        expression.clone(),
        [Condition::NonZero(x)],
        128,
        32,
    )
    .unwrap();
    let result = EvaluationContext::default()
        .with_distribution_steps(1)
        .expand(&expression)
        .unwrap();
    let evidence = StepEvidence::Distribution {
        source: &source,
        domain: ScalarDomain::Real,
        result: &result,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    assert_eq!(records.source_conditions, source.conditions());
    assert_eq!(records.steps.len(), 1);
    assert_eq!(records.steps[0].claim.relation(), Relation::Equal);
    assert_eq!(records.steps[0].conditions.len(), 1);
    assert!(matches!(
        records.conclusion,
        StepConclusion::Distribution { complete: false, .. }
    ));
    let mut forged = records.clone();
    forged.source_conditions.clear();
    assert_eq!(
        verify_step_records(evidence, &forged, StepLimits::default()),
        Err(StepRecordError::InvalidRecord)
    );
    let mut forged = records;
    forged.steps[0].claim = StepClaim::Equal {
        before: expression,
        after: 99.into(),
    };
    assert_eq!(
        verify_step_records(evidence, &forged, StepLimits::default()),
        Err(StepRecordError::InvalidRecord)
    );
}

#[test]
fn derivative_records_postorder_operands_not_a_linear_rewrite_chain() {
    let x = symbol("x");
    let expression = Expr::add([Expr::pow(x.clone().into(), 3.into()), 5.into()]);
    let source = source(expression);
    let result = EvaluationContext::default()
        .differentiate_domain_expression(&source, &x, ScalarDomain::Real)
        .unwrap();
    let evidence = StepEvidence::Derivative {
        source: &source,
        domain: ScalarDomain::Real,
        variable: &x,
        result: &result,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    let last = records.steps.last().unwrap();
    assert_eq!(
        result.derivative(),
        &Expr::mul([3.into(), Expr::pow(x.clone().into(), 2.into())])
    );
    assert_eq!(last.claim.relation(), Relation::DerivativeOf);
    assert_eq!(last.rule.stable_id(), "calculus.sum");
    assert_eq!(last.premises.len(), 2);
    assert!(last.premises.iter().all(|index| *index < last.index));
    assert_eq!(last.evidence, StepEvidenceSource::Derivative(last.index));
    let mut forged = records;
    forged.steps.last_mut().unwrap().premises.reverse();
    assert_eq!(
        verify_step_records(evidence, &forged, StepLimits::default()),
        Err(StepRecordError::InvalidRecord)
    );
}

#[test]
fn logarithm_derivative_retains_positive_domain() {
    let x = symbol("x");
    let source = DomainExpression::from_expr_in_domain(
        Expr::function(symbol("log"), x.clone().into()),
        ScalarDomain::Real,
        128,
        32,
    )
    .unwrap();
    let result = EvaluationContext::default()
        .differentiate_domain_expression(&source, &x, ScalarDomain::Real)
        .unwrap();
    let evidence = StepEvidence::Derivative {
        source: &source,
        domain: ScalarDomain::Real,
        variable: &x,
        result: &result,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    assert_eq!(
        records.source_conditions,
        vec![Condition::Positive(x.clone().into())]
    );
    assert_eq!(
        records.steps.last().unwrap().rule.stable_id(),
        "calculus.log_chain"
    );
    let wrong = symbol("y");
    assert_eq!(
        record_steps(
            StepEvidence::Derivative {
                source: &source,
                domain: ScalarDomain::Real,
                variable: &wrong,
                result: &result,
            },
            StepLimits::default()
        ),
        Err(StepRecordError::SourceMismatch)
    );
}

#[test]
fn polynomial_division_records_only_checked_final_recomposition() {
    let dividend = polynomial(&[-1, 0, 1]);
    let divisor = polynomial(&[-1, 1]);
    let quotient = polynomial(&[1, 1]);
    let remainder = polynomial(&[]);
    let evidence = StepEvidence::PolynomialDivision {
        dividend: &dividend,
        divisor: &divisor,
        quotient: &quotient,
        remainder: &remainder,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    assert_eq!(records.steps.len(), 1);
    assert_eq!(
        records.steps[0].evidence,
        StepEvidenceSource::PolynomialDivisionFinal
    );
    assert!(records.steps[0].premises.is_empty());
    verify_step_records(evidence, &records, StepLimits::default()).unwrap();
    let bad = polynomial(&[9]);
    assert_eq!(
        record_steps(
            StepEvidence::PolynomialDivision {
                dividend: &dividend,
                divisor: &divisor,
                quotient: &bad,
                remainder: &remainder,
            },
            StepLimits::default()
        ),
        Err(StepRecordError::InvalidEvidence)
    );
    let zero = polynomial(&[]);
    assert!(record_steps(
        StepEvidence::PolynomialDivision {
            dividend: &dividend,
            divisor: &zero,
            quotient: &quotient,
            remainder: &remainder,
        },
        StepLimits::default()
    ).is_err());
    assert_eq!(
        record_steps(
            StepEvidence::PolynomialDivision {
                dividend: &dividend,
                divisor: &divisor,
                quotient: &zero,
                remainder: &dividend,
            },
            StepLimits::default()
        ),
        Err(StepRecordError::InvalidEvidence)
    );
}

#[test]
fn ordered_multivariate_division_does_not_claim_unique_normal_form() {
    let limits = MultivariateLimits::default();
    let p = |terms: &[(i32, i32, i64)]| {
        MultivariatePolynomial::try_new(
            vec![symbol("x"), symbol("y")],
            terms
                .iter()
                .map(|&(x, y, c)| (vec![x, y], Rational::from(Integer::from(c)))),
            limits,
        )
        .unwrap()
    };
    let dividend = p(&[(1, 1, 1)]);
    let divisors = [
        p(&[(1, 0, 1), (0, 0, -1)]),
        p(&[(0, 1, 1), (0, 0, -2)]),
    ];
    let result = divide_multivariate(&dividend, &divisors, limits).unwrap();
    let evidence = StepEvidence::MultivariateDivision {
        dividend: &dividend,
        divisors: &divisors,
        result: &result,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    assert_eq!(records.steps.len(), 1);
    assert_eq!(
        records.steps[0].evidence,
        StepEvidenceSource::MultivariateDivisionFinal
    );
    assert_eq!(result.quotients(), &[p(&[(0, 1, 1)]), p(&[(0, 0, 1)])]);
    assert_eq!(result.remainder(), &p(&[(0, 0, 2)]));
    verify_step_records(evidence, &records, StepLimits::default()).unwrap();
    let mut forged = records;
    let StepClaim::MultivariateDivision { ordered_divisors, .. } = &mut forged.steps[0].claim else {
        panic!("expected division claim");
    };
    ordered_divisors.reverse();
    assert_eq!(
        verify_step_records(evidence, &forged, StepLimits::default()),
        Err(StepRecordError::InvalidRecord)
    );
}

#[test]
fn refuses_zero_budgets_and_unrelated_sources() {
    let source = source(Expr::mul([
        symbol("x").into(),
        Expr::add([symbol("y").into(), 1.into()]),
    ]));
    let result = EvaluationContext::default()
        .expand(source.expression())
        .unwrap();
    let evidence = StepEvidence::Distribution {
        source: &source,
        domain: ScalarDomain::Real,
        result: &result,
    };
    for limits in [
        StepLimits {
            max_steps: 0,
            ..StepLimits::default(),
        },
        StepLimits {
            max_nodes: 0,
            ..StepLimits::default(),
        },
        StepLimits {
            max_work_units: 0,
            ..StepLimits::default(),
        },
    ] {
        assert_eq!(
            record_steps(evidence, limits),
            Err(StepRecordError::ResourceLimit)
        );
    }
    let other = DomainExpression::from_expr(9.into(), 128).unwrap();
    assert_eq!(
        record_steps(
            StepEvidence::Distribution {
                source: &other,
                domain: ScalarDomain::Real,
                result: &result,
            },
            StepLimits::default()
        ),
        Err(StepRecordError::InvalidEvidence)
    );
    assert_eq!(
        record_steps(
            StepEvidence::Distribution {
                source: &source,
                domain: ScalarDomain::Integer,
                result: &result,
            },
            StepLimits::default()
        ),
        Err(StepRecordError::UnsupportedDomain)
    );
}

#[test]
fn refuted_historical_condition_is_not_accepted_as_a_premise() {
    let expression = Expr::mul([symbol("x").into(), Expr::add([1.into(), symbol("y").into()])]);
    let source = DomainExpression::with_conditions(
        expression.clone(),
        [Condition::NonZero(0.into())],
        128,
        32,
    )
    .unwrap();
    let result = EvaluationContext::default().expand(&expression).unwrap();
    let evidence = StepEvidence::Distribution {
        source: &source,
        domain: ScalarDomain::Real,
        result: &result,
    };
    assert_eq!(
        record_steps(evidence, StepLimits::default()),
        Err(StepRecordError::InvalidDomain)
    );
}

#[test]
fn inconsistent_linear_equation_retains_no_solution() {
    let x = symbol("x");
    let left = source(1.into());
    let right = source(2.into());
    let equation = Equation::new(1.into(), 2.into());
    let result = EvaluationContext::default()
        .solve_linear(&equation, &x)
        .unwrap();
    let evidence = StepEvidence::Linear {
        left: &left,
        right: &right,
        variable: &x,
        result: &result,
    };
    let records = record_steps(evidence, StepLimits::default()).unwrap();
    assert!(matches!(
        records.conclusion,
        StepConclusion::Linear {
            solution: LinearSolution::NoSolution,
            ..
        }
    ));
}

#[test]
fn unsupported_function_domain_does_not_gain_a_proof() {
    let expression = Expr::function(symbol("unknown"), symbol("x").into());
    let result = EvaluationContext::default().expand(&expression).unwrap();
    let original = source(1.into());
    assert_eq!(
        record_steps(
            StepEvidence::Distribution {
                source: &original,
                domain: ScalarDomain::Real,
                result: &result,
            },
            StepLimits::default()
        ),
        Err(StepRecordError::InvalidDomain)
    );
}
