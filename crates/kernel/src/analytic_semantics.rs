//! Checked preconditions for the existing real and complex scalar grammar.

use std::{cmp::Ordering, error::Error, fmt};

use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ExprView, Integer, Rational, ScalarDomain,
    definedness_obligations, definedness_obligations_in_domain,
};

use crate::{Assumptions, ConditionStatus, ExactQuadraticRoot, QuadraticBranch, RationalPolynomial};

const MAX_NODES: usize = 2_048;
const MAX_DEPTH: usize = 64;
const MAX_CONDITIONS: usize = 128;
const MAX_SCALAR_BITS: u64 = 4_096;
const MAX_NAME_BYTES: usize = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainPreconditionError {
    UnsupportedDomain,
    UnboundSource,
    Definedness(DefinednessError),
    DepthLimit,
    ScalarLimit,
    NameLimit,
}

impl fmt::Display for DomainPreconditionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDomain => f.write_str("unsupported scalar domain"),
            Self::UnboundSource => f.write_str("source has unbound or externally supplied history"),
            Self::Definedness(error) => error.fmt(f),
            Self::DepthLimit => f.write_str("domain precondition depth limit exceeded"),
            Self::ScalarLimit => f.write_str("domain precondition scalar limit exceeded"),
            Self::NameLimit => f.write_str("domain precondition name limit exceeded"),
        }
    }
}

impl Error for DomainPreconditionError {}

