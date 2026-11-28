//! Complete real solutions for bounded chains of exact affine absolute values.
//!
//! The input is an explicit total function, not a conversion from normalized
//! syntax. Callers must not discard source definedness conditions when lowering.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational, ScalarDomain, Symbol};

use crate::budget::WorkBudget;
use crate::{EvaluationContext, EvaluationError, ResourceLimitKind};

const MAX_LAYERS: usize = 8;

/// The affine map y = slope * x + constant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsoluteAffine {
    pub slope: Rational,
    pub constant: Rational,
}

/// Start with base(x), then apply each layer as slope * abs(y) + constant.
/// Layers are ordered from innermost to outermost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsoluteExpression {
    pub variable: Symbol,
    pub base: AbsoluteAffine,
    pub layers: Vec<AbsoluteAffine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsoluteEquation {
    pub domain: ScalarDomain,
    pub left: AbsoluteExpression,
    pub right: AbsoluteExpression,
}

/// A closed real interval, including its finite endpoints. None means negative
/// infinity for the lower bound and positive infinity for the upper bound.
/// Equal finite endpoints describe a singleton.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsoluteInterval {
    pub lower: Option<Rational>,
    pub upper: Option<Rational>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbsoluteSign {
    Nonnegative,
    Nonpositive,
}

/// One closed sign case. Signs correspond to the left layers, followed by the
/// right layers. Adjacent cases share their boundary, where both signs hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsoluteCase {
    pub interval: AbsoluteInterval,
    pub signs: Vec<AbsoluteSign>,
    pub solution: Option<AbsoluteInterval>,
}

/// The ordered solution union has no overlapping or touching components.
/// An empty union means no solution; nondegenerate intervals contain
/// infinitely many real solutions, not just rational points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsoluteEquationResult {
    pub cases: Vec<AbsoluteCase>,
    pub solutions: Vec<AbsoluteInterval>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbsoluteEquationError {
    UnsupportedDomain,
    MultipleVariables,
    MissingAbsoluteValue,
    CaseLimit,
    Evaluation(EvaluationError),
    VerificationFailed,
}

impl fmt::Display for AbsoluteEquationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDomain => f.write_str("absolute equations require the real domain"),
            Self::MultipleVariables => f.write_str("absolute equations require one variable"),
            Self::MissingAbsoluteValue => f.write_str("no absolute-value layer was supplied"),
            Self::CaseLimit => f.write_str("absolute equations support at most eight layers"),
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::VerificationFailed => {
                f.write_str("absolute equation evidence failed verification")
            }
        }
    }
}

impl Error for AbsoluteEquationError {}

impl From<EvaluationError> for AbsoluteEquationError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

fn sign(value: &Rational) -> Ordering {
    value.numerator().cmp(&Integer::zero())
}

fn compare(
    work: &mut WorkBudget,
    left: &Rational,
    right: &Rational,
) -> Result<Ordering, AbsoluteEquationError> {
    Ok(sign(&work.subtract_scalar(left, right)?))
}

fn check_scalar(
    context: &EvaluationContext,
    work: &mut WorkBudget,
    value: &Rational,
) -> Result<(), AbsoluteEquationError> {
    context.check_rational(value)?;
    work.multiply_scalar(value, &Rational::from(Integer::one()))?;
    Ok(())
}

fn check_equation(
    context: &EvaluationContext,
    work: &mut WorkBudget,
    equation: &AbsoluteEquation,
) -> Result<usize, AbsoluteEquationError> {
    if equation.domain != ScalarDomain::Real {
        return Err(AbsoluteEquationError::UnsupportedDomain);
    }
    if equation.left.variable != equation.right.variable {
        return Err(AbsoluteEquationError::MultipleVariables);
    }
    let left = equation.left.layers.len();
    let right = equation.right.layers.len();
    let total = left.saturating_add(right);
    if total == 0 {
        return Err(AbsoluteEquationError::MissingAbsoluteValue);
    }
    if total > MAX_LAYERS {
        return Err(AbsoluteEquationError::CaseLimit);
    }
    let limits = context.limits();
    if total + 2 > limits.max_expression_nodes {
        return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes).into());
    }
    if left.max(right) + 1 > limits.max_expression_depth {
        return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionDepth).into());
    }
    for expression in [&equation.left, &equation.right] {
        for affine in std::iter::once(&expression.base).chain(&expression.layers) {
            check_scalar(context, work, &affine.slope)?;
            check_scalar(context, work, &affine.constant)?;
        }
    }
    Ok(total)
}

impl AbsoluteInterval {
    fn all() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    fn point(value: Rational) -> Self {
        Self {
            lower: Some(value.clone()),
            upper: Some(value),
        }
    }

    fn contains(
        &self,
        value: &Rational,
        work: &mut WorkBudget,
    ) -> Result<bool, AbsoluteEquationError> {
        if let Some(lower) = &self.lower
            && compare(work, value, lower)? == Ordering::Less
        {
            return Ok(false);
        }
        if let Some(upper) = &self.upper
            && compare(work, value, upper)? == Ordering::Greater
        {
            return Ok(false);
        }
        Ok(true)
    }

    fn valid(
        &self,
        context: &EvaluationContext,
        work: &mut WorkBudget,
        allow_point: bool,
    ) -> Result<bool, AbsoluteEquationError> {
        for bound in self.lower.iter().chain(&self.upper) {
            check_scalar(context, work, bound)?;
        }
        match (&self.lower, &self.upper) {
            (Some(lower), Some(upper)) => {
                let order = compare(work, lower, upper)?;
                Ok(order == Ordering::Less || (allow_point && order == Ordering::Equal))
            }
            _ => Ok(true),
        }
    }
}

impl AbsoluteSign {
    fn opposite(self) -> Self {
        match self {
            Self::Nonnegative => Self::Nonpositive,
            Self::Nonpositive => Self::Nonnegative,
        }
    }
}

