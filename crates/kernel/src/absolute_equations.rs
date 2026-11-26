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
