use mathema::{Condition, DefinednessError, DomainExpression, Expr, ScalarDomain, Symbol};
use mathematishia_kernel::{
    Assumptions, ConditionStatus, DomainPreconditionError, DomainPreconditionStatus,
    check_domain_preconditions,
};

fn x() -> Expr {
    Expr::from(Symbol::new("x").unwrap())
}

fn source(expr: Expr, domain: ScalarDomain) -> DomainExpression {
    DomainExpression::from_expr_in_domain(expr, domain, 256, 128).unwrap()
}

#[test]
fn exact_real_and_complex_log_domains_are_distinct() {
    let expr = Expr::log(Expr::pow((-2).into(), 3.into()));
    let real = check_domain_preconditions(
        &source(expr.clone(), ScalarDomain::Real),
        ScalarDomain::Real,
        &Assumptions::new(),
    )
    .unwrap();
    assert_eq!(real.status(), DomainPreconditionStatus::Refuted);
    let complex = check_domain_preconditions(
        &source(expr, ScalarDomain::Complex),
        ScalarDomain::Complex,
        &Assumptions::new(),
    )
    .unwrap();
    assert_eq!(complex.status(), DomainPreconditionStatus::Proven);
}

#[test]
fn cancellation_history_is_conditional_not_an_identity() {
    let numerator = source(x(), ScalarDomain::Real);
    let quotient = numerator.clone().divide(numerator, 8).unwrap();
    let condition = Condition::NonZero(x());
    let blocked =
        check_domain_preconditions(&quotient, ScalarDomain::Real, &Assumptions::new()).unwrap();
    assert_eq!(blocked.status(), DomainPreconditionStatus::Blocked);
    let assumptions = Assumptions::from_conditions([condition.clone()]).unwrap();
    let conditional =
        check_domain_preconditions(&quotient, ScalarDomain::Real, &assumptions).unwrap();
    assert_eq!(conditional.status(), DomainPreconditionStatus::Conditional);
    assert_eq!(
        conditional.conditions(),
        &[(condition, ConditionStatus::Assumed)]
    );
}

#[test]
fn historical_condition_does_not_prove_its_own_definedness() {
    let condition = Condition::NonZero(Expr::log(x()));
    let history = DomainExpression::with_conditions(0.into(), [condition.clone()], 32, 8).unwrap();
    let premises = Assumptions::from_conditions([condition.clone()]).unwrap();
    let report = check_domain_preconditions(&history, ScalarDomain::Real, &premises).unwrap();
    assert_eq!(report.status(), DomainPreconditionStatus::Blocked);
    assert!(
        report
            .conditions()
            .contains(&(Condition::Positive(x()), ConditionStatus::Unknown))
    );

    let premises = Assumptions::from_conditions([condition, Condition::Positive(x())]).unwrap();
    let report = check_domain_preconditions(&history, ScalarDomain::Real, &premises).unwrap();
    assert_eq!(report.status(), DomainPreconditionStatus::Conditional);
}

#[test]
fn forged_history_and_premises_cannot_override_exact_refutation() {
    let condition = Condition::NonZero(Expr::log((-1).into()));
    let history = DomainExpression::with_conditions(0.into(), [condition.clone()], 32, 8).unwrap();
    let premises = Assumptions::from_conditions([condition]).unwrap();
    assert_eq!(
        check_domain_preconditions(&history, ScalarDomain::Real, &premises)
            .unwrap()
            .status(),
        DomainPreconditionStatus::Refuted
    );

    let zero_power = Expr::pow(0.into(), 2.into());
    let premises = Assumptions::from_conditions([Condition::Positive(zero_power)]).unwrap();
    assert_eq!(
        check_domain_preconditions(
            &source(1.into(), ScalarDomain::Real),
            ScalarDomain::Real,
            &premises
        )
        .unwrap()
        .status(),
        DomainPreconditionStatus::Refuted
    );
}

#[test]
fn erased_zero_power_and_tangent_obligations_are_retained() {
    let zero = source(0.into(), ScalarDomain::Real);
    let undefined = source(Expr::pow(0.into(), 0.into()), ScalarDomain::Real);
    let folded = DomainExpression::multiply([zero.clone(), undefined], 8).unwrap();
    assert_eq!(folded.expression(), &Expr::from(0));
    assert_eq!(
        check_domain_preconditions(&folded, ScalarDomain::Real, &Assumptions::new())
            .unwrap()
            .status(),
        DomainPreconditionStatus::Refuted
    );

    let tangent = source(Expr::tan(x()), ScalarDomain::Real);
    let folded = DomainExpression::multiply([zero, tangent], 8).unwrap();
    let report =
        check_domain_preconditions(&folded, ScalarDomain::Real, &Assumptions::new()).unwrap();
    assert_eq!(report.status(), DomainPreconditionStatus::Blocked);
    assert_eq!(
        report.conditions(),
        &[(Condition::NonZero(Expr::cos(x())), ConditionStatus::Unknown)]
    );
}