impl AbsoluteAffine {
    fn signed(&self, branch: AbsoluteSign) -> Self {
        match branch {
            AbsoluteSign::Nonnegative => self.clone(),
            AbsoluteSign::Nonpositive => Self {
                slope: -&self.slope,
                constant: -&self.constant,
            },
        }
    }

    fn apply(
        &self,
        layer: &Self,
        branch: AbsoluteSign,
        work: &mut WorkBudget,
    ) -> Result<Self, AbsoluteEquationError> {
        let signed = self.signed(branch);
        let slope = work.multiply_scalar(&layer.slope, &signed.slope)?;
        let offset = work.multiply_scalar(&layer.slope, &signed.constant)?;
        let constant = work.add_scalar(&offset, &layer.constant)?;
        Ok(Self { slope, constant })
    }

    fn difference(
        &self,
        other: &Self,
        work: &mut WorkBudget,
    ) -> Result<Self, AbsoluteEquationError> {
        Ok(Self {
            slope: work.subtract_scalar(&self.slope, &other.slope)?,
            constant: work.subtract_scalar(&self.constant, &other.constant)?,
        })
    }

    fn at(
        &self,
        value: &Rational,
        work: &mut WorkBudget,
    ) -> Result<Rational, AbsoluteEquationError> {
        let product = work.multiply_scalar(&self.slope, value)?;
        Ok(work.add_scalar(&product, &self.constant)?)
    }

    fn root(&self, work: &mut WorkBudget) -> Result<Rational, AbsoluteEquationError> {
        Ok(work.quotient(&(-&self.constant), &self.slope)?)
    }

    // The infimum of an affine function is at its appropriate endpoint, or
    // is negative infinity. No sampling is needed to establish a sign.
    fn minimum_sign(
        &self,
        interval: &AbsoluteInterval,
        work: &mut WorkBudget,
    ) -> Result<Ordering, AbsoluteEquationError> {
        let bound = match sign(&self.slope) {
            Ordering::Equal => return Ok(sign(&self.constant)),
            Ordering::Greater => &interval.lower,
            Ordering::Less => &interval.upper,
        };
        match bound {
            Some(point) => Ok(sign(&self.at(point, work)?)),
            None => Ok(Ordering::Less),
        }
    }
}

fn split(
    interval: &AbsoluteInterval,
    affine: &AbsoluteAffine,
    work: &mut WorkBudget,
) -> Result<Vec<(AbsoluteInterval, AbsoluteSign)>, AbsoluteEquationError> {
    use AbsoluteSign::{Nonnegative, Nonpositive};

    let right_sign = match sign(&affine.slope) {
        Ordering::Equal => {
            let branch = if sign(&affine.constant) == Ordering::Less {
                Nonpositive
            } else {
                Nonnegative
            };
            return Ok(vec![(interval.clone(), branch)]);
        }
        Ordering::Greater => Nonnegative,
        Ordering::Less => Nonpositive,
    };
    let root = affine.root(work)?;
    if let Some(lower) = &interval.lower
        && compare(work, &root, lower)? != Ordering::Greater
    {
        return Ok(vec![(interval.clone(), right_sign)]);
    }
    if let Some(upper) = &interval.upper
        && compare(work, &root, upper)? != Ordering::Less
    {
        return Ok(vec![(interval.clone(), right_sign.opposite())]);
    }

    Ok(vec![
        (
            AbsoluteInterval {
                lower: interval.lower.clone(),
                upper: Some(root.clone()),
            },
            right_sign.opposite(),
        ),
        (
            AbsoluteInterval {
                lower: Some(root),
                upper: interval.upper.clone(),
            },
            right_sign,
        ),
    ])
}

#[derive(Clone)]
struct WorkingCase {
    interval: AbsoluteInterval,
    signs: Vec<AbsoluteSign>,
    forms: [AbsoluteAffine; 2],
}

fn append_solution(union: &mut Vec<AbsoluteInterval>, interval: AbsoluteInterval) {
    // The partition is ordered and has disjoint interiors, so two successive
    // solutions can intersect only at a shared finite endpoint.
    if let Some(last) = union.last_mut()
        && last.upper.is_some()
        && last.upper == interval.lower
    {
        last.upper = interval.upper;
        return;
    }
    union.push(interval);
}

fn solve_case(
    interval: &AbsoluteInterval,
    residual: &AbsoluteAffine,
    work: &mut WorkBudget,
) -> Result<Option<AbsoluteInterval>, AbsoluteEquationError> {
    if residual.slope.numerator().is_zero() {
        return Ok(if residual.constant.numerator().is_zero() {
            Some(interval.clone())
        } else {
            None
        });
    }
    let root = residual.root(work)?;
    Ok(interval
        .contains(&root, work)?
        .then(|| AbsoluteInterval::point(root)))
}

/// Solve the complete real equation by splitting at the exact zeros of each
/// affine argument. Generation and verification share a single work budget.
/// No answer is returned if either stage exhausts its budget.
pub fn solve_absolute_equation(
    context: &EvaluationContext,
    equation: &AbsoluteEquation,
) -> Result<AbsoluteEquationResult, AbsoluteEquationError> {
    let mut work = WorkBudget::new(context.limits());
    let layers = check_equation(context, &mut work, equation)?;
    let mut pieces = vec![WorkingCase {
        interval: AbsoluteInterval::all(),
        signs: Vec::new(),
        forms: [equation.left.base.clone(), equation.right.base.clone()],
    }];

    for (side, expression) in [&equation.left, &equation.right].into_iter().enumerate() {
        for layer in &expression.layers {
            let mut next = Vec::new();
            for piece in pieces {
                work.charge(1)?;
                for (interval, branch) in split(&piece.interval, &piece.forms[side], &mut work)? {
                    let mut child = piece.clone();
                    child.interval = interval;
                    child.signs.push(branch);
                    child.forms[side] = piece.forms[side].apply(layer, branch, &mut work)?;
                    next.push(child);
                }
            }
            pieces = next;
        }
    }

    let mut result = AbsoluteEquationResult {
        cases: Vec::new(),
        solutions: Vec::new(),
    };
    for piece in pieces {
        let residual = piece.forms[0].difference(&piece.forms[1], &mut work)?;
        let solution = solve_case(&piece.interval, &residual, &mut work)?;
        if let Some(interval) = &solution {
            append_solution(&mut result.solutions, interval.clone());
        }
        result.cases.push(AbsoluteCase {
            interval: piece.interval,
            signs: piece.signs,
            solution,
        });
    }
    if !verify_cases(context, equation, &result, layers, &mut work)? {
        return Err(AbsoluteEquationError::VerificationFailed);
    }
    Ok(result)
}