impl From<DefinednessError> for DomainPreconditionError {
    fn from(error: DefinednessError) -> Self {
        Self::Definedness(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainPreconditionStatus {
    Proven,
    Conditional,
    Blocked,
    Refuted,
}

/// The assessment includes original restrictions, their nested obligations,
/// and explicit premises. It is not a certificate of expression equivalence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainPreconditionReport {
    status: DomainPreconditionStatus,
    construction_domain: Option<ScalarDomain>,
    external_history: bool,
    conditions: Vec<(Condition, ConditionStatus)>,
}

impl DomainPreconditionReport {
    pub fn status(&self) -> DomainPreconditionStatus {
        self.status
    }

    /// The domain declared during construction, not the caller's requested domain.
    pub fn construction_domain(&self) -> Option<ScalarDomain> {
        self.construction_domain
    }

    pub fn has_external_history(&self) -> bool {
        self.external_history
    }

    /// Requires a declared construction domain without externally supplied
    /// restrictions. This cannot recover history erased in a raw Expr before
    /// DomainExpression construction, or prove transformation equivalence.
    pub fn check_bound(
        source: &DomainExpression,
        domain: ScalarDomain,
        assumptions: &Assumptions,
    ) -> Result<Self, DomainPreconditionError> {
        source.check_construction_domain(domain)?;
        if source.construction_domain().is_none() || source.has_external_history() {
            return Err(DomainPreconditionError::UnboundSource);
        }
        check_domain_preconditions(source, domain, assumptions)
    }

    pub fn conditions(&self) -> &[(Condition, ConditionStatus)] {
        &self.conditions
    }
}

fn inspect(expression: &Expr, remaining: &mut usize) -> Result<(), DomainPreconditionError> {
    let mut pending = vec![(expression, 0)];
    while let Some((node, depth)) = pending.pop() {
        *remaining = remaining
            .checked_sub(1)
            .ok_or(DefinednessError::NodeLimitExceeded)?;
        if depth > MAX_DEPTH {
            return Err(DomainPreconditionError::DepthLimit);
        }
        match node.view() {
            ExprView::Integer(n) if n.bit_len() > MAX_SCALAR_BITS => {
                return Err(DomainPreconditionError::ScalarLimit);
            }
            ExprView::Rational(q)
                if q.numerator().bit_len() > MAX_SCALAR_BITS
                    || q.denominator().bit_len() > MAX_SCALAR_BITS =>
            {
                return Err(DomainPreconditionError::ScalarLimit);
            }
            ExprView::Symbol(s) if s.name().len() > MAX_NAME_BYTES => {
                return Err(DomainPreconditionError::NameLimit);
            }
            ExprView::Function { name, argument } => {
                if name.name().len() > MAX_NAME_BYTES {
                    return Err(DomainPreconditionError::NameLimit);
                }
                pending.push((argument, depth + 1));
            }
            ExprView::Pow { base, exponent } => {
                pending.push((exponent, depth + 1));
                pending.push((base, depth + 1));
            }
            ExprView::Add(children) | ExprView::Mul(children) => {
                if children.len() > remaining.saturating_sub(pending.len()) {
                    return Err(DefinednessError::NodeLimitExceeded.into());
                }
                pending.extend(children.iter().map(|child| (child, depth + 1)));
            }
            _ => {}
        }
        if pending.len() > *remaining {
            return Err(DefinednessError::NodeLimitExceeded.into());
        }
    }
    Ok(())
}

fn append(
    conditions: &mut Vec<Condition>,
    condition: Condition,
) -> Result<(), DomainPreconditionError> {
    if !conditions.contains(&condition) {
        if conditions.len() == MAX_CONDITIONS {
            return Err(DefinednessError::ConditionLimitExceeded.into());
        }
        conditions.push(condition);
    }
    Ok(())
}

fn collect(
    expression: &Expr,
    domain: Option<ScalarDomain>,
    conditions: &mut Vec<Condition>,
) -> Result<(), DomainPreconditionError> {
    let required = match domain {
        Some(domain) => definedness_obligations_in_domain(expression, MAX_NODES, domain),
        None => definedness_obligations(expression, MAX_NODES),
    }?;
    for condition in required {
        append(conditions, condition)?;
    }
    Ok(())
}

// Sign of exact literals, their integer powers and products, without expanding
// large powers. Zero to a nonpositive power has no sign.
fn exact_sign(expression: &Expr) -> Option<Ordering> {
    use Ordering::{Equal, Greater, Less};

    match expression.view() {
        ExprView::Integer(n) => Some(n.cmp(&Integer::zero())),
        ExprView::Rational(q) => Some(q.numerator().cmp(&Integer::zero())),
        ExprView::Pow { base, exponent } => {
            let ExprView::Integer(n) = exponent.view() else {
                return None;
            };
            match exact_sign(base)? {
                Equal if n <= &Integer::zero() => None,
                Equal => Some(Equal),
                Greater => Some(Greater),
                Less => {
                    if n.modulo_u32(2) == Some(0) {
                        Some(Greater)
                    } else {
                        Some(Less)
                    }
                }
            }
        }
        ExprView::Mul(factors) => {
            let mut negative = false;
            for factor in factors {
                match exact_sign(factor)? {
                    Equal => return Some(Equal),
                    Less => negative = !negative,
                    Greater => {}
                }
            }
            Some(if negative { Less } else { Greater })
        }
        _ => None,
    }
}

fn assess(condition: &Condition, assumptions: &Assumptions) -> ConditionStatus {
    let exact = match condition {
        Condition::NonZero(expr) => exact_sign(expr).map(|sign| sign != Ordering::Equal),
        Condition::Positive(expr) => exact_sign(expr).map(|sign| sign == Ordering::Greater),
        _ => None,
    };
    if let Some(holds) = exact {
        return if holds {
            ConditionStatus::Proven
        } else {
            ConditionStatus::Refuted
        };
    }
    // Exact refutations take precedence over implications from premises.
    let independent = Assumptions::new().assess(condition);
    if independent == ConditionStatus::Refuted {
        independent
    } else {
        assumptions.assess(condition)
    }
}

/// Inspect a normalized source and all retained and explicit conditions in a
/// selected real or complex domain. Symbols range over that domain; this
/// checks definedness, not complex branch identities or arbitrary membership.
/// Bound sources must be checked in their original construction domain.
/// For legacy unbound sources the caller supplies the domain; admission does
/// not recover missing provenance and is not a domain conversion API.
///
/// All input trees share a fixed node budget. Assumed conditions never become
/// unconditional facts. Unrecognized functions and noninteger powers refuse
/// admission even in erased construction history or unused premises.
///
/// Raw Expr normalization cannot recover history that was discarded before a
/// DomainExpression was constructed. Callers must retain the original source
/// and independently verify any proposed transformation.
pub fn check_domain_preconditions(
    source: &DomainExpression,
    domain: ScalarDomain,
    assumptions: &Assumptions,
) -> Result<DomainPreconditionReport, DomainPreconditionError> {
    if !matches!(domain, ScalarDomain::Real | ScalarDomain::Complex) {
        return Err(DomainPreconditionError::UnsupportedDomain);
    }
    source.check_construction_domain(domain)?;
    check_preconditions(source, Some(domain), assumptions)
}

/// The legacy rewrite API has no domain argument. Unbound sources must use
/// the domain-independent algebraic grammar, including historical conditions.
pub(crate) fn check_rewrite_preconditions(
    source: &DomainExpression,
    assumptions: &Assumptions,
) -> Result<DomainPreconditionReport, DefinednessError> {
    let domain = source
        .construction_domain()
        .filter(|domain| matches!(domain, ScalarDomain::Real | ScalarDomain::Complex));
    check_preconditions(source, domain, assumptions).map_err(|error| match error {
        DomainPreconditionError::Definedness(error) => error,
        _ => DefinednessError::NodeLimitExceeded,
    })
}

fn check_preconditions(
    source: &DomainExpression,
    domain: Option<ScalarDomain>,
    assumptions: &Assumptions,
) -> Result<DomainPreconditionReport, DomainPreconditionError> {
    if source
        .conditions()
        .len()
        .saturating_add(assumptions.premises().len())
        > MAX_CONDITIONS
    {
        return Err(DefinednessError::ConditionLimitExceeded.into());
    }
    let mut remaining = MAX_NODES;
    inspect(source.expression(), &mut remaining)?;
    let inputs = source.conditions().iter().chain(assumptions.premises());
    for condition in inputs.clone() {
        inspect(condition.expression(), &mut remaining)?;
    }

    let mut conditions = Vec::new();
    for condition in inputs {
        append(&mut conditions, condition.clone())?;
        collect(condition.expression(), domain, &mut conditions)?;
    }
    collect(source.expression(), domain, &mut conditions)?;

    let conditions: Vec<_> = conditions
        .into_iter()
        .map(|condition| {
            let status = assess(&condition, assumptions);
            (condition, status)
        })
        .collect();
    let has = |status| conditions.iter().any(|(_, actual)| *actual == status);
    let status = if has(ConditionStatus::Refuted) {
        DomainPreconditionStatus::Refuted
    } else if has(ConditionStatus::Unknown) {
        DomainPreconditionStatus::Blocked
    } else if has(ConditionStatus::Assumed) {
        DomainPreconditionStatus::Conditional
    } else {
        DomainPreconditionStatus::Proven
    };

    Ok(DomainPreconditionReport {
        status,
        construction_domain: source.construction_domain(),
        external_history: source.has_external_history(),
        conditions,
    })
}

/// A real principal square-root operation with its original radicand and
/// checked source restrictions. The result is only valid on that source domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrincipalSquareRootClaim {
    pub source: DomainExpression,
    pub preconditions: DomainPreconditionReport,
    pub value: PrincipalSquareRootValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrincipalSquareRootValue {
    Exact(ExactQuadraticRoot),
    Symbolic(DomainExpression),
    Undefined,
    Unsupported,
}

impl PrincipalSquareRootClaim {
    pub fn exact_value(&self) -> Option<&ExactQuadraticRoot> {
        match &self.value {
            PrincipalSquareRootValue::Exact(value) => Some(value),
            _ => None,
        }
    }

    pub fn symbolic_value(&self) -> Option<&DomainExpression> {
        match &self.value {
            PrincipalSquareRootValue::Symbolic(value) => Some(value),
            _ => None,
        }
    }

    pub fn is_undefined(&self) -> bool {
        self.value == PrincipalSquareRootValue::Undefined
    }

    pub fn is_unsupported(&self) -> bool {
        self.value == PrincipalSquareRootValue::Unsupported
    }
}

fn literal_rational(expression: &Expr) -> Option<Rational> {
    match expression.view() {
        ExprView::Integer(value) => Some(value.clone().into()),
        ExprView::Rational(value) => Some(value.clone()),
        _ => None,
    }
}

fn square_base(expression: &Expr) -> Option<&Expr> {
    match expression.view() {
        ExprView::Pow { base, exponent } if exponent == &Expr::from(2) => Some(base),
        _ => None,
    }
}

fn square_root_polynomial(radicand: &Rational) -> RationalPolynomial {
    RationalPolynomial::new(vec![
        -radicand,
        Integer::zero().into(),
        Integer::one().into(),
    ])
}

fn bounded_scalar(value: &Rational) -> bool {
    value.numerator().bit_len() <= MAX_SCALAR_BITS
        && value.denominator().bit_len() <= MAX_SCALAR_BITS
}

impl DomainPreconditionReport {
    /// Evaluate a real principal square root of an explicitly bound radicand.
    /// Admits rational literals and syntactic squares, i.e. a bounded subset
    /// of exponent 1/2. Other rational powers and variable powers are refused.
    /// A square rewrites to abs(base), never to base. Unknown source
    /// conditions remain prerequisites, not unconditional facts.
    pub fn principal_square_root(
        source: &DomainExpression,
        assumptions: &Assumptions,
    ) -> Result<PrincipalSquareRootClaim, DomainPreconditionError> {
        let preconditions = Self::check_bound(source, ScalarDomain::Real, assumptions)?;
        let value = if preconditions.status() == DomainPreconditionStatus::Refuted {
            PrincipalSquareRootValue::Undefined
        } else if let Some(radicand) = literal_rational(source.expression()) {
            if radicand.numerator() < &Integer::zero() {
                PrincipalSquareRootValue::Undefined
            } else if let Some(root) = radicand.exact_sqrt() {
                PrincipalSquareRootValue::Exact(ExactQuadraticRoot::Rational(root))
            } else {
                PrincipalSquareRootValue::Exact(ExactQuadraticRoot::Algebraic {
                    minimal_polynomial: square_root_polynomial(&radicand),
                    branch: QuadraticBranch::UpperReal,
                })
            }
        } else if let Some(base) = square_base(source.expression()) {
            PrincipalSquareRootValue::Symbolic(source.rebuild_with_expression(
                Expr::abs(base.clone()),
                MAX_NODES,
                MAX_CONDITIONS,
            )?)
        } else {
            PrincipalSquareRootValue::Unsupported
        };
        Ok(PrincipalSquareRootClaim {
            source: source.clone(),
            preconditions,
            value,
        })
    }

    /// Independently check the source, domain report, branch and local identity.
    /// Does not invoke the square-root generator.
    pub fn verify_principal_square_root(
        expected_source: &DomainExpression,
        assumptions: &Assumptions,
        claim: &PrincipalSquareRootClaim,
    ) -> Result<bool, DomainPreconditionError> {
        let report = Self::check_bound(expected_source, ScalarDomain::Real, assumptions)?;
        Self::check_bound(&claim.source, ScalarDomain::Real, assumptions)?;
        if claim.source != *expected_source || claim.preconditions != report {
            return Ok(false);
        }
        let literal = literal_rational(expected_source.expression());
        if report.status() == DomainPreconditionStatus::Refuted
            || literal
                .as_ref()
                .is_some_and(|value| value.numerator() < &Integer::zero())
        {
            return Ok(claim.is_undefined());
        }
        if let Some(radicand) = literal {
            return Ok(match &claim.value {
                PrincipalSquareRootValue::Exact(ExactQuadraticRoot::Rational(root)) => {
                    bounded_scalar(root)
                        && root.numerator() >= &Integer::zero()
                        && root * root == radicand
                }
                PrincipalSquareRootValue::Exact(ExactQuadraticRoot::Algebraic {
                    minimal_polynomial,
                    branch,
                }) => {
                    *branch == QuadraticBranch::UpperReal
                        && minimal_polynomial.coefficients().len() == 3
                        && minimal_polynomial.coefficients().iter().all(bounded_scalar)
                        && *minimal_polynomial == square_root_polynomial(&radicand)
                        && radicand.exact_sqrt().is_none()
                }
                _ => false,
            });
        }
        if let Some(base) = square_base(expected_source.expression()) {
            let Some(result) = claim.symbolic_value() else {
                return Ok(false);
            };
            Self::check_bound(result, ScalarDomain::Real, assumptions)?;
            return Ok(result.expression() == &Expr::abs(base.clone())
                && result.conditions() == expected_source.conditions());
        }
        Ok(claim.is_unsupported())
    }
}

#[cfg(test)]
mod principal_root_tests {
    use super::*;
    use mathema::Symbol;

    fn bound(expression: Expr) -> DomainExpression {
        DomainExpression::from_expr_in_domain(
            expression,
            ScalarDomain::Real,
            MAX_NODES,
            MAX_CONDITIONS,
        )
        .unwrap()
    }

    fn claim(source: &DomainExpression) -> PrincipalSquareRootClaim {
        DomainPreconditionReport::principal_square_root(source, &Assumptions::new()).unwrap()
    }

    fn verify(source: &DomainExpression, claim: &PrincipalSquareRootClaim) -> bool {
        DomainPreconditionReport::verify_principal_square_root(source, &Assumptions::new(), claim)
            .unwrap()
    }

    #[test]
    fn literal_roots_are_exact_and_nonnegative() {
        let fraction = Rational::new(9.into(), 25.into()).unwrap();
        let fraction_root = Rational::new(3.into(), 5.into()).unwrap();
        for (radicand, expected) in [
            (16.into(), Rational::from(Integer::from(4))),
            (0.into(), Rational::from(Integer::zero())),
            (fraction.into(), fraction_root),
        ] {
            let source = bound(radicand);
            let result = claim(&source);
            assert_eq!(
                result.exact_value(),
                Some(&ExactQuadraticRoot::Rational(expected))
            );
            assert!(verify(&source, &result));
        }
    }

    #[test]
    fn irrational_root_has_checked_upper_branch() {
        let source = bound(2.into());
        let mut result = claim(&source);
        assert_eq!(
            result.exact_value().unwrap().branch(),
            Some(QuadraticBranch::UpperReal)
        );
        assert!(verify(&source, &result));
        result.value = PrincipalSquareRootValue::Exact(ExactQuadraticRoot::Algebraic {
            minimal_polynomial: square_root_polynomial(&Integer::from(2).into()),
            branch: QuadraticBranch::LowerReal,
        });
        assert!(!verify(&source, &result));
    }

    #[test]
    fn negative_radicand_and_general_symbol_are_distinct() {
        let negative = bound((-1).into());
        let result = claim(&negative);
        assert!(result.is_undefined());
        assert!(verify(&negative, &result));
        let x = bound(Symbol::new("x").unwrap().into());
        assert!(claim(&x).is_unsupported());
    }

    #[test]
    fn square_rewrite_retains_erased_denominator_and_rejects_forgery() {
        let x = Expr::from(Symbol::new("x").unwrap());
        let erased = bound(0.into()).divide(bound(x.clone()), 16).unwrap();
        let sum = DomainExpression::add([erased, bound(x.clone())], 16).unwrap();
        let source = sum.pow_integer(2.into(), 16).unwrap();
        let mut result = claim(&source);
        let symbolic = result.symbolic_value().unwrap();
        assert_eq!(symbolic.expression(), &Expr::abs(x.clone()));
        assert_eq!(symbolic.conditions(), &[Condition::NonZero(x.clone())]);
        assert_eq!(
            result.preconditions.status(),
            DomainPreconditionStatus::Blocked
        );
        assert!(verify(&source, &result));
        result.value = PrincipalSquareRootValue::Symbolic(bound(Expr::abs(x.clone())));
        assert!(!verify(&source, &result));
        result.value = PrincipalSquareRootValue::Symbolic(bound(x));
        assert!(!verify(&source, &result));
    }

    #[test]
    fn root_admission_refuses_unbound_external_and_complex_sources() {
        let assumptions = Assumptions::new();
        let raw = DomainExpression::from_expr(16.into(), 16).unwrap();
        assert_eq!(
            DomainPreconditionReport::principal_square_root(&raw, &assumptions),
            Err(DomainPreconditionError::UnboundSource)
        );
        let external =
            DomainExpression::with_conditions(0.into(), [Condition::NonZero(2.into())], 16, 16)
                .unwrap();
        let source = DomainExpression::add([external, bound(16.into())], 16).unwrap();
        assert_eq!(
            DomainPreconditionReport::principal_square_root(&source, &assumptions),
            Err(DomainPreconditionError::UnboundSource)
        );
        let complex =
            DomainExpression::from_expr_in_domain(16.into(), ScalarDomain::Complex, 16, 16)
                .unwrap();
        assert!(DomainPreconditionReport::principal_square_root(&complex, &assumptions).is_err());
    }

    #[test]
    fn log_absolute_value_requires_nonzero_not_positive_argument() {
        for (argument, status) in [
            (-2, DomainPreconditionStatus::Proven),
            (0, DomainPreconditionStatus::Refuted),
            (2, DomainPreconditionStatus::Proven),
        ] {
            let source = bound(Expr::log(Expr::abs(argument.into())));
            assert_eq!(source.conditions(), &[Condition::NonZero(argument.into())]);
            let report = DomainPreconditionReport::check_bound(
                &source,
                ScalarDomain::Real,
                &Assumptions::new(),
            )
            .unwrap();
            assert_eq!(report.status(), status);
        }
        assert!(
            DomainExpression::from_expr_in_domain(
                Expr::abs(2.into()),
                ScalarDomain::Complex,
                16,
                16,
            )
            .is_err()
        );
    }

    #[test]
    fn zero_power_and_resource_limits_remain_explicit() {
        let source = bound(Expr::pow(0.into(), 0.into()));
        assert!(claim(&source).is_undefined());
        let two = Integer::from(2);
        let mut value = two.clone();
        for _ in 0..MAX_SCALAR_BITS {
            value = &value * &two;
        }
        let source = bound(value.into());
        assert_eq!(
            DomainPreconditionReport::principal_square_root(&source, &Assumptions::new()),
            Err(DomainPreconditionError::ScalarLimit)
        );
    }
}
