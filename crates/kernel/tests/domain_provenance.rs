use mathema::{Condition, DefinednessError, DomainExpression, Expr, Integer, ScalarDomain, Symbol};
use mathematishia_kernel::{
    Assumptions, DomainPreconditionError, DomainPreconditionReport, DomainPreconditionStatus,
    DomainRewriteOutcome, check_domain_preconditions, rewrite_domain_once, verify_proof_graph,
};

fn x() -> Expr {
    Symbol::new("x").unwrap().into()
}

fn bound(expression: Expr, domain: ScalarDomain) -> DomainExpression {
    DomainExpression::from_expr_in_domain(expression, domain, 128, 16).unwrap()
}

fn unbound(expression: Expr) -> DomainExpression {
    DomainExpression::from_expr(expression, 128).unwrap()
}

#[test]
fn erased_complex_log_cannot_be_reinterpreted_as_real() {
    let logarithm = bound(Expr::log((-1).into()), ScalarDomain::Complex);
    let erased = DomainExpression::multiply([unbound(0.into()), logarithm], 16).unwrap();
    assert_eq!(erased.expression(), &Expr::from(0));
    assert_eq!(erased.conditions(), &[Condition::NonZero((-1).into())]);
    assert_eq!(erased.construction_domain(), Some(ScalarDomain::Complex));
    let assumptions = Assumptions::new();
    assert_eq!(
        check_domain_preconditions(&erased, ScalarDomain::Complex, &assumptions)
            .unwrap()
            .status(),
        DomainPreconditionStatus::Proven
    );
    assert_eq!(
        check_domain_preconditions(&erased, ScalarDomain::Real, &assumptions),
        Err(DomainPreconditionError::Definedness(
            DefinednessError::DomainMismatch {
                left: ScalarDomain::Complex,
                right: ScalarDomain::Real,
            }
        ))
    );
}

#[test]
fn real_and_complex_values_are_distinct_even_with_identical_restrictions() {
    let real = bound(Expr::sin(x()), ScalarDomain::Real);
    let complex = bound(Expr::sin(x()), ScalarDomain::Complex);
    assert_eq!(real.expression(), complex.expression());
    assert_eq!(real.conditions(), complex.conditions());
    assert_ne!(real, complex);
    let mismatch = DefinednessError::DomainMismatch {
        left: ScalarDomain::Real,
        right: ScalarDomain::Complex,
    };
    assert_eq!(
        DomainExpression::add([real.clone(), complex.clone()], 16),
        Err(mismatch)
    );
    assert_eq!(
        DomainExpression::multiply([real.clone(), complex.clone()], 16),
        Err(mismatch)
    );
    assert_eq!(real.clone().divide(complex.clone(), 16), Err(mismatch));
    assert_eq!(
        real.clone().pow(bound(2.into(), ScalarDomain::Complex), 16),
        Err(mismatch)
    );
    assert_eq!(
        DomainExpression::function(Symbol::new("exp").unwrap(), real, ScalarDomain::Complex, 16),
        Err(mismatch)
    );
}

#[test]
fn zero_power_and_exponent_history_keep_construction_domain() {
    let real = bound(Expr::log(x()), ScalarDomain::Real);
    let erased = DomainExpression::multiply([real, unbound(0.into())], 16).unwrap();
    let power = unbound(x()).pow(erased, 16).unwrap();
    assert_eq!(power.construction_domain(), Some(ScalarDomain::Real));
    assert_eq!(
        power.conditions(),
        &[Condition::Positive(x()), Condition::NonZero(x())]
    );
    let square = power.pow_integer(Integer::from(2), 16).unwrap();
    assert_eq!(square.construction_domain(), Some(ScalarDomain::Real));
    let mismatch = DefinednessError::DomainMismatch {
        left: ScalarDomain::Real,
        right: ScalarDomain::Complex,
    };
    assert_eq!(
        square.divide(bound(2.into(), ScalarDomain::Complex), 16),
        Err(mismatch)
    );
}