fn evaluate(
    expression: &AbsoluteExpression,
    point: &Rational,
    work: &mut WorkBudget,
) -> Result<Rational, AbsoluteEquationError> {
    let mut value = expression.base.at(point, work)?;
    for layer in &expression.layers {
        if sign(&value) == Ordering::Less {
            value = -value;
        }
        value = layer.at(&value, work)?;
    }
    Ok(value)
}

fn verify_case_solution(
    equation: &AbsoluteEquation,
    case: &AbsoluteCase,
    residual: &AbsoluteAffine,
    work: &mut WorkBudget,
) -> Result<bool, AbsoluteEquationError> {
    if residual.slope.numerator().is_zero() {
        return Ok(if residual.constant.numerator().is_zero() {
            case.solution.as_ref() == Some(&case.interval)
        } else {
            case.solution.is_none()
        });
    }
    let Some(solution) = &case.solution else {
        // A nonconstant affine function has no zero on a closed interval
        // exactly when it is strictly positive or strictly negative there.
        let positive = residual.minimum_sign(&case.interval, work)?;
        if positive == Ordering::Greater {
            return Ok(true);
        }
        let negative = residual
            .signed(AbsoluteSign::Nonpositive)
            .minimum_sign(&case.interval, work)?;
        return Ok(negative == Ordering::Greater);
    };
    let (Some(point), Some(upper)) = (&solution.lower, &solution.upper) else {
        return Ok(false);
    };
    if point != upper || !case.interval.contains(point, work)? {
        return Ok(false);
    }
    Ok(residual.at(point, work)?.numerator().is_zero()
        && evaluate(&equation.left, point, work)? == evaluate(&equation.right, point, work)?)
}

fn verify_cases(
    context: &EvaluationContext,
    equation: &AbsoluteEquation,
    result: &AbsoluteEquationResult,
    layers: usize,
    work: &mut WorkBudget,
) -> Result<bool, AbsoluteEquationError> {
    if result.cases.is_empty()
        || result.cases.len() > (1usize << layers)
        || result.solutions.len() > result.cases.len()
    {
        return Ok(false);
    }
    let mut union = Vec::new();
    let mut previous_upper = None;
    for (index, case) in result.cases.iter().enumerate() {
        work.charge(1)?;
        if case.signs.len() != layers || !case.interval.valid(context, work, false)? {
            return Ok(false);
        }
        if index == 0 {
            if case.interval.lower.is_some() {
                return Ok(false);
            }
        } else if previous_upper.is_none() || previous_upper != case.interval.lower.as_ref() {
            return Ok(false);
        }
        previous_upper = case.interval.upper.as_ref();

        let mut signs = case.signs.iter().copied();
        let mut forms = [equation.left.base.clone(), equation.right.base.clone()];
        for (side, expression) in [&equation.left, &equation.right].into_iter().enumerate() {
            for layer in &expression.layers {
                let branch = signs.next().expect("checked sign count");
                let affine = &forms[side];
                let minimum = affine.signed(branch).minimum_sign(&case.interval, work)?;
                if minimum == Ordering::Less {
                    return Ok(false);
                }
                forms[side] = affine.apply(layer, branch, work)?;
            }
        }

        let residual = forms[0].difference(&forms[1], work)?;
        if let Some(solution) = &case.solution
            && !solution.valid(context, work, true)?
        {
            return Ok(false);
        }
        if !verify_case_solution(equation, case, &residual, work)? {
            return Ok(false);
        }
        if let Some(solution) = &case.solution {
            append_solution(&mut union, solution.clone());
        }
    }
    if previous_upper.is_some() {
        return Ok(false);
    }
    for interval in &result.solutions {
        if !interval.valid(context, work, true)? {
            return Ok(false);
        }
    }
    Ok(union == result.solutions)
}

/// Verify evidence against the supplied original typed equation. The checker
/// proves interval coverage, sign validity and exact local solution sets; it
/// does not rerun splitting or trust the reported solution union.
pub fn verify_absolute_equation(
    context: &EvaluationContext,
    equation: &AbsoluteEquation,
    result: &AbsoluteEquationResult,
) -> Result<bool, AbsoluteEquationError> {
    let mut work = WorkBudget::new(context.limits());
    let layers = check_equation(context, &mut work, equation)?;
    verify_cases(context, equation, result, layers, &mut work)
}

const MAX_PIECEWISE_TERMS: usize = 16;

/// Finite rational endpoints may be open or closed; infinite endpoints are open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiecewiseInterval {
    pub lower: Option<Rational>,
    pub lower_closed: bool,
    pub upper: Option<Rational>,
    pub upper_closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedAbsoluteTerm {
    pub weight: Rational,
    pub argument: AbsoluteAffine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardedAffine {
    pub guard: PiecewiseInterval,
    pub form: AbsoluteAffine,
}

/// Both alternatives denote functions of the equation's single variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PiecewiseAffine {
    AbsoluteSum {
        base: AbsoluteAffine,
        terms: Vec<WeightedAbsoluteTerm>,
    },
    Explicit(Vec<GuardedAffine>),
}

