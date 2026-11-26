//! Certified isolation of all distinct real roots of square-free polynomials over Q.

use std::{cmp::Ordering, error::Error, fmt};

use mathema::{DomainExpression, Integer, Rational, Symbol};

use crate::budget::WorkBudget;
use crate::{
    EvaluationContext, EvaluationError, RationalPolynomial, SturmCountCertificate, SturmCountError,
    SturmVerificationError, verify_sturm_count,
};

/// An open dyadic interval containing exactly one real root. The midpoint is
/// an approximation, with absolute error strictly less than the half-width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IsolatedRealRoot {
    lower: Rational,
    upper: Rational,
    midpoint: Rational,
    error_bound: Rational,
}

impl IsolatedRealRoot {
    pub fn lower(&self) -> &Rational {
        &self.lower
    }
    pub fn upper(&self) -> &Rational {
        &self.upper
    }
    pub fn midpoint(&self) -> &Rational {
        &self.midpoint
    }
    pub fn error_bound(&self) -> &Rational {
        &self.error_bound
    }
}

/// Complete isolation evidence, including the checked enclosing Sturm chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealRootIsolation {
    enclosing_count: SturmCountCertificate,
    roots: Vec<IsolatedRealRoot>,
}

impl RealRootIsolation {
    pub fn enclosing_count(&self) -> &SturmCountCertificate {
        &self.enclosing_count
    }
    pub fn roots(&self) -> &[IsolatedRealRoot] {
        &self.roots
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealRootIsolationError {
    RestrictedDomain,
    NonpositiveTargetWidth,
    ZeroPolynomial,
    NonSquareFree,
    Evaluation(EvaluationError),
    InvalidEvidence,
}

impl fmt::Display for RealRootIsolationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for RealRootIsolationError {}
impl From<EvaluationError> for RealRootIsolationError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}
impl From<SturmCountError> for RealRootIsolationError {
    fn from(error: SturmCountError) -> Self {
        match error {
            SturmCountError::ZeroPolynomial => Self::ZeroPolynomial,
            SturmCountError::NonSquareFree => Self::NonSquareFree,
            SturmCountError::Evaluation(error) => Self::Evaluation(error),
            _ => Self::InvalidEvidence,
        }
    }
}
impl From<SturmVerificationError> for RealRootIsolationError {
    fn from(error: SturmVerificationError) -> Self {
        match error {
            SturmVerificationError::Evaluation(error) => Self::Evaluation(error),
            _ => Self::InvalidEvidence,
        }
    }
}

fn integer(n: i32) -> Rational {
    Rational::from(Integer::from(n))
}

fn input(
    ctx: &EvaluationContext,
    work: &mut WorkBudget,
    source: &DomainExpression,
    variable: &Symbol,
    width: &Rational,
) -> Result<RationalPolynomial, RealRootIsolationError> {
    if !source.conditions().is_empty() {
        return Err(RealRootIsolationError::RestrictedDomain);
    }
    ctx.check_rational(width)?;
    if width.numerator() <= &Integer::zero() {
        return Err(RealRootIsolationError::NonpositiveTargetWidth);
    }
    ctx.check_expression(source.expression())?;
    let polynomial = work.convert(source.expression(), variable)?;
    ctx.check_polynomial(&polynomial)?;
    if polynomial.is_zero() {
        return Err(RealRootIsolationError::ZeroPolynomial);
    }
    Ok(polynomial)
}

// Cauchy's bound: every complex root has modulus <= 1 + max |a_i/a_n|.
// A strictly larger power of two gives non-root dyadic endpoints.
fn enclosing_bound(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
) -> Result<Rational, EvaluationError> {
    let (leading, coefficients) = polynomial.coefficients().split_last().expect("nonzero");
    let mut largest = integer(0);
    for coefficient in coefficients {
        let ratio = work.quotient(coefficient, leading)?;
        let magnitude = if ratio.numerator() < &Integer::zero() {
            -ratio
        } else {
            ratio
        };
        if work.subtract_scalar(&magnitude, &largest)?.numerator() > &Integer::zero() {
            largest = magnitude;
        }
    }
    let limit = work.add_scalar(&largest, &integer(1))?;
    let mut bound = integer(2);
    while work.subtract_scalar(&bound, &limit)?.numerator() <= &Integer::zero() {
        bound = work.multiply_scalar(&bound, &integer(2))?;
    }
    Ok(bound)
}

fn midpoint(
    work: &mut WorkBudget,
    a: &Rational,
    b: &Rational,
) -> Result<Rational, EvaluationError> {
    let sum = work.add_scalar(a, b)?;
    work.quotient(&sum, &integer(2))
}

// Only call with a verified Sturm chain. None means the endpoint is a root
// of the original polynomial; zeros in later chain members are skipped.
fn variations(
    work: &mut WorkBudget,
    chain: &[RationalPolynomial],
    at: &Rational,
) -> Result<Option<usize>, EvaluationError> {
    let mut previous = None;
    let mut changes = 0;
    for (index, polynomial) in chain.iter().enumerate() {
        work.charge(1)?;
        let sign = work
            .evaluate(polynomial, at)?
            .numerator()
            .cmp(&Integer::zero());
        if sign == Ordering::Equal {
            if index == 0 {
                return Ok(None);
            }
            continue;
        }
        if previous.is_some_and(|old| old != sign) {
            changes += 1;
        }
        previous = Some(sign);
    }
    Ok(Some(changes))
}

fn bracket(
    work: &mut WorkBudget,
    lower: Rational,
    upper: Rational,
) -> Result<IsolatedRealRoot, EvaluationError> {
    let center = midpoint(work, &lower, &upper)?;
    let width = work.subtract_scalar(&upper, &lower)?;
    let error_bound = work.quotient(&width, &integer(2))?;
    Ok(IsolatedRealRoot {
        lower,
        upper,
        midpoint: center,
        error_bound,
    })
}

fn dyadic(work: &mut WorkBudget, value: &Rational) -> Result<bool, EvaluationError> {
    let mut denominator = value.denominator().clone();
    let two = Integer::from(2);
    while !denominator.is_one() {
        work.charge(1)?;
        let Some(half) = denominator.divide_exact(&two) else {
            return Ok(false);
        };
        denominator = half;
    }
    Ok(true)
}

fn less(work: &mut WorkBudget, a: &Rational, b: &Rational) -> Result<bool, EvaluationError> {
    Ok(work.subtract_scalar(a, b)?.numerator() < &Integer::zero())
}

impl EvaluationContext {
    /// Isolate every real root of an unrestricted nonzero square-free Q[x]
    /// polynomial. Nonempty original-domain conditions are refused, not erased.
    /// The target width is an exact positive rational. A resource refusal
    /// returns no partial certificate. Each of the fixed conversion/subdivision
    /// and Sturm generation/verification phases uses the context's work limit.
    pub fn isolate_real_roots(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        target_width: &Rational,
    ) -> Result<RealRootIsolation, RealRootIsolationError> {
        let mut work = WorkBudget::new(self.limits());
        let polynomial = input(self, &mut work, source, variable, target_width)?;
        let bound = enclosing_bound(&mut work, &polynomial)?;
        let enclosing_count = self.count_real_roots_sturm(&polynomial, &(-&bound), &bound)?;
        let mut pending = vec![(
            -&bound,
            bound,
            enclosing_count.lower_variations(),
            enclosing_count.upper_variations(),
        )];
        let mut roots: Vec<IsolatedRealRoot> = Vec::new();
        while let Some((lower, upper, left, right)) = pending.pop() {
            work.charge(1)?;
            let count = left
                .checked_sub(right)
                .ok_or(RealRootIsolationError::InvalidEvidence)?;
            if count == 0 {
                continue;
            }
            let width = work.subtract_scalar(&upper, &lower)?;
            if count == 1
                && !less(&mut work, target_width, &width)?
                && match roots.last() {
                    Some(previous) => less(&mut work, &previous.upper, &lower)?,
                    None => true,
                }
            {
                roots.push(bracket(&mut work, lower, upper)?);
                continue;
            }
            let mut split = midpoint(&mut work, &lower, &upper)?;
            // A root at the midpoint is never an endpoint of a Sturm interval.
            // Move to a distinct dyadic point until the split is not a root.
            let middle = loop {
                if let Some(value) = variations(&mut work, enclosing_count.sequence(), &split)? {
                    break value;
                }
                split = midpoint(&mut work, &lower, &split)?;
            };
            if middle > left || middle < right {
                return Err(RealRootIsolationError::InvalidEvidence);
            }
            if middle > right {
                pending.push((split.clone(), upper, middle, right));
            }
            if left > middle {
                pending.push((lower, split, left, middle));
            }
        }
        let result = RealRootIsolation {
            enclosing_count,
            roots,
        };
        verify_real_root_isolation(self, source, variable, target_width, &result)?;
        Ok(result)
    }
}

/// Checks completeness without replaying the subdivision algorithm. Rebuilds
/// the source polynomial and enclosing bound, verifies the Sturm chain, and
/// independently counts one root per strictly disjoint bracket. Equality of
/// their number with the enclosing count proves that no real root was omitted.
pub fn verify_real_root_isolation(
    ctx: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    target_width: &Rational,
    claimed: &RealRootIsolation,
) -> Result<(), RealRootIsolationError> {
    let mut work = WorkBudget::new(ctx.limits());
    let polynomial = input(ctx, &mut work, source, variable, target_width)?;
    let bound = enclosing_bound(&mut work, &polynomial)?;
    let lower_bound = -&bound;
    verify_sturm_count(
        ctx,
        &polynomial,
        &lower_bound,
        &bound,
        &claimed.enclosing_count,
    )?;
    if claimed.roots.len() != claimed.enclosing_count.root_count() {
        return Err(RealRootIsolationError::InvalidEvidence);
    }
    let mut previous = None;
    for root in &claimed.roots {
        work.charge(1)?;
        for value in [&root.lower, &root.upper, &root.midpoint, &root.error_bound] {
            ctx.check_rational(value)?;
            if !dyadic(&mut work, value)? {
                return Err(RealRootIsolationError::InvalidEvidence);
            }
        }
        if less(&mut work, &root.lower, &lower_bound)?
            || less(&mut work, &bound, &root.upper)?
            || !less(&mut work, &root.lower, &root.upper)?
            || match previous {
                Some(endpoint) => !less(&mut work, endpoint, &root.lower)?,
                None => false,
            }
        {
            return Err(RealRootIsolationError::InvalidEvidence);
        }
        let width = work.subtract_scalar(&root.upper, &root.lower)?;
        if less(&mut work, target_width, &width)?
            || bracket(&mut work, root.lower.clone(), root.upper.clone())? != *root
        {
            return Err(RealRootIsolationError::InvalidEvidence);
        }
        let chain = claimed.enclosing_count.sequence();
        let left = variations(&mut work, chain, &root.lower)?;
        let right = variations(&mut work, chain, &root.upper)?;
        if !matches!((left, right), (Some(a), Some(b)) if a.checked_sub(b) == Some(1)) {
            return Err(RealRootIsolationError::InvalidEvidence);
        }
        previous = Some(&root.upper);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResourceLimitKind, ResourceLimits};
    use mathema::{Condition, Expr};

    fn x() -> Symbol {
        Symbol::new("x").unwrap()
    }
    fn variable() -> Expr {
        Expr::from(x())
    }
    fn source(expr: Expr) -> DomainExpression {
        DomainExpression::from_expr(expr, 128).unwrap()
    }
    fn rational(n: i32, d: i32) -> Rational {
        Rational::new(Integer::from(n), Integer::from(d)).unwrap()
    }
    fn quadratic() -> DomainExpression {
        source(Expr::add([
            Expr::pow(variable(), Expr::from(2)),
            Expr::from(-2),
        ]))
    }
    fn isolate(source: &DomainExpression, width: &Rational) -> RealRootIsolation {
        EvaluationContext::default()
            .isolate_real_roots(source, &x(), width)
            .unwrap()
    }

    #[test]
    fn irrational_roots_have_exact_dyadic_error_bounds() {
        let input = quadratic();
        let width = rational(1, 1024);
        let result = isolate(&input, &width);
        assert_eq!(result.roots().len(), 2);
        assert_eq!(result, isolate(&input, &width));
        let p = result.enclosing_count().polynomial();
        for root in result.roots() {
            let ctx = EvaluationContext::default();
            let a = ctx.evaluate_polynomial(p, root.lower()).unwrap();
            let b = ctx.evaluate_polynomial(p, root.upper()).unwrap();
            assert!(a.numerator() * b.numerator() < Integer::zero());
            assert!((root.upper() - root.lower()) <= width);
            assert_eq!(
                root.error_bound() + root.error_bound(),
                root.upper() - root.lower()
            );
        }
        assert!(result.roots()[0].upper() < result.roots()[1].lower());
    }

    #[test]
    fn closely_spaced_roots_and_dyadic_split_roots_are_not_lost() {
        let epsilon = rational(1, 65536);
        let input = source(Expr::mul([
            variable(),
            Expr::add([variable(), Expr::from(-&epsilon)]),
        ]));
        let result = isolate(&input, &integer(1));
        assert_eq!(result.roots().len(), 2);
        let a = &result.roots()[0];
        let b = &result.roots()[1];
        assert!(a.lower() < &integer(0) && a.upper() > &integer(0));
        assert!(b.lower() < &epsilon && b.upper() > &epsilon);
        assert!(a.upper() < b.lower());
    }

    #[test]
    fn even_number_of_roots_and_multiple_sign_changes_are_counted() {
        let input = source(Expr::mul(
            [-3, -1, 1, 3].map(|n| Expr::add([variable(), Expr::from(-n)])),
        ));
        let result = isolate(&input, &rational(1, 8));
        assert_eq!(result.roots().len(), 4);
        for (root, n) in result.roots().iter().zip([-3, -1, 1, 3]) {
            assert!(root.lower() < &integer(n) && root.upper() > &integer(n));
        }
    }

    #[test]
    fn constants_and_positive_quadratics_have_no_real_roots() {
        for input in [
            source(Expr::from(7)),
            source(Expr::add([
                Expr::pow(variable(), Expr::from(2)),
                Expr::from(1),
            ])),
        ] {
            assert!(isolate(&input, &integer(1)).roots().is_empty());
        }
    }

    #[test]
    fn repeated_roots_zero_polynomial_and_invalid_width_are_refused() {
        let ctx = EvaluationContext::default();
        let repeated = source(Expr::pow(variable(), Expr::from(2)));
        assert_eq!(
            ctx.isolate_real_roots(&repeated, &x(), &integer(1)),
            Err(RealRootIsolationError::NonSquareFree)
        );
        assert_eq!(
            ctx.isolate_real_roots(&source(Expr::from(0)), &x(), &integer(1)),
            Err(RealRootIsolationError::ZeroPolynomial)
        );
        for width in [integer(0), integer(-1)] {
            assert_eq!(
                ctx.isolate_real_roots(&quadratic(), &x(), &width),
                Err(RealRootIsolationError::NonpositiveTargetWidth)
            );
        }
    }

    #[test]
    fn inherited_exclusions_and_nonpolynomial_sources_are_refused() {
        let ctx = EvaluationContext::default();
        let restricted =
            DomainExpression::with_conditions(variable(), [Condition::NonZero(variable())], 128, 8)
                .unwrap();
        assert_eq!(
            ctx.isolate_real_roots(&restricted, &x(), &integer(1)),
            Err(RealRootIsolationError::RestrictedDomain)
        );
        let other = source(Expr::from(Symbol::new("y").unwrap()));
        assert!(matches!(
            ctx.isolate_real_roots(&other, &x(), &integer(1)),
            Err(RealRootIsolationError::Evaluation(
                EvaluationError::Polynomial(_)
            ))
        ));
    }

    #[test]
    fn exhausted_work_and_scalar_limits_never_produce_a_certificate() {
        let ctx = EvaluationContext::new(ResourceLimits {
            max_work_units: 1,
            ..ResourceLimits::default()
        });
        assert_eq!(
            ctx.isolate_real_roots(&quadratic(), &x(), &integer(1)),
            Err(RealRootIsolationError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::WorkUnits
            )))
        );
        let ctx = EvaluationContext::new(ResourceLimits {
            max_intermediate_scalar_bits: 4,
            ..ResourceLimits::default()
        });
        assert!(matches!(
            ctx.isolate_real_roots(&quadratic(), &x(), &rational(1, 1024)),
            Err(RealRootIsolationError::Evaluation(EvaluationError::Limit(
                _
            )))
        ));
    }

    #[test]
    fn verifier_rejects_forged_coverage_order_width_and_approximation() {
        let ctx = EvaluationContext::default();
        let input = quadratic();
        let width = rational(1, 16);
        let result = isolate(&input, &width);
        let verify = |candidate: &RealRootIsolation| {
            verify_real_root_isolation(&ctx, &input, &x(), &width, candidate)
        };
        assert_eq!(verify(&result), Ok(()));
        let mut forged = result.clone();
        forged.roots.pop();
        assert_eq!(
            verify(&forged),
            Err(RealRootIsolationError::InvalidEvidence)
        );
        let mut forged = result.clone();
        forged.roots[1] = forged.roots[0].clone();
        assert_eq!(
            verify(&forged),
            Err(RealRootIsolationError::InvalidEvidence)
        );
        let mut forged = result.clone();
        forged.roots.reverse();
        assert_eq!(
            verify(&forged),
            Err(RealRootIsolationError::InvalidEvidence)
        );
        let mut forged = result.clone();
        forged.roots[0].midpoint = integer(0);
        assert_eq!(
            verify(&forged),
            Err(RealRootIsolationError::InvalidEvidence)
        );
        let mut forged = result.clone();
        forged.roots[0].error_bound = integer(0);
        assert_eq!(
            verify(&forged),
            Err(RealRootIsolationError::InvalidEvidence)
        );
        let mut forged = result.clone();
        forged.roots[0] =
            bracket(&mut WorkBudget::new(ctx.limits()), integer(-2), integer(-1)).unwrap();
        assert_eq!(
            verify(&forged),
            Err(RealRootIsolationError::InvalidEvidence)
        );
        let mut forged = result.clone();
        forged.roots[0].lower = rational(-3, 2);
        forged.roots[0].upper = rational(-4, 3);
        assert_eq!(
            verify(&forged),
            Err(RealRootIsolationError::InvalidEvidence)
        );
        assert!(
            verify_real_root_isolation(&ctx, &source(variable()), &x(), &width, &result).is_err()
        );
        assert!(
            verify_real_root_isolation(&ctx, &input, &x(), &rational(1, 1024), &result).is_err()
        );
    }

    #[test]
    fn verifier_rejects_a_root_as_an_interval_endpoint() {
        let ctx = EvaluationContext::default();
        let input = source(variable());
        let width = integer(1);
        let mut result = isolate(&input, &width);
        result.roots[0] = bracket(
            &mut WorkBudget::new(ctx.limits()),
            integer(0),
            rational(1, 2),
        )
        .unwrap();
        assert_eq!(
            verify_real_root_isolation(&ctx, &input, &x(), &width, &result),
            Err(RealRootIsolationError::InvalidEvidence)
        );
    }
}