#[test]
fn same_domain_composition_and_neutral_constants_preserve_history() {
    let x = x();
    let numerator = bound(x.clone(), ScalarDomain::Real);
    let quotient = numerator.clone().divide(numerator, 16).unwrap();
    let sum = DomainExpression::add([unbound(3.into()), quotient], 16).unwrap();
    assert_eq!(sum.construction_domain(), Some(ScalarDomain::Real));
    assert_eq!(sum.conditions(), &[Condition::NonZero(x.clone())]);
    let premises = Assumptions::from_conditions([Condition::NonZero(x)]).unwrap();
    let report = check_domain_preconditions(&sum, ScalarDomain::Real, &premises).unwrap();
    assert_eq!(report.status(), DomainPreconditionStatus::Conditional);
    assert_eq!(
        check_domain_preconditions(&sum, ScalarDomain::Real, &premises).unwrap(),
        report
    );
}

#[test]
fn rebuilding_retains_source_provenance_and_collects_new_obligations() {
    let source = bound(Expr::log(x()), ScalarDomain::Real);
    let rebuilt = source
        .rebuild_with_expression(Expr::tan(x()), 128, 16)
        .unwrap();
    assert_eq!(rebuilt.construction_domain(), Some(ScalarDomain::Real));
    assert_eq!(
        rebuilt.conditions(),
        &[Condition::Positive(x()), Condition::NonZero(Expr::cos(x()))]
    );
    assert_eq!(
        source.rebuild_with_expression(0.into(), 128, 0),
        Err(DefinednessError::ConditionLimitExceeded)
    );
    assert_eq!(
        source.rebuild_with_expression(Expr::tan(x()), 128, 1),
        Err(DefinednessError::ConditionLimitExceeded)
    );
    assert_eq!(
        source.rebuild_with_expression(Expr::tan(x()), 1, 16),
        Err(DefinednessError::NodeLimitExceeded)
    );
    let unknown = Expr::function(Symbol::new("abs").unwrap(), x());
    assert_eq!(
        source.rebuild_with_expression(unknown, 128, 16),
        Err(DefinednessError::UnsupportedFunction)
    );
    assert_eq!(
        source.rebuild_with_expression(Expr::pow(x(), x()), 128, 16),
        Err(DefinednessError::UnsupportedExponent)
    );
}

#[test]
fn external_history_is_unbound_and_still_requires_checked_premises() {
    let history =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(Expr::log(x()))], 128, 16)
            .unwrap();
    assert_eq!(history.construction_domain(), None);
    let assumptions = Assumptions::from_conditions(history.conditions().iter().cloned()).unwrap();
    let report = check_domain_preconditions(&history, ScalarDomain::Real, &assumptions).unwrap();
    assert_eq!(report.status(), DomainPreconditionStatus::Blocked);
    assert!(
        report
            .conditions()
            .iter()
            .any(|(condition, _)| condition == &Condition::Positive(x()))
    );
}

#[test]
fn node_limit_counts_exponents_and_refuses_wide_trees() {
    let expression = Expr::pow(x(), (-1).into());
    assert_eq!(
        DomainExpression::from_expr(expression.clone(), 2),
        Err(DefinednessError::NodeLimitExceeded)
    );
    assert_eq!(
        DomainExpression::from_expr(expression, 3)
            .unwrap()
            .conditions(),
        &[Condition::NonZero(x())]
    );
    let wide = Expr::add((0..128).map(|i| Expr::from(Symbol::new(format!("x{i}")).unwrap())));
    assert_eq!(
        DomainExpression::from_expr(wide, 128),
        Err(DefinednessError::NodeLimitExceeded)
    );
}

#[test]
fn strict_admission_distinguishes_legacy_and_declared_sources() {
    let assumptions = Assumptions::new();
    let legacy = unbound(x());
    let declared = bound(x(), ScalarDomain::Real);
    let report = check_domain_preconditions(&legacy, ScalarDomain::Real, &assumptions).unwrap();
    assert_eq!(report.construction_domain(), None);
    assert_eq!(report.status(), DomainPreconditionStatus::Proven);
    assert_eq!(
        DomainPreconditionReport::check_bound(&legacy, ScalarDomain::Real, &assumptions),
        Err(DomainPreconditionError::UnboundSource)
    );
    let report =
        DomainPreconditionReport::check_bound(&declared, ScalarDomain::Real, &assumptions).unwrap();
    assert_eq!(report.construction_domain(), Some(ScalarDomain::Real));
    assert!(!report.has_external_history());
}