/// Admission is an explicit prerequisite, not a proof about erased syntax.
/// Adapters must reject restricted or unverified construction histories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiecewiseSourceDomain {
    Total,
    RestrictedOrUnverified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiecewiseEquation {
    pub variable: Symbol,
    pub domain: ScalarDomain,
    pub source_domain: PiecewiseSourceDomain,
    pub left: PiecewiseAffine,
    pub right: PiecewiseAffine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PiecewiseBranch {
    Signs(Vec<AbsoluteSign>),
    Explicit(usize),
}

/// Each cell is an open interval or a singleton. The residual is left minus right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiecewiseCase {
    pub interval: PiecewiseInterval,
    pub left: PiecewiseBranch,
    pub right: PiecewiseBranch,
    pub residual: AbsoluteAffine,
    pub solution: Option<PiecewiseInterval>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiecewiseEquationResult {
    pub source: PiecewiseEquation,
    pub cases: Vec<PiecewiseCase>,
    pub solutions: Vec<PiecewiseInterval>,
}

impl PiecewiseEquationResult {
    pub fn method_id(&self) -> &'static str {
        "piecewise_affine_partition"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiecewiseEquationError {
    UnsupportedDomain,
    RestrictedOrUnverifiedSource,
    InvalidGuard,
    Gap,
    Overlap,
    CaseLimit,
    Arithmetic(AbsoluteEquationError),
    VerificationFailed,
}

impl fmt::Display for PiecewiseEquationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDomain => f.write_str("piecewise equations require the real domain"),
            Self::RestrictedOrUnverifiedSource => {
                f.write_str("piecewise equations require an explicitly total source")
            }
            Self::InvalidGuard => f.write_str("invalid piecewise interval guard"),
            Self::Gap => f.write_str("piecewise guards do not cover the real domain"),
            Self::Overlap => f.write_str("piecewise guards overlap"),
            Self::CaseLimit => {
                f.write_str("piecewise equations support at most sixteen terms or guards")
            }
            Self::Arithmetic(error) => write!(f, "{error}"),
            Self::VerificationFailed => {
                f.write_str("piecewise equation evidence failed verification")
            }
        }
    }
}

impl Error for PiecewiseEquationError {}

impl From<AbsoluteEquationError> for PiecewiseEquationError {
    fn from(error: AbsoluteEquationError) -> Self {
        Self::Arithmetic(error)
    }
}

impl From<EvaluationError> for PiecewiseEquationError {
    fn from(error: EvaluationError) -> Self {
        Self::Arithmetic(error.into())
    }
}

impl PiecewiseInterval {
    fn open(lower: Option<Rational>, upper: Option<Rational>) -> Self {
        Self {
            lower,
            lower_closed: false,
            upper,
            upper_closed: false,
        }
    }

    fn point(value: Rational) -> Self {
        Self {
            lower: Some(value.clone()),
            lower_closed: true,
            upper: Some(value),
            upper_closed: true,
        }
    }

    fn hull(&self) -> AbsoluteInterval {
        AbsoluteInterval {
            lower: self.lower.clone(),
            upper: self.upper.clone(),
        }
    }

    fn valid(
        &self,
        context: &EvaluationContext,
        work: &mut WorkBudget,
    ) -> Result<bool, PiecewiseEquationError> {
        if (self.lower.is_none() && self.lower_closed)
            || (self.upper.is_none() && self.upper_closed)
        {
            return Ok(false);
        }
        for value in self.lower.iter().chain(&self.upper) {
            check_scalar(context, work, value)?;
        }
        if let (Some(lower), Some(upper)) = (&self.lower, &self.upper) {
            return Ok(match compare(work, lower, upper)? {
                Ordering::Less => true,
                Ordering::Equal => self.lower_closed && self.upper_closed,
                Ordering::Greater => false,
            });
        }
        Ok(true)
    }