#[test]
fn unsupported_history_and_domains_are_refused() {
    let unknown = Expr::function(Symbol::new("abs").unwrap(), x());
    let history =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(unknown)], 32, 8).unwrap();
    assert_eq!(
        check_domain_preconditions(&history, ScalarDomain::Real, &Assumptions::new()),
        Err(DomainPreconditionError::Definedness(
            DefinednessError::UnsupportedFunction
        ))
    );
    assert_eq!(
        check_domain_preconditions(&history, ScalarDomain::Rational, &Assumptions::new()),
        Err(DomainPreconditionError::UnsupportedDomain)
    );
    let premise = Condition::NonZero(Expr::pow(x(), x()));
    let premises = Assumptions::from_conditions([premise]).unwrap();
    assert_eq!(
        check_domain_preconditions(
            &source(1.into(), ScalarDomain::Real),
            ScalarDomain::Real,
            &premises
        ),
        Err(DomainPreconditionError::Definedness(
            DefinednessError::UnsupportedExponent
        ))
    );
}

#[test]
fn condition_and_depth_budgets_cover_history_and_premises() {
    let conditions =
        (0..129).map(|i| Condition::NonZero(Expr::from(Symbol::new(format!("x{i}")).unwrap())));
    let history = DomainExpression::with_conditions(0.into(), conditions, 32, 129).unwrap();
    assert_eq!(
        check_domain_preconditions(&history, ScalarDomain::Real, &Assumptions::new()),
        Err(DomainPreconditionError::Definedness(
            DefinednessError::ConditionLimitExceeded
        ))
    );
    let deep = (0..65).fold(x(), |arg, _| Expr::sin(arg));
    let history =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(deep)], 32, 8).unwrap();
    assert_eq!(
        check_domain_preconditions(&history, ScalarDomain::Real, &Assumptions::new()),
        Err(DomainPreconditionError::DepthLimit)
    );
}

#[test]
fn division_cannot_bypass_existing_condition_limit() {
    let numerator = source(Expr::pow(x(), (-1).into()), ScalarDomain::Real);
    let divisor = source(2.into(), ScalarDomain::Real);
    assert_eq!(
        numerator.divide(divisor, 0),
        Err(DefinednessError::ConditionLimitExceeded)
    );
}

#[test]
fn aggregate_node_and_scalar_limits_cover_historical_conditions() {
    use mathema::Integer;

    let wide = Expr::add((0..2048).map(|i| Expr::from(Symbol::new(format!("v{i}")).unwrap())));
    let history =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(wide)], 32, 8).unwrap();
    assert_eq!(
        check_domain_preconditions(&history, ScalarDomain::Real, &Assumptions::new()),
        Err(DomainPreconditionError::Definedness(
            DefinednessError::NodeLimitExceeded
        ))
    );

    let large: Integer = format!("1{}", "0".repeat(1500)).parse().unwrap();
    let history =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(large.into())], 32, 8)
            .unwrap();
    assert_eq!(
        check_domain_preconditions(&history, ScalarDomain::Real, &Assumptions::new()),
        Err(DomainPreconditionError::ScalarLimit)
    );
}

#[test]
fn checked_cancellation_preserves_history_and_requires_its_original_premise() {
    use mathematishia_kernel::{DomainRewriteOutcome, rewrite_domain_once, verify_proof_graph};

    let numerator = source(x(), ScalarDomain::Real);
    let quotient = numerator.clone().divide(numerator, 8).unwrap();
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x())]).unwrap();
    let report = check_domain_preconditions(&quotient, ScalarDomain::Real, &assumptions).unwrap();
    assert_eq!(report.status(), DomainPreconditionStatus::Conditional);
    let DomainRewriteOutcome::Applied(rewrite) =
        rewrite_domain_once(&quotient, &assumptions).unwrap()
    else {
        panic!("expected checked cancellation");
    };
    assert_eq!(rewrite.result().expression(), &Expr::from(1));
    assert_eq!(rewrite.result().conditions(), quotient.conditions());
    assert!(verify_proof_graph(quotient.expression(), &assumptions, rewrite.proof()).is_ok());
    assert!(
        verify_proof_graph(quotient.expression(), &Assumptions::new(), rewrite.proof()).is_err()
    );
}
