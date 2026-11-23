//! Certified sign charts for degree-at-most-two real rational inequalities.
//!
//! The indeterminate is real. Exact roots are supplied by the checked algebraic
//! root solver; signs between them follow from root multiplicities and the
//! leading coefficients. Inherited denominator restrictions remain in force.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::budget::WorkBudget;
use crate::{
    AlgebraicRootSet, EvaluationContext, EvaluationError, ExactQuadraticRoot, QuadraticBranch,
    RationalFunction, RationalPolynomial,
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

    pub fn intervals(&self) -> &[RealInterval] {
        &self.intervals
    }

    pub fn excluded_zero_sets(&self) -> &[RationalPolynomial] {
        self.function.excluded_zero_sets()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InequalityError {
    /// Completeness is not claimed above degree two for any input factor or
    /// inherited domain restriction.
    UnsupportedDegree,
    Evaluation(EvaluationError),
    IncorrectCertificate,
}

impl fmt::Display for InequalityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDegree => {
                f.write_str("real inequality factor or domain restriction exceeds degree two")
            }
            Self::Evaluation(error) => write!(f, "{error}"),
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

fn collect_roots(
    context: &EvaluationContext,
    polynomial: &RationalPolynomial,
    source: usize,
    points: &mut Vec<InequalityCriticalPoint>,
    work: &mut WorkBudget,
) -> Result<(), InequalityError> {
    if polynomial.degree().is_some_and(|degree| degree > 2) {
        return Err(InequalityError::UnsupportedDegree);
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
    if let Some(previous) = intervals.last_mut() {
        if previous.upper == next.lower && (previous.upper_closed || next.lower_closed) {
            previous.upper = next.upper;
            previous.upper_closed = next.upper_closed;
            return;
        }
    }
    intervals.push(next);
}

fn build(
    context: &EvaluationContext,
    function: &RationalFunction,
    comparison: ComparisonOperator,
) -> Result<InequalityResult, InequalityError> {
    context.check_rational_function(function)?;
    let mut work = WorkBudget::new(context.limits());
    work.charge(1)?;
    let mut points = Vec::new();
    collect_roots(context, function.numerator(), 0, &mut points, &mut work)?;
    collect_roots(context, function.denominator(), 1, &mut points, &mut work)?;
    for condition in function.excluded_zero_sets() {
        collect_roots(context, condition, 2, &mut points, &mut work)?;
    }
    let mut intervals = Vec::new();
    let mut lower = RealBound::NegativeInfinity;
    let mut current_sign = negative_infinity_sign(function.numerator())
        * negative_infinity_sign(function.denominator());

    for point in &points {
        work.charge(1)?;
        let bound = RealBound::Finite(point.root.clone());
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
        intervals,
    })
}

/// Independently regenerates certified degree-two boundaries from the input
/// and checks the complete sign partition, including inherited domain holes.
/// The sign rule is exact: off the roots a real polynomial has the sign of
/// its leading term at -infinity, toggled at each odd-multiplicity real root.
pub fn verify_inequality_result(
    context: &EvaluationContext,
    proposed: &InequalityResult,
) -> Result<bool, InequalityError> {
    let certified = build(context, &proposed.function, proposed.comparison)?;
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
    let result = build(context, function, comparison)?;
    if !verify_inequality_result(context, &result)? {
        return Err(InequalityError::IncorrectCertificate);
    }
    Ok(result)
}

impl EvaluationContext {
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
}