    fn contains(
        &self,
        point: &Rational,
        work: &mut WorkBudget,
    ) -> Result<bool, PiecewiseEquationError> {
        if let Some(lower) = &self.lower {
            let order = compare(work, point, lower)?;
            if order == Ordering::Less || (order == Ordering::Equal && !self.lower_closed) {
                return Ok(false);
            }
        }
        if let Some(upper) = &self.upper {
            let order = compare(work, point, upper)?;
            if order == Ordering::Greater || (order == Ordering::Equal && !self.upper_closed) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn subset_of(
        &self,
        guard: &Self,
        work: &mut WorkBudget,
    ) -> Result<bool, PiecewiseEquationError> {
        if let Some(bound) = &guard.lower {
            let Some(lower) = &self.lower else {
                return Ok(false);
            };
            let order = compare(work, lower, bound)?;
            if order == Ordering::Less
                || (order == Ordering::Equal && self.lower_closed && !guard.lower_closed)
            {
                return Ok(false);
            }
        }
        if let Some(bound) = &guard.upper {
            let Some(upper) = &self.upper else {
                return Ok(false);
            };
            let order = compare(work, upper, bound)?;
            if order == Ordering::Greater
                || (order == Ordering::Equal && self.upper_closed && !guard.upper_closed)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl PiecewiseAffine {
    fn size(&self) -> usize {
        match self {
            Self::AbsoluteSum { terms, .. } => terms.len(),
            Self::Explicit(cases) => cases.len(),
        }
    }
}

fn add_breakpoint(
    points: &mut Vec<Rational>,
    point: Rational,
    work: &mut WorkBudget,
) -> Result<(), PiecewiseEquationError> {
    for (index, current) in points.iter().enumerate() {
        match compare(work, &point, current)? {
            Ordering::Less => {
                points.insert(index, point);
                return Ok(());
            }
            Ordering::Equal => return Ok(()),
            Ordering::Greater => {}
        }
    }
    points.push(point);
    Ok(())
}

fn check_piecewise_affine(
    context: &EvaluationContext,
    work: &mut WorkBudget,
    form: &AbsoluteAffine,
) -> Result<(), PiecewiseEquationError> {
    check_scalar(context, work, &form.slope)?;
    check_scalar(context, work, &form.constant)?;
    Ok(())
}

// All zeros of nonconstant arguments and all finite guard boundaries suffice.
// Separate singleton cells make endpoint ownership explicit even at jumps.
fn piecewise_partition(
    context: &EvaluationContext,
    equation: &PiecewiseEquation,
    work: &mut WorkBudget,
) -> Result<Vec<PiecewiseInterval>, PiecewiseEquationError> {
    if equation.domain != ScalarDomain::Real {
        return Err(PiecewiseEquationError::UnsupportedDomain);
    }
    if equation.source_domain != PiecewiseSourceDomain::Total {
        return Err(PiecewiseEquationError::RestrictedOrUnverifiedSource);
    }
    let count = equation.left.size().saturating_add(equation.right.size());
    if count > MAX_PIECEWISE_TERMS {
        return Err(PiecewiseEquationError::CaseLimit);
    }
    if count + 2 > context.limits().max_expression_nodes {
        return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes).into());
    }
    if context.limits().max_expression_depth < 3 {
        return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionDepth).into());
    }
    let mut points = Vec::new();
    for side in [&equation.left, &equation.right] {
        match side {
            PiecewiseAffine::AbsoluteSum { base, terms } => {
                check_piecewise_affine(context, work, base)?;
                for term in terms {
                    check_scalar(context, work, &term.weight)?;
                    check_piecewise_affine(context, work, &term.argument)?;
                    if sign(&term.argument.slope) != Ordering::Equal {
                        add_breakpoint(&mut points, term.argument.root(work)?, work)?;
                    }
                }
            }
            PiecewiseAffine::Explicit(cases) => {
                for case in cases {
                    check_piecewise_affine(context, work, &case.form)?;
                    if !case.guard.valid(context, work)? {
                        return Err(PiecewiseEquationError::InvalidGuard);
                    }
                    for point in case.guard.lower.iter().chain(&case.guard.upper) {
                        add_breakpoint(&mut points, point.clone(), work)?;
                    }
                }
            }
        }
    }
    let mut cells = Vec::with_capacity(2 * points.len() + 1);
    let mut lower = None;
    for point in points {
        cells.push(PiecewiseInterval::open(lower, Some(point.clone())));
        cells.push(PiecewiseInterval::point(point.clone()));
        lower = Some(point);
    }
    cells.push(PiecewiseInterval::open(lower, None));
    Ok(cells)
}

// On this partition each guard either contains an entire cell or misses it.
fn explicit_branch(
    cases: &[GuardedAffine],
    cell: &PiecewiseInterval,
    work: &mut WorkBudget,
) -> Result<usize, PiecewiseEquationError> {
    let mut found = None;
    for (index, case) in cases.iter().enumerate() {
        work.charge(1)?;
        if cell.subset_of(&case.guard, work)? {
            if found.is_some() {
                return Err(PiecewiseEquationError::Overlap);
            }
            found = Some(index);
        }
    }
    found.ok_or(PiecewiseEquationError::Gap)
}

fn choose_piecewise_branch(
    side: &PiecewiseAffine,
    cell: &PiecewiseInterval,
    work: &mut WorkBudget,
) -> Result<PiecewiseBranch, PiecewiseEquationError> {
    match side {
        PiecewiseAffine::Explicit(cases) => Ok(PiecewiseBranch::Explicit(explicit_branch(
            cases, cell, work,
        )?)),
        PiecewiseAffine::AbsoluteSum { terms, .. } => {
            let hull = cell.hull();
            let mut signs = Vec::with_capacity(terms.len());
            for term in terms {
                let branch = if term.argument.minimum_sign(&hull, work)? == Ordering::Less {
                    AbsoluteSign::Nonpositive
                } else {
                    AbsoluteSign::Nonnegative
                };
                signs.push(branch);
            }
            Ok(PiecewiseBranch::Signs(signs))
        }
    }
}

// Validate the supplied sign witnesses over the entire cell, not a sample.
fn checked_piecewise_form(
    side: &PiecewiseAffine,
    cell: &PiecewiseInterval,
    branch: &PiecewiseBranch,
    work: &mut WorkBudget,
) -> Result<Option<AbsoluteAffine>, PiecewiseEquationError> {
    match (side, branch) {
        (PiecewiseAffine::Explicit(cases), PiecewiseBranch::Explicit(index)) => {
            let expected = explicit_branch(cases, cell, work)?;
            Ok((*index == expected).then(|| cases[expected].form.clone()))
        }
        (PiecewiseAffine::AbsoluteSum { base, terms }, PiecewiseBranch::Signs(signs))
            if terms.len() == signs.len() =>
        {
            let hull = cell.hull();
            let mut form = base.clone();
            for (term, &branch) in terms.iter().zip(signs) {
                work.charge(1)?;
                let signed = term.argument.signed(branch);
                if signed.minimum_sign(&hull, work)? == Ordering::Less {
                    return Ok(None);
                }
                let slope = work.multiply_scalar(&term.weight, &signed.slope)?;
                let constant = work.multiply_scalar(&term.weight, &signed.constant)?;
                form.slope = work.add_scalar(&form.slope, &slope)?;
                form.constant = work.add_scalar(&form.constant, &constant)?;
            }
            Ok(Some(form))
        }
        _ => Ok(None),
    }
}

fn solve_piecewise_cell(
    cell: &PiecewiseInterval,
    residual: &AbsoluteAffine,
    work: &mut WorkBudget,
) -> Result<Option<PiecewiseInterval>, PiecewiseEquationError> {
    if sign(&residual.slope) == Ordering::Equal {
        return Ok((sign(&residual.constant) == Ordering::Equal).then(|| cell.clone()));
    }
    let root = residual.root(work)?;
    Ok(cell
        .contains(&root, work)?
        .then(|| PiecewiseInterval::point(root)))
}

fn append_piecewise_solution(union: &mut Vec<PiecewiseInterval>, interval: PiecewiseInterval) {
    if let Some(last) = union.last_mut()
        && last.upper.is_some()
        && last.upper == interval.lower
        && (last.upper_closed || interval.lower_closed)
    {
        last.upper = interval.upper;
        last.upper_closed = interval.upper_closed;
        return;
    }
    union.push(interval);
}

// For a nonconstant affine form a zero infimum is strictly positive on
// the cell only when its minimizing endpoint is excluded.
fn strictly_positive(
    form: &AbsoluteAffine,
    cell: &PiecewiseInterval,
    work: &mut WorkBudget,
) -> Result<bool, PiecewiseEquationError> {
    let minimum = form.minimum_sign(&cell.hull(), work)?;
    Ok(minimum == Ordering::Greater
        || (minimum == Ordering::Equal
            && match sign(&form.slope) {
                Ordering::Greater => !cell.lower_closed,
                Ordering::Less => !cell.upper_closed,
                Ordering::Equal => false,
            }))
}

fn verify_piecewise_cell(
    case: &PiecewiseCase,
    residual: &AbsoluteAffine,
    work: &mut WorkBudget,
) -> Result<bool, PiecewiseEquationError> {
    if sign(&residual.slope) == Ordering::Equal {
        return Ok(if sign(&residual.constant) == Ordering::Equal {
            case.solution.as_ref() == Some(&case.interval)
        } else {
            case.solution.is_none()
        });
    }
    let Some(solution) = &case.solution else {
        return Ok(strictly_positive(residual, &case.interval, work)?
            || strictly_positive(
                &residual.signed(AbsoluteSign::Nonpositive),
                &case.interval,
                work,
            )?);
    };
    let Some(point) = &solution.lower else {
        return Ok(false);
    };
    Ok(solution.upper.as_ref() == Some(point)
        && solution.lower_closed
        && solution.upper_closed
        && case.interval.contains(point, work)?
        && sign(&residual.at(point, work)?) == Ordering::Equal)
}

fn verify_piecewise_cases(
    context: &EvaluationContext,
    equation: &PiecewiseEquation,
    result: &PiecewiseEquationResult,
    cells: &[PiecewiseInterval],
    work: &mut WorkBudget,
) -> Result<bool, PiecewiseEquationError> {
    if result.cases.len() != cells.len()
        || result.solutions.len() > cells.len()
        || &result.source != equation
    {
        return Ok(false);
    }
    let mut union = Vec::new();
    for (case, cell) in result.cases.iter().zip(cells) {
        work.charge(1)?;
        if !case.interval.valid(context, work)? || &case.interval != cell {
            return Ok(false);
        }
        let Some(left) = checked_piecewise_form(&equation.left, cell, &case.left, work)? else {
            return Ok(false);
        };
        let Some(right) = checked_piecewise_form(&equation.right, cell, &case.right, work)? else {
            return Ok(false);
        };
        let residual = left.difference(&right, work)?;
        check_piecewise_affine(context, work, &case.residual)?;
        if case.residual != residual {
            return Ok(false);
        }
        if let Some(solution) = &case.solution
            && !solution.valid(context, work)?
        {
            return Ok(false);
        }
        if !verify_piecewise_cell(case, &residual, work)? {
            return Ok(false);
        }
        if let Some(solution) = &case.solution {
            append_piecewise_solution(&mut union, solution.clone());
        }
    }
    for solution in &result.solutions {
        if !solution.valid(context, work)? {
            return Ok(false);
        }
    }
    Ok(union == result.solutions)
}

impl EvaluationContext {
    /// Solve a total real equation with at most sixteen weighted terms or guards
    /// across both sides (at most 65 cells). Ambiguous or incomplete guards are
    /// refused. Construction and verification share the same work budget.
    pub fn solve_piecewise_equation(
        &self,
        equation: &PiecewiseEquation,
    ) -> Result<PiecewiseEquationResult, PiecewiseEquationError> {
        let mut work = WorkBudget::new(self.limits());
        let cells = piecewise_partition(self, equation, &mut work)?;
        let mut result = PiecewiseEquationResult {
            source: equation.clone(),
            cases: Vec::with_capacity(cells.len()),
            solutions: Vec::new(),
        };
        for cell in &cells {
            work.charge(1)?;
            let left = choose_piecewise_branch(&equation.left, cell, &mut work)?;
            let right = choose_piecewise_branch(&equation.right, cell, &mut work)?;
            let left_form = checked_piecewise_form(&equation.left, cell, &left, &mut work)?
                .ok_or(PiecewiseEquationError::VerificationFailed)?;
            let right_form = checked_piecewise_form(&equation.right, cell, &right, &mut work)?
                .ok_or(PiecewiseEquationError::VerificationFailed)?;
            let residual = left_form.difference(&right_form, &mut work)?;
            let solution = solve_piecewise_cell(cell, &residual, &mut work)?;
            if let Some(interval) = &solution {
                append_piecewise_solution(&mut result.solutions, interval.clone());
            }
            result.cases.push(PiecewiseCase {
                interval: cell.clone(),
                left,
                right,
                residual,
                solution,
            });
        }
        if !verify_piecewise_cases(self, equation, &result, &cells, &mut work)? {
            return Err(PiecewiseEquationError::VerificationFailed);
        }
        Ok(result)
    }

