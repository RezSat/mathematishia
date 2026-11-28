//! Certified sign charts for bounded real rational inequalities.
//!
//! The indeterminate is real. Exact roots are supplied by the checked algebraic
//! root solver; signs between them follow from root multiplicities and the
//! leading coefficients. Inherited denominator restrictions remain in force.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{DomainExpression, Integer, Rational, Symbol};

use crate::budget::WorkBudget;
use crate::{
    AlgebraicRootSet, EvaluationContext, EvaluationError, ExactQuadraticRoot, QuadraticBranch,
    RationalExpressionError, RationalFactorizationError, RationalFunction, RationalPolynomial,
    ResourceLimitKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComparisonOperator {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl ComparisonOperator {
    fn accepts(self, sign: i8) -> bool {
        match self {
            Self::Less => sign < 0,
            Self::LessOrEqual => sign <= 0,
            Self::Greater => sign > 0,
            Self::GreaterOrEqual => sign >= 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealBound {
    NegativeInfinity,
    Finite(ExactQuadraticRoot),
    PositiveInfinity,
}

/// An interval in the canonical, ordered real solution union. Equal, closed
/// endpoints represent a singleton; infinite endpoints are always open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealInterval {
    pub lower: RealBound,
    pub upper: RealBound,
    pub lower_closed: bool,
    pub upper_closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InequalityCriticalPoint {
    root: ExactQuadraticRoot,
    numerator_multiplicity: u8,
    denominator_multiplicity: u8,
    excluded: bool,
}

impl InequalityCriticalPoint {
    pub fn root(&self) -> &ExactQuadraticRoot {
        &self.root
    }

    pub const fn numerator_multiplicity(&self) -> u8 {
        self.numerator_multiplicity
    }

    pub const fn denominator_multiplicity(&self) -> u8 {
        self.denominator_multiplicity
    }

    pub const fn is_excluded(&self) -> bool {
        self.excluded
    }
}

/// A complete answer over the real domain, including all holes carried by
/// the original rational function before cancellation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InequalityResult {
    function: RationalFunction,
    comparison: ComparisonOperator,
    critical_points: Vec<InequalityCriticalPoint>,
    segment_signs: Vec<i8>,
    intervals: Vec<RealInterval>,
}

impl InequalityResult {
    pub fn function(&self) -> &RationalFunction {
        &self.function
    }

    pub const fn comparison(&self) -> ComparisonOperator {
        self.comparison
    }

    pub fn critical_points(&self) -> &[InequalityCriticalPoint] {
        &self.critical_points
    }

    /// Signs of the open cells from left to right, including the two tails.
    /// There is one more sign than critical points; signs are -1, 0 or 1.
    pub fn segment_signs(&self) -> &[i8] {
        &self.segment_signs
    }

    pub fn intervals(&self) -> &[RealInterval] {
        &self.intervals
    }

    pub fn excluded_zero_sets(&self) -> &[RationalPolynomial] {
        self.function.excluded_zero_sets()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InequalityError {
    /// The selected method cannot certify every required real endpoint.
    UnsupportedDegree,
    Evaluation(EvaluationError),
    Conversion(RationalExpressionError),
    IncorrectCertificate,
}

impl fmt::Display for InequalityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDegree => {
                f.write_str("real inequality has unsupported degree or unresolved roots")
            }
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Conversion(error) => write!(f, "{error}"),
            Self::IncorrectCertificate => f.write_str("inequality sign chart failed verification"),
        }
    }
}

impl Error for InequalityError {}

impl From<EvaluationError> for InequalityError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

fn zero() -> Rational {
    Rational::from(Integer::zero())
}

fn sign(value: &Rational) -> i8 {
    match value.cmp(&zero()) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// Compare a certified irrational root with a rational exactly. The sign of
/// its monic quadratic on the appropriate side of the vertex determines order.
fn compare_with_rational(
    root: &ExactQuadraticRoot,
    point: &Rational,
    work: &mut WorkBudget,
) -> Result<Ordering, InequalityError> {
    let ExactQuadraticRoot::Algebraic {
        minimal_polynomial,
        branch,
    } = root
    else {
        let ExactQuadraticRoot::Rational(value) = root else {
            unreachable!()
        };
        return Ok(value.cmp(point));
    };
    let vertex = work.quotient(
        &(-&minimal_polynomial.coefficients()[1]),
        &Rational::from(Integer::from(2)),
    )?;
    let position = point.cmp(&vertex);
    let at_point = work.evaluate(minimal_polynomial, point)?;
    Ok(match branch {
        QuadraticBranch::LowerReal => {
            if position != Ordering::Less || sign(&at_point) < 0 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        QuadraticBranch::UpperReal => {
            if position != Ordering::Greater || sign(&at_point) < 0 {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        QuadraticBranch::NegativeImaginary | QuadraticBranch::PositiveImaginary => {
            return Err(InequalityError::IncorrectCertificate);
        }
    })
}

/// Evaluate the sign of a second monic quadratic at an algebraic root without
/// radicals: subtraction of minimal polynomials leaves an affine expression.
fn quadratic_sign_at_root(
    polynomial: &RationalPolynomial,
    root: &ExactQuadraticRoot,
    work: &mut WorkBudget,
) -> Result<i8, InequalityError> {
    if let ExactQuadraticRoot::Rational(value) = root {
        return Ok(sign(&work.evaluate(polynomial, value)?));
    }
    let ExactQuadraticRoot::Algebraic {
        minimal_polynomial, ..
    } = root
    else {
        unreachable!()
    };
    let slope = work.subtract_scalar(
        &polynomial.coefficients()[1],
        &minimal_polynomial.coefficients()[1],
    )?;
    let constant = work.subtract_scalar(
        &polynomial.coefficients()[0],
        &minimal_polynomial.coefficients()[0],
    )?;
    if sign(&slope) == 0 {
        return Ok(sign(&constant));
    }
    let threshold = work.quotient(&(-&constant), &slope)?;
    let order = compare_with_rational(root, &threshold, work)?;
    let direction = match order {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    };
    Ok(sign(&slope) * direction)
}

/// Compare two exact real roots of unrelated quadratics. A branch's position
/// relative to its vertex and the other polynomial's exact sign suffice.
fn compare_roots(
    left: &ExactQuadraticRoot,
    right: &ExactQuadraticRoot,
    work: &mut WorkBudget,
) -> Result<Ordering, InequalityError> {
    work.charge(1)?;
    if left == right {
        return Ok(Ordering::Equal);
    }
    if let ExactQuadraticRoot::Rational(value) = right {
        return compare_with_rational(left, value, work);
    }
    if let ExactQuadraticRoot::Rational(value) = left {
        return Ok(compare_with_rational(right, value, work)?.reverse());
    }
    let ExactQuadraticRoot::Algebraic {
        minimal_polynomial,
        branch,
    } = right
    else {
        unreachable!()
    };
    let vertex = work.quotient(
        &(-&minimal_polynomial.coefficients()[1]),
        &Rational::from(Integer::from(2)),
    )?;
    let side = compare_with_rational(left, &vertex, work)?;
    match branch {
        QuadraticBranch::LowerReal if side != Ordering::Less => {
            return Ok(Ordering::Greater);
        }
        QuadraticBranch::UpperReal if side != Ordering::Greater => {
            return Ok(Ordering::Less);
        }
        QuadraticBranch::LowerReal | QuadraticBranch::UpperReal => {}
        _ => return Err(InequalityError::IncorrectCertificate),
    }
    let value_sign = quadratic_sign_at_root(minimal_polynomial, left, work)?;
    Ok(match (branch, value_sign) {
        (_, 0) => Ordering::Equal,
        (QuadraticBranch::LowerReal, 1) | (QuadraticBranch::UpperReal, -1) => Ordering::Less,
        _ => Ordering::Greater,
    })
}

fn register_root(
    points: &mut Vec<InequalityCriticalPoint>,
    root: ExactQuadraticRoot,
    multiplicity: u8,
    source: usize,
    work: &mut WorkBudget,
) -> Result<(), InequalityError> {
    let mut position = 0;
    while position < points.len() {
        match compare_roots(&points[position].root, &root, work)? {
            Ordering::Less => position += 1,
            Ordering::Equal => {
                let point = &mut points[position];
                match source {
                    0 => point.numerator_multiplicity = multiplicity,
                    1 => {
                        point.denominator_multiplicity = multiplicity;
                        point.excluded = true;
                    }
                    _ => point.excluded = true,
                }
                return Ok(());
            }
            Ordering::Greater => break,
        }
    }
    work.charge(1)?;
    let mut point = InequalityCriticalPoint {
        root,
        numerator_multiplicity: 0,
        denominator_multiplicity: 0,
        excluded: source != 0,
    };
    match source {
        0 => point.numerator_multiplicity = multiplicity,
        1 => point.denominator_multiplicity = multiplicity,
        _ => {}
    }
    points.insert(position, point);
    Ok(())
}

// Bound the factorization scope even when the caller raises the context's
// polynomial-degree limit. Multiplicities fit the existing u8 interface.
const MAX_FACTORED_DEGREE: usize = 32;

fn factorization_error(error: RationalFactorizationError) -> InequalityError {
    match error {
        RationalFactorizationError::Evaluation(error)
        | RationalFactorizationError::Reconstruction(crate::HenselError::Evaluation(error)) => {
            InequalityError::Evaluation(error)
        }
        RationalFactorizationError::Reconstruction(_) => InequalityError::UnsupportedDegree,
        RationalFactorizationError::ZeroPolynomial
        | RationalFactorizationError::InvalidCertificate(_) => InequalityError::IncorrectCertificate,
    }
}

fn collect_factored_roots(
    context: &EvaluationContext,
    polynomial: &RationalPolynomial,
    source: usize,
    points: &mut Vec<InequalityCriticalPoint>,
    work: &mut WorkBudget,
) -> Result<(), InequalityError> {
    if polynomial
        .degree()
        .is_some_and(|degree| degree > MAX_FACTORED_DEGREE)
    {
        return Err(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree).into());
    }
    work.charge(1)?;
    // The native factorizer checks the exact product, scalar unit, factor
    // multiplicities and irreducibility. A certified but unrepresentable
    // higher-degree factor is still not a complete root classification.
    let factorization = context
        .factor_rational_polynomial(polynomial)
        .map_err(factorization_error)?;
    if !factorization.is_complete()
        || factorization
            .factors()
            .iter()
            .any(|factor| factor.polynomial().degree().is_none_or(|d| d > 2))
    {
        return Err(InequalityError::UnsupportedDegree);
    }
    for factor in factorization.factors() {
        work.charge(1)?;
        let multiplicity = u8::try_from(factor.multiplicity())
            .map_err(|_| InequalityError::IncorrectCertificate)?;
        let proof = context.algebraic_roots(factor.polynomial())?;
        let AlgebraicRootSet::Finite(roots) = proof.roots() else {
            return Err(InequalityError::IncorrectCertificate);
        };
        for root in roots {
            if !matches!(
                root.branch(),
                Some(QuadraticBranch::NegativeImaginary | QuadraticBranch::PositiveImaginary)
            ) {
                register_root(points, root.clone(), multiplicity, source, work)?;
            }
        }
    }
    Ok(())
}

fn collect_roots(
    context: &EvaluationContext,
    polynomial: &RationalPolynomial,
    source: usize,
    points: &mut Vec<InequalityCriticalPoint>,
    work: &mut WorkBudget,
    factored: bool,
) -> Result<(), InequalityError> {
    if polynomial.degree().is_some_and(|degree| degree > 2) {
        return if factored {
            collect_factored_roots(context, polynomial, source, points, work)
        } else {
            Err(InequalityError::UnsupportedDegree)
        };
    }
    work.charge(1)?;
    let proof = context.algebraic_roots(polynomial)?;
    let AlgebraicRootSet::Finite(roots) = proof.roots() else {
        // Only a zero numerator is allowed; all-real zeros cause no isolated
        // critical points, and its sign is handled separately.
        return if source == 0 {
            Ok(())
        } else {
            Err(InequalityError::IncorrectCertificate)
        };
    };
    let real_roots: Vec<_> = roots
        .iter()
        .filter(|root| {
            !matches!(
                root.branch(),
                Some(QuadraticBranch::NegativeImaginary | QuadraticBranch::PositiveImaginary)
            )
        })
        .cloned()
        .collect();
    let multiplicity = if polynomial.degree() == Some(2) && real_roots.len() == 1 {
        2
    } else {
        1
    };
    for root in real_roots {
        register_root(points, root, multiplicity, source, work)?;
    }
    Ok(())
}

fn negative_infinity_sign(polynomial: &RationalPolynomial) -> i8 {
    let Some(degree) = polynomial.degree() else {
        return 0;
    };
    let leading = polynomial
        .coefficients()
        .last()
        .expect("nonzero polynomial");
    sign(leading) * if degree % 2 == 0 { 1 } else { -1 }
}

fn append_interval(intervals: &mut Vec<RealInterval>, next: RealInterval) {
    if let Some(previous) = intervals.last_mut()
        && previous.upper == next.lower
        && (previous.upper_closed || next.lower_closed)
    {
        previous.upper = next.upper;
        previous.upper_closed = next.upper_closed;
        return;
    }
    intervals.push(next);
}

fn build(
    context: &EvaluationContext,
    function: &RationalFunction,
    comparison: ComparisonOperator,
    factored: bool,
) -> Result<InequalityResult, InequalityError> {
    context.check_rational_function(function)?;
    let mut work = WorkBudget::new(context.limits());
    work.charge(1)?;
    let mut points = Vec::new();
    collect_roots(
        context,
        function.numerator(),
        0,
        &mut points,
        &mut work,
        factored,
    )?;
    collect_roots(
        context,
        function.denominator(),
        1,
        &mut points,
        &mut work,
        factored,
    )?;
    for condition in function.excluded_zero_sets() {
        collect_roots(context, condition, 2, &mut points, &mut work, factored)?;
    }
    let mut intervals = Vec::new();
    let mut segment_signs = Vec::new();
    let mut lower = RealBound::NegativeInfinity;
    let mut current_sign = negative_infinity_sign(function.numerator())
        * negative_infinity_sign(function.denominator());

    for point in &points {
        work.charge(1)?;
        let bound = RealBound::Finite(point.root.clone());
        segment_signs.push(current_sign);
        if comparison.accepts(current_sign) {
            append_interval(
                &mut intervals,
                RealInterval {
                    lower: lower.clone(),
                    upper: bound.clone(),
                    lower_closed: false,
                    upper_closed: false,
                },
            );
        }
        if !point.excluded {
            let point_sign = if point.numerator_multiplicity > 0 {
                0
            } else {
                current_sign
            };
            if comparison.accepts(point_sign) {
                append_interval(
                    &mut intervals,
                    RealInterval {
                        lower: bound.clone(),
                        upper: bound.clone(),
                        lower_closed: true,
                        upper_closed: true,
                    },
                );
            }
        }
        if (point.numerator_multiplicity + point.denominator_multiplicity) % 2 == 1 {
            current_sign = -current_sign;
        }
        lower = bound;
    }
    segment_signs.push(current_sign);
    if comparison.accepts(current_sign) {
        append_interval(
            &mut intervals,
            RealInterval {
                lower,
                upper: RealBound::PositiveInfinity,
                lower_closed: false,
                upper_closed: false,
            },
        );
    }
    Ok(InequalityResult {
        function: function.clone(),
        comparison,
        critical_points: points,
        segment_signs,
        intervals,
    })
}

/// Independently regenerates certified rational and quadratic boundaries from the input
/// and checks the complete sign partition, including inherited domain holes.
/// The sign rule is exact: off the roots a real polynomial has the sign of
/// its leading term at -infinity, toggled at each odd-multiplicity real root.
pub fn verify_inequality_result(
    context: &EvaluationContext,
    proposed: &InequalityResult,
) -> Result<bool, InequalityError> {
    let certified = build(context, &proposed.function, proposed.comparison, true)?;
    if certified.critical_points != proposed.critical_points {
        return Ok(false);
    }

    let numerator = proposed.function.numerator();
    let denominator = proposed.function.denominator();
    let leading_sign =
        |polynomial: &RationalPolynomial| polynomial.coefficients().last().map_or(0, sign);
    let mut right_sign = leading_sign(numerator) * leading_sign(denominator);
    let mut segment_signs = vec![right_sign];
    for point in certified.critical_points.iter().rev() {
        if (point.numerator_multiplicity + point.denominator_multiplicity) % 2 == 1 {
            right_sign = -right_sign;
        }
        segment_signs.push(right_sign);
    }
    segment_signs.reverse();
    if segment_signs != proposed.segment_signs {
        return Ok(false);
    }

    let mut checked_intervals = Vec::new();
    let points = &certified.critical_points;
    for (index, &segment_sign) in segment_signs.iter().enumerate() {
        let lower = if index == 0 {
            RealBound::NegativeInfinity
        } else {
            RealBound::Finite(points[index - 1].root.clone())
        };
        let upper = if index == points.len() {
            RealBound::PositiveInfinity
        } else {
            RealBound::Finite(points[index].root.clone())
        };
        if proposed.comparison.accepts(segment_sign) {
            append_interval(
                &mut checked_intervals,
                RealInterval {
                    lower,
                    upper,
                    lower_closed: false,
                    upper_closed: false,
                },
            );
        }
        if let Some(point) = points.get(index) {
            let point_sign = if point.numerator_multiplicity > 0 {
                0
            } else {
                segment_sign
            };
            if !point.excluded && proposed.comparison.accepts(point_sign) {
                let bound = RealBound::Finite(point.root.clone());
                append_interval(
                    &mut checked_intervals,
                    RealInterval {
                        lower: bound.clone(),
                        upper: bound,
                        lower_closed: true,
                        upper_closed: true,
                    },
                );
            }
        }
    }
    Ok(checked_intervals == proposed.intervals)
}

/// Solve a checked univariate rational function compared with zero over R.
/// The function must already preserve restrictions from both original sides.
/// Any numerator, denominator or inherited restriction above degree two is
/// explicitly unsupported; no completeness claim extends to higher degrees.
pub fn solve_rational_inequality(
    context: &EvaluationContext,
    function: &RationalFunction,
    comparison: ComparisonOperator,
) -> Result<InequalityResult, InequalityError> {
    solve_with_factorization(context, function, comparison, false)
}

fn solve_with_factorization(
    context: &EvaluationContext,
    function: &RationalFunction,
    comparison: ComparisonOperator,
    factored: bool,
) -> Result<InequalityResult, InequalityError> {
    let result = build(context, function, comparison, factored)?;
    if !verify_inequality_result(context, &result)? {
        return Err(InequalityError::IncorrectCertificate);
    }
    Ok(result)
}

impl EvaluationContext {
    fn comparison_residual(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
    ) -> Result<RationalFunction, InequalityError> {
        let left = self
            .rational_expression(left, variable)
            .map_err(InequalityError::Conversion)?;
        let right = self
            .rational_expression(right, variable)
            .map_err(InequalityError::Conversion)?;
        Ok(self.subtract_rational_functions(&left, &right)?)
    }

    /// Convert both sides before subtracting so cancellation cannot erase either
    /// operand's original exclusions.
    pub fn solve_expression_inequality(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        comparison: ComparisonOperator,
    ) -> Result<InequalityResult, InequalityError> {
        let residual = self.comparison_residual(left, right, variable)?;
        self.solve_rational_inequality(&residual, comparison)
    }

    /// Check a certificate against the original operands and comparison.
    pub fn verify_expression_inequality(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        comparison: ComparisonOperator,
        result: &InequalityResult,
    ) -> Result<bool, InequalityError> {
        let residual = self.comparison_residual(left, right, variable)?;
        if result.function() != &residual || result.comparison() != comparison {
            return Ok(false);
        }
        verify_inequality_result(self, result)
    }

    /// Opt-in higher-degree sign chart. Every nonconstant factor must have
    /// certified rational or quadratic roots; unresolved factors are refused.
    /// The existing degree-two entry point remains unchanged.
    pub fn solve_factored_rational_inequality(
        &self,
        function: &RationalFunction,
        comparison: ComparisonOperator,
    ) -> Result<InequalityResult, InequalityError> {
        solve_with_factorization(self, function, comparison, true)
    }

    /// Preserve the domain of both original operands before factoring their
    /// comparison residual, including restrictions lost through cancellation.
    pub fn solve_factored_expression_inequality(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        comparison: ComparisonOperator,
    ) -> Result<InequalityResult, InequalityError> {
        let residual = self.comparison_residual(left, right, variable)?;
        self.solve_factored_rational_inequality(&residual, comparison)
    }

    pub fn solve_rational_inequality(
        &self,
        function: &RationalFunction,
        comparison: ComparisonOperator,
    ) -> Result<InequalityResult, InequalityError> {
        solve_rational_inequality(self, function, comparison)
    }
}

#[cfg(test)]
mod certificate_tests {
    use super::*;

    fn polynomial(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            values
                .iter()
                .map(|&value| Rational::from(Integer::from(value)))
                .collect(),
        )
    }

    #[test]
    fn forged_interval_and_missing_exclusion_fail_verification() {
        let ctx = EvaluationContext::default();
        let f = ctx
            .rational_function(&polynomial(&[-1, 1]), &polynomial(&[2, 1]))
            .unwrap();
        let result = ctx
            .solve_rational_inequality(&f, ComparisonOperator::LessOrEqual)
            .unwrap();
        let mut changed = result.clone();
        changed.intervals[0].lower_closed = true;
        assert_eq!(verify_inequality_result(&ctx, &changed), Ok(false));
        let mut changed = result.clone();
        changed.critical_points[0].excluded = false;
        assert_eq!(verify_inequality_result(&ctx, &changed), Ok(false));
    }

    fn finite(value: i64) -> RealBound {
        RealBound::Finite(ExactQuadraticRoot::Rational(Rational::from(Integer::from(
            value,
        ))))
    }

    fn interval(lower: RealBound, upper: RealBound, left: bool, right: bool) -> RealInterval {
        RealInterval {
            lower,
            upper,
            lower_closed: left,
            upper_closed: right,
        }
    }

    #[test]
    fn cubic_signs_and_endpoint_closure_are_exact() {
        let ctx = EvaluationContext::default();
        let f = RationalFunction::from_polynomial(polynomial(&[6, -5, -2, 1]));
        let result = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::GreaterOrEqual)
            .unwrap();
        assert_eq!(result.segment_signs(), &[-1, 1, -1, 1]);
        assert_eq!(
            result.intervals(),
            &[
                interval(finite(-2), finite(1), true, true),
                interval(finite(3), RealBound::PositiveInfinity, true, false),
            ]
        );
        assert_eq!(
            ctx.solve_factored_rational_inequality(&f, ComparisonOperator::GreaterOrEqual),
            Ok(result.clone())
        );
        assert_eq!(verify_inequality_result(&ctx, &result), Ok(true));
        let strict = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::Greater)
            .unwrap();
        assert_eq!(
            strict.intervals(),
            &[
                interval(finite(-2), finite(1), false, false),
                interval(finite(3), RealBound::PositiveInfinity, false, false),
            ]
        );
        assert_eq!(
            ctx.solve_rational_inequality(&f, ComparisonOperator::Greater),
            Err(InequalityError::UnsupportedDegree)
        );
    }

    #[test]
    fn repeated_zeros_and_poles_do_not_toggle_even_multiplicity() {
        let ctx = EvaluationContext::default();
        let cubic = polynomial(&[2, -3, 0, 1]);
        let f = RationalFunction::from_polynomial(cubic.clone());
        let result = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::Less)
            .unwrap();
        assert_eq!(result.segment_signs(), &[-1, 1, 1]);
        assert_eq!(result.critical_points()[1].numerator_multiplicity(), 2);
        assert_eq!(
            result.intervals(),
            &[interval(
                RealBound::NegativeInfinity,
                finite(-2),
                false,
                false,
            )]
        );
        let nonstrict = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::LessOrEqual)
            .unwrap();
        assert_eq!(
            nonstrict.intervals(),
            &[
                interval(RealBound::NegativeInfinity, finite(-2), false, true),
                interval(finite(1), finite(1), true, true),
            ]
        );
        let reciprocal = ctx.rational_function(&polynomial(&[1]), &cubic).unwrap();
        let poles = ctx
            .solve_factored_rational_inequality(&reciprocal, ComparisonOperator::Greater)
            .unwrap();
        assert_eq!(poles.critical_points()[1].denominator_multiplicity(), 2);
        assert!(
            poles
                .critical_points()
                .iter()
                .all(|point| point.is_excluded())
        );
        assert_eq!(poles.segment_signs(), &[-1, 1, 1]);
        assert_eq!(
            poles.intervals(),
            &[
                interval(finite(-2), finite(1), false, false),
                interval(finite(1), RealBound::PositiveInfinity, false, false),
            ]
        );
    }

    #[test]
    fn mixed_rational_and_irrational_roots_and_nonreal_factors() {
        let ctx = EvaluationContext::default();
        let f = RationalFunction::from_polynomial(polynomial(&[6, -2, -3, 1]));
        let result = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::LessOrEqual)
            .unwrap();
        let roots = ctx.algebraic_roots(&polynomial(&[-2, 0, 1])).unwrap();
        let AlgebraicRootSet::Finite(values) = roots.roots() else {
            panic!("expected two real roots");
        };
        assert_eq!(result.segment_signs(), &[-1, 1, -1, 1]);
        assert_eq!(
            result.intervals(),
            &[
                interval(
                    RealBound::NegativeInfinity,
                    RealBound::Finite(values[0].clone()),
                    false,
                    true,
                ),
                interval(RealBound::Finite(values[1].clone()), finite(3), true, true),
            ]
        );
        let f = RationalFunction::from_polynomial(polynomial(&[-1, 1, -1, 1]));
        let result = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::Greater)
            .unwrap();
        assert_eq!(result.segment_signs(), &[-1, 1]);
        assert_eq!(result.critical_points().len(), 1);
        assert_eq!(
            result.intervals(),
            &[interval(
                finite(1),
                RealBound::PositiveInfinity,
                false,
                false,
            )]
        );
    }

    #[test]
    fn cancelled_cubic_and_zero_numerator_keep_all_holes() {
        let ctx = EvaluationContext::default();
        let cubic = polynomial(&[6, -5, -2, 1]);
        for numerator in [polynomial(&[0]), cubic.clone()] {
            let f = ctx.rational_function(&numerator, &cubic).unwrap();
            let result = ctx
                .solve_factored_rational_inequality(&f, ComparisonOperator::GreaterOrEqual)
                .unwrap();
            assert_eq!(result.excluded_zero_sets().len(), 1);
            assert_eq!(result.critical_points().len(), 3);
            assert!(
                result
                    .critical_points()
                    .iter()
                    .all(|point| point.is_excluded())
            );
            assert_eq!(
                result.intervals(),
                &[
                    interval(RealBound::NegativeInfinity, finite(-2), false, false),
                    interval(finite(-2), finite(1), false, false),
                    interval(finite(1), finite(3), false, false),
                    interval(finite(3), RealBound::PositiveInfinity, false, false),
                ]
            );
            assert_eq!(
                result.segment_signs(),
                if numerator.is_zero() { &[0; 4] } else { &[1; 4] }
            );
        }
        let f = ctx
            .rational_function(&polynomial(&[-1, 0, 1]), &polynomial(&[-1, 1]))
            .unwrap();
        let result = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::GreaterOrEqual)
            .unwrap();
        assert_eq!(
            result.intervals(),
            &[
                interval(finite(-1), finite(1), true, false),
                interval(finite(1), RealBound::PositiveInfinity, false, false),
            ]
        );
    }

    #[test]
    fn unrepresentable_roots_and_limits_refuse() {
        let ctx = EvaluationContext::default();
        let cubic = polynomial(&[1, 1, 0, 1]);
        let f = RationalFunction::from_polynomial(cubic.clone());
        assert_eq!(
            ctx.solve_factored_rational_inequality(&f, ComparisonOperator::Greater),
            Err(InequalityError::UnsupportedDegree)
        );
        let restricted = RationalFunction::from_polynomial(polynomial(&[0]))
            .restrict_nonzero(cubic)
            .unwrap();
        assert_eq!(
            ctx.solve_factored_rational_inequality(&restricted, ComparisonOperator::GreaterOrEqual),
            Err(InequalityError::UnsupportedDegree)
        );
        let small = EvaluationContext::new(crate::ResourceLimits {
            max_work_units: 0,
            ..crate::ResourceLimits::default()
        });
        let f = RationalFunction::from_polynomial(polynomial(&[6, -5, -2, 1]));
        assert_eq!(
            small.solve_factored_rational_inequality(&f, ComparisonOperator::Greater),
            Err(InequalityError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::WorkUnits
            )))
        );
        let larger = EvaluationContext::new(crate::ResourceLimits {
            max_polynomial_degree: 64,
            ..crate::ResourceLimits::default()
        });
        let mut coefficients = vec![0; MAX_FACTORED_DEGREE + 2];
        coefficients[MAX_FACTORED_DEGREE + 1] = 1;
        let f = RationalFunction::from_polynomial(polynomial(&coefficients));
        assert_eq!(
            larger.solve_factored_rational_inequality(&f, ComparisonOperator::Greater),
            Err(InequalityError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::PolynomialDegree
            )))
        );
    }

    #[test]
    fn forged_factored_chart_fails_verification() {
        let ctx = EvaluationContext::default();
        let f = RationalFunction::from_polynomial(polynomial(&[2, -3, 0, 1]));
        let result = ctx
            .solve_factored_rational_inequality(&f, ComparisonOperator::LessOrEqual)
            .unwrap();
        let mut missing = result.clone();
        missing.critical_points.pop();
        assert_eq!(verify_inequality_result(&ctx, &missing), Ok(false));
        let mut wrong_order = result.clone();
        wrong_order.critical_points.swap(0, 1);
        assert_eq!(verify_inequality_result(&ctx, &wrong_order), Ok(false));
        let mut wrong_multiplicity = result.clone();
        wrong_multiplicity.critical_points[1].numerator_multiplicity = 1;
        assert_eq!(
            verify_inequality_result(&ctx, &wrong_multiplicity),
            Ok(false)
        );
        let mut wrong_sign = result.clone();
        wrong_sign.segment_signs[1] = -1;
        assert_eq!(verify_inequality_result(&ctx, &wrong_sign), Ok(false));
        let mut wrong_endpoint = result;
        wrong_endpoint.intervals[0].upper_closed = false;
        assert_eq!(verify_inequality_result(&ctx, &wrong_endpoint), Ok(false));
    }
}