#[test]
fn external_history_cannot_be_certified_by_composition() {
    let history =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(x())], 128, 16).unwrap();
    let declared = bound(2.into(), ScalarDomain::Real);
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x())]).unwrap();
    let combined = [
        DomainExpression::add([history.clone(), declared.clone()], 16).unwrap(),
        DomainExpression::multiply([history.clone(), declared.clone()], 16).unwrap(),
        history.clone().divide(declared.clone(), 16).unwrap(),
        declared.clone().pow(history.clone(), 16).unwrap(),
        DomainExpression::function(Symbol::new("exp").unwrap(), history, ScalarDomain::Real, 16)
            .unwrap(),
    ];
    for source in combined {
        assert_eq!(source.construction_domain(), Some(ScalarDomain::Real));
        assert!(source.has_external_history());
        let rebuilt = source.rebuild_with_expression(1.into(), 128, 16).unwrap();
        assert!(rebuilt.has_external_history());
        assert!(
            rebuilt
                .pow_integer(Integer::from(2), 16)
                .unwrap()
                .has_external_history()
        );
        assert_eq!(
            DomainPreconditionReport::check_bound(&source, ScalarDomain::Real, &assumptions),
            Err(DomainPreconditionError::UnboundSource)
        );
    }
}

fn inverse_pair() -> DomainExpression {
    let value = bound(x(), ScalarDomain::Real);
    value.clone().divide(value, 16).unwrap()
}

#[test]
fn checked_rewrite_reports_retained_source_assumptions() {
    let source = inverse_pair();
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x())]).unwrap();
    let DomainRewriteOutcome::Applied(rewrite) =
        rewrite_domain_once(&source, &assumptions).unwrap()
    else {
        panic!("expected cancellation");
    };
    assert_eq!(rewrite.result().conditions(), source.conditions());
    assert_eq!(
        rewrite.result().construction_domain(),
        Some(ScalarDomain::Real)
    );
    assert_eq!(
        rewrite.preconditions().status(),
        DomainPreconditionStatus::Conditional
    );
    assert_eq!(
        rewrite.preconditions().construction_domain(),
        Some(ScalarDomain::Real)
    );
    assert!(verify_proof_graph(source.expression(), &assumptions, rewrite.proof()).is_ok());
}

#[test]
fn rewrite_cannot_trust_undefined_historical_premise() {
    let history_condition = Condition::NonZero(Expr::log((-1).into()));
    let history =
        DomainExpression::with_conditions(0.into(), [history_condition.clone()], 128, 16).unwrap();
    let source = DomainExpression::add([inverse_pair(), history], 16).unwrap();
    let assumptions =
        Assumptions::from_conditions([Condition::NonZero(x()), history_condition]).unwrap();
    assert_eq!(
        rewrite_domain_once(&source, &assumptions).unwrap(),
        DomainRewriteOutcome::Refuted(Condition::Positive((-1).into()))
    );
}

#[test]
fn rewrite_blocks_missing_nested_historical_condition() {
    let y = Expr::from(Symbol::new("y").unwrap());
    let history_condition = Condition::NonZero(Expr::log(y.clone()));
    let history =
        DomainExpression::with_conditions(0.into(), [history_condition.clone()], 128, 16).unwrap();
    let source = DomainExpression::add([inverse_pair(), history], 16).unwrap();
    let assumptions =
        Assumptions::from_conditions([Condition::NonZero(x()), history_condition]).unwrap();
    assert_eq!(
        rewrite_domain_once(&source, &assumptions).unwrap(),
        DomainRewriteOutcome::Blocked(vec![Condition::Positive(y)])
    );
}

#[test]
fn legacy_rewrite_refuses_ambiguous_analytic_history() {
    let history =
        DomainExpression::with_conditions(0.into(), [Condition::NonZero(Expr::log(x()))], 128, 16)
            .unwrap();
    let assumptions = Assumptions::from_conditions(history.conditions().iter().cloned()).unwrap();
    assert!(rewrite_domain_once(&history, &assumptions).is_err());
}

#[test]
fn rewrite_refuses_refuted_unused_premises() {
    let assumptions = Assumptions::from_conditions([
        Condition::NonZero(x()),
        Condition::Positive(Expr::pow(0.into(), 2.into())),
    ])
    .unwrap();
    assert_eq!(
        rewrite_domain_once(&inverse_pair(), &assumptions).unwrap(),
        DomainRewriteOutcome::Refuted(Condition::Positive(Expr::pow(0.into(), 2.into())))
    );
}