    /// Reconstruct only the required partition from the supplied source, then
    /// verify every branch witness and local solution without running the solver.
    pub fn verify_piecewise_equation(
        &self,
        equation: &PiecewiseEquation,
        result: &PiecewiseEquationResult,
    ) -> Result<bool, PiecewiseEquationError> {
        let mut work = WorkBudget::new(self.limits());
        let cells = piecewise_partition(self, equation, &mut work)?;
        verify_piecewise_cases(self, equation, result, &cells, &mut work)
    }
}

#[cfg(test)]
mod piecewise_tests {
    use super::*;
    use crate::ResourceLimits;

    fn q(n: i64) -> Rational {
        Rational::from(Integer::from(n))
    }

    fn affine(a: i64, b: i64) -> AbsoluteAffine {
        AbsoluteAffine {
            slope: q(a),
            constant: q(b),
        }
    }

    fn sum(a: i64, b: i64, terms: &[(i64, i64, i64)]) -> PiecewiseAffine {
        PiecewiseAffine::AbsoluteSum {
            base: affine(a, b),
            terms: terms
                .iter()
                .map(|&(w, s, c)| WeightedAbsoluteTerm {
                    weight: q(w),
                    argument: affine(s, c),
                })
                .collect(),
        }
    }

    fn equation(left: PiecewiseAffine, right: PiecewiseAffine) -> PiecewiseEquation {
        PiecewiseEquation {
            variable: Symbol::new("x").unwrap(),
            domain: ScalarDomain::Real,
            source_domain: PiecewiseSourceDomain::Total,
            left,
            right,
        }
    }

