use mathema::{Condition, Expr, Integer, Rational, ScalarDomain, Symbol};
use mathematishia_kernel::{
    AssumptionError, Assumptions, ConditionStatus, GuardedRewriteOutcome,
    cancel_symbol_inverse_once, verify_guarded_rewrite_step,
};

fn symbol(name: &str) -> Expr {
    Expr::from(Symbol::new(name).unwrap())
}

#[test]
fn rational_facts_are_exact_and_cannot_be_overridden() {
    let mut assumptions = Assumptions::new();
    let half = Expr::from(Rational::new(Integer::one(), Integer::from(2)).unwrap());

    assert_eq!(
        assumptions.assess(&Condition::NonZero(half.clone())),
        ConditionStatus::Proven
    );
    assert_eq!(
        assumptions.assess(&Condition::Positive(half.clone())),
        ConditionStatus::Proven
    );
    assert_eq!(
        assumptions.assess(&Condition::Integer(half.clone())),
        ConditionStatus::Refuted
    );
    assert_eq!(
        assumptions.assess(&Condition::InDomain {
            expression: half,
            domain: ScalarDomain::Complex
        }),
        ConditionStatus::Proven
    );
    assert_eq!(
        assumptions.add(Condition::NonZero(Expr::from(0))),
        Err(AssumptionError::RefutedPremise(Condition::NonZero(
            Expr::from(0)
        )))
    );
    assert!(assumptions.premises().is_empty());
}

#[test]
fn premises_are_distinguished_from_proven_facts() {
    let x = symbol("x");
    let y = symbol("y");
    let mut assumptions = Assumptions::new();
    assert_eq!(
        assumptions.assess(&Condition::NonZero(x.clone())),
        ConditionStatus::Unknown
    );
    assumptions.add(Condition::Positive(x.clone())).unwrap();
    assert_eq!(
        assumptions.assess(&Condition::NonZero(x.clone())),
        ConditionStatus::Assumed
    );
    assert_eq!(
        assumptions.assess(&Condition::Real(x.clone())),
        ConditionStatus::Assumed
    );
    assert_eq!(
        assumptions.assess(&Condition::InDomain {
            expression: x.clone(),
            domain: ScalarDomain::Complex
        }),
        ConditionStatus::Assumed
    );
    assert_eq!(
        assumptions.assess(&Condition::Integer(x.clone())),
        ConditionStatus::Unknown
    );
    assert_eq!(
        assumptions.assess(&Condition::NonZero(y)),
        ConditionStatus::Unknown
    );
    assumptions.add(Condition::NonZero(x.clone())).unwrap();
    assert_eq!(assumptions.premises(), &[Condition::Positive(x)]);
}

#[test]
fn domain_membership_implications_are_one_way() {
    let x = symbol("x");
    let assumptions = Assumptions::from_conditions([Condition::InDomain {
        expression: x.clone(),
        domain: ScalarDomain::Rational,
    }])
    .unwrap();
    assert_eq!(
        assumptions.assess(&Condition::Real(x.clone())),
        ConditionStatus::Assumed
    );
    assert_eq!(
        assumptions.assess(&Condition::Integer(x.clone())),
        ConditionStatus::Unknown
    );
    assert_eq!(
        assumptions.assess(&Condition::NonZero(x)),
        ConditionStatus::Unknown
    );
}

#[test]
fn symbolic_inverse_cancellation_requires_and_checks_nonzero() {
    let x = symbol("x");
    let y = symbol("y");
    let expression = Expr::mul([x.clone(), Expr::pow(x.clone(), Expr::from(-1)), y.clone()]);
    let empty = Assumptions::new();
    assert_eq!(
        cancel_symbol_inverse_once(&expression, &empty),
        GuardedRewriteOutcome::Blocked(Condition::NonZero(x.clone()))
    );

    let assumptions = Assumptions::from_conditions([Condition::Positive(x.clone())]).unwrap();
    let step = match cancel_symbol_inverse_once(&expression, &assumptions) {
        GuardedRewriteOutcome::Applied(step) => step,
        other => panic!("expected applied rewrite, got {other:?}"),
    };
    assert_eq!(step.after(), &y);
    assert_eq!(step.condition(), &Condition::NonZero(x));
    assert_eq!(step.justification(), ConditionStatus::Assumed);
    assert_eq!(verify_guarded_rewrite_step(&step, &assumptions), Ok(()));
    assert!(verify_guarded_rewrite_step(&step, &empty).is_err());
}

#[test]
fn inverse_rule_does_not_assume_definedness_of_composite_bases() {
    let x = symbol("x");
    let composite = Expr::add([x.clone(), Expr::from(1)]);
    let expression = Expr::mul([composite.clone(), Expr::pow(composite, Expr::from(-1))]);
    let assumptions = Assumptions::from_conditions([Condition::NonZero(x)]).unwrap();
    assert_eq!(
        cancel_symbol_inverse_once(&expression, &assumptions),
        GuardedRewriteOutcome::NotApplicable
    );
}