    fn interval(
        lower: Option<i64>,
        lower_closed: bool,
        upper: Option<i64>,
        upper_closed: bool,
    ) -> PiecewiseInterval {
        PiecewiseInterval {
            lower: lower.map(q),
            lower_closed,
            upper: upper.map(q),
            upper_closed,
        }
    }

    fn assert_solutions(
        source: &PiecewiseEquation,
        expected: Vec<PiecewiseInterval>,
    ) -> PiecewiseEquationResult {
        let context = EvaluationContext::default();
        let result = context.solve_piecewise_equation(source).unwrap();
        assert_eq!(result.solutions, expected);
        assert_eq!(context.verify_piecewise_equation(source, &result), Ok(true));
        assert_eq!(context.solve_piecewise_equation(source), Ok(result.clone()));
        assert_eq!(result.method_id(), "piecewise_affine_partition");
        result
    }

    #[test]
    fn sums_and_differences_of_absolute_terms() {
        let left = sum(0, 0, &[(1, 1, -1), (1, 1, 1)]);
        assert_solutions(
            &equation(left.clone(), sum(0, 4, &[])),
            vec![
                PiecewiseInterval::point(q(-2)),
                PiecewiseInterval::point(q(2)),
            ],
        );
        assert_solutions(
            &equation(left.clone(), sum(0, 2, &[])),
            vec![interval(Some(-1), true, Some(1), true)],
        );
        assert_solutions(&equation(left, sum(0, 1, &[])), vec![]);
        assert_solutions(
            &equation(sum(0, 0, &[(1, 1, -1), (-1, 1, 1)]), sum(0, 2, &[])),
            vec![interval(None, false, Some(-1), true)],
        );
    }

    #[test]
    fn rays_constants_and_exact_rational_roots() {
        assert_solutions(
            &equation(sum(0, 0, &[(1, 1, 0)]), sum(1, 0, &[])),
            vec![interval(Some(0), true, None, false)],
        );
        assert_solutions(
            &equation(sum(0, 0, &[(1, 2, -5)]), sum(0, 7, &[])),
            vec![
                PiecewiseInterval::point(q(-1)),
                PiecewiseInterval::point(q(6)),
            ],
        );
        assert_solutions(
            &equation(sum(0, 0, &[(1, 2, 0)]), sum(0, 1, &[])),
            vec![
                PiecewiseInterval::point(
                    Rational::new(Integer::from(-1), Integer::from(2)).unwrap(),
                ),
                PiecewiseInterval::point(
                    Rational::new(Integer::from(1), Integer::from(2)).unwrap(),
                ),
            ],
        );
        assert_solutions(
            &equation(sum(0, 0, &[(1, 0, -3), (-1, 0, 3)]), sum(0, 0, &[])),
            vec![interval(None, false, None, false)],
        );
    }

    fn guarded(guard: PiecewiseInterval, a: i64, b: i64) -> GuardedAffine {
        GuardedAffine {
            guard,
            form: affine(a, b),
        }
    }

    #[test]
    fn explicit_discontinuous_cases_and_singletons() {
        let left = PiecewiseAffine::Explicit(vec![
            guarded(interval(Some(0), false, None, false), 0, 0),
            guarded(interval(None, false, Some(0), false), 0, 0),
            guarded(interval(Some(0), true, Some(0), true), 0, 1),
        ]);
        let source = equation(left.clone(), sum(0, 0, &[]));
        assert_solutions(
            &source,
            vec![
                interval(None, false, Some(0), false),
                interval(Some(0), false, None, false),
            ],
        );
        assert_solutions(
            &equation(left, sum(0, 1, &[])),
            vec![PiecewiseInterval::point(q(0))],
        );
    }

    #[test]
    fn mixed_operands_and_two_explicit_partitions() {
        let left = PiecewiseAffine::Explicit(vec![
            guarded(interval(None, false, Some(0), true), -1, 0),
            guarded(interval(Some(0), false, None, false), 1, 0),
        ]);
        assert_solutions(
            &equation(left.clone(), sum(0, 0, &[(1, 1, 0)])),
            vec![interval(None, false, None, false)],
        );
        let right = PiecewiseAffine::Explicit(vec![
            guarded(interval(None, false, Some(1), false), 0, 1),
            guarded(interval(Some(1), true, None, false), 0, 2),
        ]);
        assert_solutions(
            &equation(left, right),
            vec![
                PiecewiseInterval::point(q(-1)),
                PiecewiseInterval::point(q(2)),
            ],
        );
    }

    #[test]
    fn explicit_ambiguity_gaps_and_invalid_guards_are_refused() {
        let context = EvaluationContext::default();
        for (guards, expected) in [
            (
                vec![
                    guarded(interval(None, false, Some(0), true), 0, 0),
                    guarded(interval(Some(0), true, None, false), 0, 0),
                ],
                PiecewiseEquationError::Overlap,
            ),
            (
                vec![
                    guarded(interval(None, false, Some(0), false), 0, 0),
                    guarded(interval(Some(0), false, None, false), 0, 0),
                ],
                PiecewiseEquationError::Gap,
            ),
            (
                vec![
                    guarded(interval(None, false, Some(1), false), 0, 0),
                    guarded(interval(Some(0), false, None, false), 0, 0),
                ],
                PiecewiseEquationError::Overlap,
            ),
            (
                vec![guarded(interval(Some(1), false, Some(1), true), 0, 0)],
                PiecewiseEquationError::InvalidGuard,
            ),
        ] {
            let source = equation(PiecewiseAffine::Explicit(guards), sum(0, 0, &[]));
            assert_eq!(context.solve_piecewise_equation(&source), Err(expected));
        }
    }

    #[test]
    fn forged_partition_sign_residual_and_union_fail() {
        let source = equation(sum(0, 0, &[(1, 1, -1), (1, 1, 1)]), sum(0, 2, &[]));
        let context = EvaluationContext::default();
        let result = context.solve_piecewise_equation(&source).unwrap();
        let mut forged = result.clone();
        forged.cases.remove(1);
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
        let mut forged = result.clone();
        forged.cases[0].interval.upper = Some(q(0));
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
        let mut forged = result.clone();
        forged.cases[0].left = PiecewiseBranch::Signs(vec![AbsoluteSign::Nonnegative; 2]);
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
        let mut forged = result.clone();
        forged.cases[2].residual.constant = q(1);
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
        let mut forged = result.clone();
        forged.cases[1].solution = None;
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
        let mut forged = result;
        forged.solutions[0].lower_closed = false;
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
    }

    #[test]
    fn forged_explicit_endpoint_and_branch_index_fail() {
        let source = equation(
            PiecewiseAffine::Explicit(vec![
                guarded(interval(None, false, Some(0), false), 1, 0),
                guarded(interval(Some(0), true, None, false), 0, 1),
            ]),
            sum(0, 0, &[]),
        );
        let context = EvaluationContext::default();
        let result = context.solve_piecewise_equation(&source).unwrap();
        assert!(result.solutions.is_empty());
        let mut forged = result.clone();
        forged.cases[0].solution = Some(PiecewiseInterval::point(q(0)));
        forged.solutions.push(PiecewiseInterval::point(q(0)));
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
        let mut forged = result;
        forged.cases[1].left = PiecewiseBranch::Explicit(0);
        assert_eq!(
            context.verify_piecewise_equation(&source, &forged),
            Ok(false)
        );
    }

    #[test]
    fn domain_and_resource_refusals_are_not_empty_answers() {
        let context = EvaluationContext::default();
        let mut source = equation(sum(0, 0, &[(1, 1, 0)]), sum(0, 0, &[]));
        source.source_domain = PiecewiseSourceDomain::RestrictedOrUnverified;
        assert_eq!(
            context.solve_piecewise_equation(&source),
            Err(PiecewiseEquationError::RestrictedOrUnverifiedSource)
        );
        source.source_domain = PiecewiseSourceDomain::Total;
        source.domain = ScalarDomain::Complex;
        assert_eq!(
            context.solve_piecewise_equation(&source),
            Err(PiecewiseEquationError::UnsupportedDomain)
        );
        let excessive = equation(sum(0, 0, &[(1, 1, 0); 17]), sum(0, 0, &[]));
        assert_eq!(
            context.solve_piecewise_equation(&excessive),
            Err(PiecewiseEquationError::CaseLimit)
        );
        let source = equation(sum(0, 0, &[(1, 1, 0)]), sum(0, 0, &[]));
        let result = context.solve_piecewise_equation(&source).unwrap();
        let limited = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        let error = PiecewiseEquationError::Arithmetic(AbsoluteEquationError::Evaluation(
            EvaluationError::Limit(ResourceLimitKind::WorkUnits),
        ));
        assert_eq!(limited.solve_piecewise_equation(&source), Err(error));
        assert_eq!(
            limited.verify_piecewise_equation(&source, &result),
            Err(error)
        );
    }

    fn eval_sum(side: &PiecewiseAffine, x: &Rational) -> Rational {
        let PiecewiseAffine::AbsoluteSum { base, terms } = side else {
            panic!("test requires absolute sum");
        };
        let mut value = &base.slope * x + base.constant.clone();
        for term in terms {
            let mut arg = &term.argument.slope * x + term.argument.constant.clone();
            if arg.numerator() < &Integer::zero() {
                arg = -arg;
            }
            value = value + &term.weight * &arg;
        }
        value
    }

    #[test]
    fn exact_substitution_and_side_swap_on_rational_grid() {
        let context = EvaluationContext::default();
        for a in -2..=2 {
            for b in -2..=2 {
                let source = equation(
                    sum(a, 1, &[(1, 2, b), (-2, 1, -1)]),
                    sum(b, 0, &[(1, -1, a)]),
                );
                let result = context.solve_piecewise_equation(&source).unwrap();
                let reversed = equation(source.right.clone(), source.left.clone());
                let swapped = context.solve_piecewise_equation(&reversed).unwrap();
                assert_eq!(result.solutions, swapped.solutions);
                for n in -12..=12 {
                    let x = Rational::new(Integer::from(n), Integer::from(3)).unwrap();
                    let mut work = WorkBudget::new(context.limits());
                    let included = result
                        .solutions
                        .iter()
                        .any(|interval| interval.contains(&x, &mut work).unwrap());
                    assert_eq!(
                        included,
                        eval_sum(&source.left, &x) == eval_sum(&source.right, &x)
                    );
                }
            }
        }
    }
}
