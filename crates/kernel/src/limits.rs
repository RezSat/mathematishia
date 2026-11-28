//! Checked finite-point rational and standard analytic limits, and rational limits at infinity.
//!
//! A removed point stays removed from the source function even when its
//! punctured-neighborhood limit exists. The certificate proves local
//! factorization and signs using exact rational arithmetic.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{
    Condition, DomainExpression, Expr, ExprView, Integer, Rational, ScalarDomain, Symbol,
    definedness_obligations_in_domain,
};

use crate::budget::WorkBudget;
use crate::{
    EvaluationContext, EvaluationError, RationalExpressionError, RationalFunction,
    RationalPolynomial,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LimitDirection {
    Left,
    Right,
    TwoSided,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LimitValue {
    Finite(Rational),
    PositiveInfinity,
    NegativeInfinity,
    DoesNotExist,
}

/// Exact identity p(x) = (x - a)^order * cofactor(x), with cofactor(a) != 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitFactor {
    order: usize,
    cofactor: RationalPolynomial,
}

impl LimitFactor {
    pub const fn order(&self) -> usize {
        self.order
    }

    pub fn cofactor(&self) -> &RationalPolynomial {
        &self.cofactor
    }
}

/// A checked local factorization, direction, result and source-domain record.
///
/// A missing numerator factor denotes the identically zero numerator.
/// Source exclusions are retained in the embedded partial rational function;
/// their exact evaluations at the target are recorded, never discarded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalLimit {
    source: RationalFunction,
    at: Rational,
    direction: LimitDirection,
    value: LimitValue,
    numerator_factor: Option<LimitFactor>,
    denominator_factor: LimitFactor,
    exclusion_values: Vec<Rational>,
}

impl RationalLimit {
    pub fn source(&self) -> &RationalFunction {
        &self.source
    }

    pub fn at(&self) -> &Rational {
        &self.at
    }

    pub const fn direction(&self) -> LimitDirection {
        self.direction
    }

    pub fn value(&self) -> &LimitValue {
        &self.value
    }

    pub fn numerator_factor(&self) -> Option<&LimitFactor> {
        self.numerator_factor.as_ref()
    }

    pub fn denominator_factor(&self) -> &LimitFactor {
        &self.denominator_factor
    }

    pub fn exclusion_values(&self) -> &[Rational] {
        &self.exclusion_values
    }

    /// Whether the original partial function excludes the target itself.
    /// This does not prevent a limit through admissible nearby points.
    pub fn excluded_at_point(&self) -> bool {
        self.exclusion_values
            .iter()
            .any(|value| value.numerator().is_zero())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitError {
    Evaluation(EvaluationError),
    Conversion(RationalExpressionError),
    Verification(LimitVerificationError),
    InconsistentFactorization,
}

impl fmt::Display for LimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for LimitError {}

impl From<EvaluationError> for LimitError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl From<RationalExpressionError> for LimitError {
    fn from(error: RationalExpressionError) -> Self {
        Self::Conversion(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitVerificationError {
    InvalidCertificate,
    Evaluation(EvaluationError),
}

impl fmt::Display for LimitVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for LimitVerificationError {}

impl From<EvaluationError> for LimitVerificationError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

fn zero() -> Rational {
    Rational::from(Integer::zero())
}

fn one() -> Rational {
    Rational::from(Integer::one())
}

fn linear_factor(at: &Rational) -> RationalPolynomial {
    RationalPolynomial::new(vec![-at, one()])
}

fn factor_at(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
    at: &Rational,
) -> Result<LimitFactor, LimitError> {
    let linear = linear_factor(at);
    let mut cofactor = polynomial.clone();
    let mut order = 0;
    loop {
        let evaluation = work.evaluate(&cofactor, at)?;
        if !evaluation.numerator().is_zero() {
            break;
        }
        let (quotient, remainder) = work.divide(&cofactor, &linear)?;
        if !remainder.is_zero() {
            return Err(LimitError::InconsistentFactorization);
        }
        cofactor = quotient;
        order += 1;
    }
    Ok(LimitFactor { order, cofactor })
}

fn verify_factor(
    work: &mut WorkBudget,
    original: &RationalPolynomial,
    at: &Rational,
    claimed: &LimitFactor,
) -> Result<Rational, LimitVerificationError> {
    if claimed.order > original.degree().unwrap_or(0) {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    let evaluation = work.evaluate(&claimed.cofactor, at)?;
    if evaluation.numerator().is_zero() {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    let linear = linear_factor(at);
    let mut reconstructed = claimed.cofactor.clone();
    for _ in 0..claimed.order {
        reconstructed = work.multiply(&reconstructed, &linear)?;
    }
    if reconstructed != *original {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    Ok(evaluation)
}

fn classify(
    work: &mut WorkBudget,
    numerator: Option<(&LimitFactor, &Rational)>,
    denominator: (&LimitFactor, &Rational),
    direction: LimitDirection,
) -> Result<LimitValue, EvaluationError> {
    let Some((numerator, leading_n)) = numerator else {
        return Ok(LimitValue::Finite(zero()));
    };
    let (denominator, leading_d) = denominator;
    match numerator.order.cmp(&denominator.order) {
        Ordering::Greater => Ok(LimitValue::Finite(zero())),
        Ordering::Equal => Ok(LimitValue::Finite(work.quotient(leading_n, leading_d)?)),
        Ordering::Less => {
            let positive_n = leading_n.numerator() > &Integer::zero();
            let positive_d = leading_d.numerator() > &Integer::zero();
            let positive_right = positive_n == positive_d;
            let odd_pole = (denominator.order - numerator.order) % 2 == 1;
            if direction == LimitDirection::TwoSided && odd_pole {
                return Ok(LimitValue::DoesNotExist);
            }
            let positive = if direction == LimitDirection::Left && odd_pole {
                !positive_right
            } else {
                positive_right
            };
            Ok(if positive {
                LimitValue::PositiveInfinity
            } else {
                LimitValue::NegativeInfinity
            })
        }
    }
}

impl EvaluationContext {
    /// Computes a finite-point one-sided or two-sided limit of a partial Q(x)
    /// function. No target evaluation or numerical sampling is performed.
    ///
    /// A nonzero rational denominator is required; transcendental and symbolic
    /// targets are not part of this API. Work and bit limits are enforced
    /// during factor division, evaluation and independent verification.
    pub fn rational_limit(
        &self,
        source: &RationalFunction,
        at: &Rational,
        direction: LimitDirection,
    ) -> Result<RationalLimit, LimitError> {
        self.check_rational_function(source)?;
        self.check_rational(at)?;
        let mut work = WorkBudget::new(self.limits());
        work.charge(1)?;

        let numerator_factor = if source.numerator().is_zero() {
            None
        } else {
            Some(factor_at(&mut work, source.numerator(), at)?)
        };
        let denominator_factor = factor_at(&mut work, source.denominator(), at)?;
        let numerator_value = numerator_factor
            .as_ref()
            .map(|factor| work.evaluate(&factor.cofactor, at))
            .transpose()?;
        let denominator_value = work.evaluate(&denominator_factor.cofactor, at)?;
        let exclusion_values = source
            .excluded_zero_sets()
            .iter()
            .map(|condition| work.evaluate(condition, at))
            .collect::<Result<Vec<_>, _>>()?;

        let value = classify(
            &mut work,
            numerator_factor.as_ref().zip(numerator_value.as_ref()),
            (&denominator_factor, &denominator_value),
            direction,
        )?;
        let result = RationalLimit {
            source: source.clone(),
            at: at.clone(),
            direction,
            value,
            numerator_factor,
            denominator_factor,
            exclusion_values,
        };
        verify_rational_limit(self, source, at, direction, &result)
            .map_err(LimitError::Verification)?;
        Ok(result)
    }

    /// Converts a domain-aware source without erasing historical exclusions.
    /// Unsupported functions, foreign symbols and powers return typed errors.
    pub fn rational_expression_limit(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        at: &Rational,
        direction: LimitDirection,
    ) -> Result<RationalLimit, LimitError> {
        let rational = self.rational_expression(source, variable)?;
        self.rational_limit(&rational, at, direction)
    }
}

/// Independently verifies local polynomial identities, nonvanishing cofactors,
/// all recorded source restrictions and the exact directional classification.
/// It never calls the limit generator and fails closed on exhausted budgets.
pub fn verify_rational_limit(
    ctx: &EvaluationContext,
    source: &RationalFunction,
    at: &Rational,
    direction: LimitDirection,
    claimed: &RationalLimit,
) -> Result<(), LimitVerificationError> {
    ctx.check_rational_function(source)?;
    ctx.check_rational(at)?;
    if claimed.source != *source || claimed.at != *at || claimed.direction != direction {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    let mut work = WorkBudget::new(ctx.limits());
    work.charge(1)?;
    let numerator_value = match (&claimed.numerator_factor, source.numerator().is_zero()) {
        (None, true) => None,
        (Some(factor), false) => Some(verify_factor(&mut work, source.numerator(), at, factor)?),
        _ => return Err(LimitVerificationError::InvalidCertificate),
    };
    let denominator_value = verify_factor(
        &mut work,
        source.denominator(),
        at,
        &claimed.denominator_factor,
    )?;
    if claimed.exclusion_values.len() != source.excluded_zero_sets().len() {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    for (condition, recorded) in source
        .excluded_zero_sets()
        .iter()
        .zip(&claimed.exclusion_values)
    {
        if work.evaluate(condition, at)? != *recorded {
            return Err(LimitVerificationError::InvalidCertificate);
        }
    }
    let expected = classify(
        &mut work,
        claimed
            .numerator_factor
            .as_ref()
            .zip(numerator_value.as_ref()),
        (&claimed.denominator_factor, &denominator_value),
        direction,
    )?;
    if expected != claimed.value {
        return Err(LimitVerificationError::InvalidCertificate);
    }
    Ok(())
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;
    use crate::ResourceLimits;

    fn r(n: i64) -> Rational {
        Rational::from(Integer::from(n))
    }

    fn p(coefs: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(coefs.iter().copied().map(r).collect())
    }

    #[test]
    fn forged_factors_domain_claims_and_answers_are_rejected() {
        let ctx = EvaluationContext::default();
        let source = ctx
            .rational_function(&p(&[-1, 0, 1]), &p(&[-1, 1]))
            .unwrap();
        let at = r(1);
        let good = ctx
            .rational_limit(&source, &at, LimitDirection::TwoSided)
            .unwrap();
        assert_eq!(
            verify_rational_limit(&ctx, &source, &at, LimitDirection::TwoSided, &good),
            Ok(())
        );

        let mut false_value = good.clone();
        false_value.value = LimitValue::Finite(r(3));
        let mut false_order = good.clone();
        false_order.numerator_factor.as_mut().unwrap().order = 1;
        let mut false_cofactor = good.clone();
        false_cofactor.denominator_factor.cofactor = p(&[2]);
        let mut false_domain = good.clone();
        false_domain.exclusion_values[0] = r(1);
        let mut false_point = good.clone();
        false_point.at = r(2);
        for bad in [
            false_value,
            false_order,
            false_cofactor,
            false_domain,
            false_point,
        ] {
            assert_eq!(
                verify_rational_limit(&ctx, &source, &at, LimitDirection::TwoSided, &bad),
                Err(LimitVerificationError::InvalidCertificate)
            );
        }
    }

    #[test]
    fn zero_numerator_cannot_be_forged_into_nonzero_evidence() {
        let ctx = EvaluationContext::default();
        let source = ctx.rational_function(&p(&[]), &p(&[0, 1])).unwrap();
        let mut result = ctx
            .rational_limit(&source, &r(0), LimitDirection::Left)
            .unwrap();
        result.numerator_factor = Some(LimitFactor {
            order: 0,
            cofactor: p(&[1]),
        });
        assert_eq!(
            verify_rational_limit(&ctx, &source, &r(0), LimitDirection::Left, &result),
            Err(LimitVerificationError::InvalidCertificate)
        );
    }

    #[test]
    fn verifier_does_not_ignore_exhausted_budget() {
        let ctx = EvaluationContext::default();
        let source = ctx.rational_function(&p(&[1]), &p(&[0, 1])).unwrap();
        let result = ctx
            .rational_limit(&source, &r(0), LimitDirection::Right)
            .unwrap();
        let restricted = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        assert_eq!(
            verify_rational_limit(&restricted, &source, &r(0), LimitDirection::Right, &result),
            Err(LimitVerificationError::Evaluation(EvaluationError::Limit(
                crate::ResourceLimitKind::WorkUnits
            )))
        );
    }
}

/// The real end of the line being approached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InfinityEnd {
    Positive,
    Negative,
}

/// Leading term and a conservative strict tail bound. For |x| > tail_bound,
/// the polynomial has the sign of its leading monomial and cannot vanish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LimitLeadingTerm {
    pub degree: usize,
    pub coefficient: Rational,
    pub tail_bound: Rational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalInfinityLimit {
    pub source: RationalFunction,
    pub end: InfinityEnd,
    pub value: LimitValue,
    pub numerator: Option<LimitLeadingTerm>,
    pub denominator: LimitLeadingTerm,
    pub exclusions: Vec<LimitLeadingTerm>,
}

fn absolute(value: &Rational) -> Rational {
    if value.numerator() < &Integer::zero() {
        -value
    } else {
        value.clone()
    }
}

fn leading_term(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
) -> Result<Option<LimitLeadingTerm>, EvaluationError> {
    work.charge(1)?;
    let Some((coefficient, lower)) = polynomial.coefficients().split_last() else {
        return Ok(None);
    };
    if coefficient.numerator().is_zero() {
        return Err(EvaluationError::DivisionByZero);
    }
    let mut sum = zero();
    for item in lower {
        sum = work.add_scalar(&sum, &absolute(item))?;
    }
    let ratio = work.quotient(&sum, &absolute(coefficient))?;
    let tail_bound = work.add_scalar(&one(), &ratio)?;
    Ok(Some(LimitLeadingTerm {
        degree: lower.len(),
        coefficient: coefficient.clone(),
        tail_bound,
    }))
}

fn infinity_value(
    work: &mut WorkBudget,
    numerator: Option<&LimitLeadingTerm>,
    denominator: &LimitLeadingTerm,
    end: InfinityEnd,
) -> Result<LimitValue, EvaluationError> {
    let Some(numerator) = numerator else {
        return Ok(LimitValue::Finite(zero()));
    };
    match numerator.degree.cmp(&denominator.degree) {
        Ordering::Less => Ok(LimitValue::Finite(zero())),
        Ordering::Equal => Ok(LimitValue::Finite(
            work.quotient(&numerator.coefficient, &denominator.coefficient)?,
        )),
        Ordering::Greater => {
            let same_sign = (numerator.coefficient.numerator() > &Integer::zero())
                == (denominator.coefficient.numerator() > &Integer::zero());
            let flip = end == InfinityEnd::Negative
                && (numerator.degree - denominator.degree) % 2 == 1;
            Ok(if same_sign != flip {
                LimitValue::PositiveInfinity
            } else {
                LimitValue::NegativeInfinity
            })
        }
    }
}

impl EvaluationContext {
    /// Exact limits of partial Q(x) functions at either real infinity.
    /// Nonzero exclusion polynomials have no roots beyond their certified bounds.
    pub fn rational_infinity_limit(
        &self,
        source: &RationalFunction,
        end: InfinityEnd,
    ) -> Result<RationalInfinityLimit, LimitError> {
        self.check_rational_function(source)?;
        let mut work = WorkBudget::new(self.limits());
        let numerator = leading_term(&mut work, source.numerator())?;
        let denominator = leading_term(&mut work, source.denominator())?
            .ok_or(LimitError::InconsistentFactorization)?;
        let exclusions = source
            .excluded_zero_sets()
            .iter()
            .map(|polynomial| {
                leading_term(&mut work, polynomial)?
                    .ok_or(LimitError::InconsistentFactorization)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let value = infinity_value(&mut work, numerator.as_ref(), &denominator, end)?;
        let result = RationalInfinityLimit {
            source: source.clone(),
            end,
            value,
            numerator,
            denominator,
            exclusions,
        };
        result
            .verify(self, source, end)
            .map_err(LimitError::Verification)?;
        Ok(result)
    }

    pub fn rational_expression_infinity_limit(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        end: InfinityEnd,
    ) -> Result<RationalInfinityLimit, LimitError> {
        let rational = self.rational_expression(source, variable)?;
        self.rational_infinity_limit(&rational, end)
    }
}

impl RationalInfinityLimit {
    /// Verify against an independently supplied partial rational function.
    /// For a nonzero p of degree d, division by x^d gives its leading
    /// coefficient plus terms tending to zero. The tail bound also proves
    /// eventual nonvanishing of every denominator and historical exclusion.
    pub fn verify(
        &self,
        ctx: &EvaluationContext,
        source: &RationalFunction,
        end: InfinityEnd,
    ) -> Result<(), LimitVerificationError> {
        ctx.check_rational_function(source)?;
        if self.source != *source
            || self.end != end
            || self.exclusions.len() != source.excluded_zero_sets().len()
        {
            return Err(LimitVerificationError::InvalidCertificate);
        }
        let mut work = WorkBudget::new(ctx.limits());
        if leading_term(&mut work, source.numerator())? != self.numerator
            || leading_term(&mut work, source.denominator())?.as_ref() != Some(&self.denominator)
        {
            return Err(LimitVerificationError::InvalidCertificate);
        }
        for (polynomial, evidence) in source.excluded_zero_sets().iter().zip(&self.exclusions) {
            if leading_term(&mut work, polynomial)?.as_ref() != Some(evidence) {
                return Err(LimitVerificationError::InvalidCertificate);
            }
        }
        let value = infinity_value(&mut work, self.numerator.as_ref(), &self.denominator, end)?;
        if value != self.value {
            return Err(LimitVerificationError::InvalidCertificate);
        }
        Ok(())
    }
}

/// The standard real limits used by the bounded zero-point matcher.
/// Trigonometric arguments are in radians.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardLimitRule {
    Sine,
    OneMinusCosine,
    ExponentialMinusOne,
    NaturalLogarithm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalyticLimitError {
    Evaluation(EvaluationError),
    Unsupported,
    InvalidCertificate,
}

impl From<EvaluationError> for AnalyticLimitError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl fmt::Display for AnalyticLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for AnalyticLimitError {}

/// A final certificate, not a trace of numerical estimates or differentiation.
/// The source is a real-domain expression; all its conditions are proved on
/// some two-sided punctured neighborhood of zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyticZeroLimit {
    pub source: DomainExpression,
    pub variable: Symbol,
    pub direction: LimitDirection,
    pub rule: StandardLimitRule,
    pub argument_scale: Rational,
    pub outer_scale: Rational,
    pub denominator_order: usize,
    pub value: LimitValue,
}

fn exact_scalar(expr: &Expr) -> Option<Rational> {
    match expr.view() {
        ExprView::Integer(n) => Some(Rational::from(n.clone())),
        ExprView::Rational(q) => Some(q.clone()),
        _ => None,
    }
}

fn polynomial(
    work: &mut WorkBudget,
    expr: &Expr,
    variable: &Symbol,
) -> Result<RationalPolynomial, AnalyticLimitError> {
    match work.convert(expr, variable) {
        Err(EvaluationError::Polynomial(
            crate::PolynomialConversionError::OtherSymbol
            | crate::PolynomialConversionError::UnsupportedPower
            | crate::PolynomialConversionError::UnsupportedFunction,
        )) => Err(AnalyticLimitError::Unsupported),
        result => Ok(result?),
    }
}

fn check_analytic_source(
    ctx: &EvaluationContext,
    work: &mut WorkBudget,
    source: &DomainExpression,
    variable: &Symbol,
) -> Result<(), AnalyticLimitError> {
    if source.construction_domain() != Some(ScalarDomain::Real) || source.has_external_history() {
        return Err(AnalyticLimitError::Unsupported);
    }
    ctx.check_expression(source.expression())?;
    if source.conditions().len() > ctx.limits().max_rational_conditions {
        return Err(EvaluationError::Limit(crate::ResourceLimitKind::RationalConditions).into());
    }
    let direct = definedness_obligations_in_domain(
        source.expression(),
        ctx.limits().max_expression_nodes,
        ScalarDomain::Real,
    )
    .map_err(|_| AnalyticLimitError::Unsupported)?;
    for condition in direct {
        work.charge(1)?;
        if !source.conditions().contains(&condition) {
            return Err(AnalyticLimitError::Unsupported);
        }
    }
    for condition in source.conditions() {
        ctx.check_expression(condition.expression())?;
        let p = polynomial(work, condition.expression(), variable)?;
        // A nonzero polynomial has isolated zeros. A polynomial strictly
        // positive at zero remains positive nearby by continuity.
        match condition {
            Condition::NonZero(_) if !p.is_zero() => {}
            Condition::Positive(_)
                if p.coefficients()
                    .first()
                    .is_some_and(|q| q.numerator() > &Integer::zero()) => {}
            _ => return Err(AnalyticLimitError::Unsupported),
        }
    }
    Ok(())
}

fn signed_function(expr: &Expr, sign: i64) -> Option<(&str, &Expr)> {
    let node = if sign == 1 {
        expr
    } else {
        let ExprView::Mul(parts) = expr.view() else {
            return None;
        };
        if parts.len() != 2 {
            return None;
        }
        let minus_one = Rational::from(Integer::from(-1));
        if exact_scalar(&parts[0]).as_ref() == Some(&minus_one) {
            &parts[1]
        } else if exact_scalar(&parts[1]).as_ref() == Some(&minus_one) {
            &parts[0]
        } else {
            return None;
        }
    };
    match node.view() {
        ExprView::Function { name, argument } => Some((name.name(), argument)),
        _ => None,
    }
}

fn standard_numerator(expr: &Expr) -> Option<(StandardLimitRule, &Expr)> {
    if let Some((name, argument)) = signed_function(expr, 1) {
        return match name {
            "sin" => Some((StandardLimitRule::Sine, argument)),
            "log" => Some((StandardLimitRule::NaturalLogarithm, argument)),
            _ => None,
        };
    }
    let ExprView::Add(parts) = expr.view() else {
        return None;
    };
    if parts.len() != 2 {
        return None;
    }
    let (constant, other) = if let Some(q) = exact_scalar(&parts[0]) {
        (q, &parts[1])
    } else {
        (exact_scalar(&parts[1])?, &parts[0])
    };
    if constant == one() {
        let ("cos", argument) = signed_function(other, -1)? else {
            return None;
        };
        Some((StandardLimitRule::OneMinusCosine, argument))
    } else if constant == -&one() {
        let ("exp", argument) = signed_function(other, 1)? else {
            return None;
        };
        Some((StandardLimitRule::ExponentialMinusOne, argument))
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AnalyticShape {
    rule: StandardLimitRule,
    argument_scale: Rational,
    outer_scale: Rational,
    denominator_order: usize,
}

fn analytic_shape(
    work: &mut WorkBudget,
    expr: &Expr,
    variable: &Symbol,
) -> Result<AnalyticShape, AnalyticLimitError> {
    let ExprView::Mul(parts) = expr.view() else {
        return Err(AnalyticLimitError::Unsupported);
    };
    let mut outer_scale = one();
    let mut numerator = None;
    let mut order = 0;
    for part in parts {
        work.charge(1)?;
        if let Some(q) = exact_scalar(part) {
            outer_scale = work.multiply_scalar(&outer_scale, &q)?;
        } else if let ExprView::Pow { base, exponent } = part.view() {
            let power = match exponent.view() {
                ExprView::Integer(n) if *n == Integer::from(-1) => 1,
                ExprView::Integer(n) if *n == Integer::from(-2) => 2,
                _ => return Err(AnalyticLimitError::Unsupported),
            };
            let p = polynomial(work, base, variable)?;
            let degree = p.degree().ok_or(AnalyticLimitError::Unsupported)?;
            if degree == 0
                || degree > 2
                || p.coefficients()[..degree]
                    .iter()
                    .any(|q| !q.numerator().is_zero())
                || order + degree * power > 2
            {
                return Err(AnalyticLimitError::Unsupported);
            }
            order += degree * power;
            for _ in 0..power {
                outer_scale = work.quotient(&outer_scale, &p.coefficients()[degree])?;
            }
        } else if numerator.replace(part).is_some() {
            return Err(AnalyticLimitError::Unsupported);
        }
    }
    let (rule, argument) = numerator
        .and_then(standard_numerator)
        .ok_or(AnalyticLimitError::Unsupported)?;
    let required_order = if rule == StandardLimitRule::OneMinusCosine {
        2
    } else {
        1
    };
    if order != required_order {
        return Err(AnalyticLimitError::Unsupported);
    }
    let argument = polynomial(work, argument, variable)?;
    let constant = argument.coefficients().first().cloned().unwrap_or_else(zero);
    let required_constant = if rule == StandardLimitRule::NaturalLogarithm {
        one()
    } else {
        zero()
    };
    if argument.degree().is_some_and(|degree| degree > 1) || constant != required_constant {
        return Err(AnalyticLimitError::Unsupported);
    }
    Ok(AnalyticShape {
        rule,
        argument_scale: argument.coefficients().get(1).cloned().unwrap_or_else(zero),
        outer_scale,
        denominator_order: order,
    })
}

fn standard_value(
    work: &mut WorkBudget,
    shape: &AnalyticShape,
) -> Result<LimitValue, EvaluationError> {
    // sin(t)/t, (exp(t)-1)/t and log(1+t)/t tend to 1;
    // (1-cos(t))/t² tends to 1/2. If a=0 the numerator is identically
    // zero on the punctured domain, so no cancellation by a is needed.
    let a = &shape.argument_scale;
    let normalized = if shape.rule == StandardLimitRule::OneMinusCosine {
        let square = work.multiply_scalar(a, a)?;
        work.quotient(&square, &Rational::from(Integer::from(2)))?
    } else {
        a.clone()
    };
    Ok(LimitValue::Finite(
        work.multiply_scalar(&shape.outer_scale, &normalized)?,
    ))
}

impl EvaluationContext {
    /// Real standard-limit patterns at zero; all three directions agree.
    /// Unsupported compositions and unresolved conditions are not limit values.
    pub fn analytic_limit_at_zero(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        direction: LimitDirection,
    ) -> Result<AnalyticZeroLimit, AnalyticLimitError> {
        let mut work = WorkBudget::new(self.limits());
        check_analytic_source(self, &mut work, source, variable)?;
        let shape = analytic_shape(&mut work, source.expression(), variable)?;
        let value = standard_value(&mut work, &shape)?;
        let result = AnalyticZeroLimit {
            source: source.clone(),
            variable: variable.clone(),
            direction,
            rule: shape.rule,
            argument_scale: shape.argument_scale,
            outer_scale: shape.outer_scale,
            denominator_order: shape.denominator_order,
            value,
        };
        result.verify(self, source, variable, direction)?;
        Ok(result)
    }
}

impl AnalyticZeroLimit {
    /// Independently check source binding, admitted structure, theorem
    /// prerequisites, direction and exact scaled result without calling the solver.
    pub fn verify(
        &self,
        ctx: &EvaluationContext,
        source: &DomainExpression,
        variable: &Symbol,
        direction: LimitDirection,
    ) -> Result<(), AnalyticLimitError> {
        let mut work = WorkBudget::new(ctx.limits());
        check_analytic_source(ctx, &mut work, source, variable)?;
        if self.source != *source || self.variable != *variable || self.direction != direction {
            return Err(AnalyticLimitError::InvalidCertificate);
        }
        let shape = analytic_shape(&mut work, source.expression(), variable)?;
        if self.rule != shape.rule
            || self.argument_scale != shape.argument_scale
            || self.outer_scale != shape.outer_scale
            || self.denominator_order != shape.denominator_order
            || self.value != standard_value(&mut work, &shape)?
        {
            return Err(AnalyticLimitError::InvalidCertificate);
        }
        Ok(())
    }
}

#[cfg(test)]
mod extended_limit_tests {
    use super::*;
    use crate::ResourceLimits;

    fn r(n: i64) -> Rational {
        Rational::from(Integer::from(n))
    }

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(values.iter().copied().map(r).collect())
    }

    fn bound(expr: Expr) -> DomainExpression {
        DomainExpression::from_expr_in_domain(expr, ScalarDomain::Real, 128, 32).unwrap()
    }

    fn x() -> Symbol {
        Symbol::new("x").unwrap()
    }

    fn quotient(numerator: Expr, denominator: Expr) -> DomainExpression {
        bound(numerator).divide(bound(denominator), 32).unwrap()
    }

    #[test]
    fn infinity_degree_sign_and_exact_ratio() {
        let ctx = EvaluationContext::default();
        let cases = [
            (p(&[1, 0, 3]), p(&[-1, 0, 2]), LimitValue::Finite(
                Rational::new(Integer::from(3), Integer::from(2)).unwrap(),
            ), LimitValue::Finite(
                Rational::new(Integer::from(3), Integer::from(2)).unwrap(),
            )),
            (p(&[0, 0, 0, 1]), p(&[1, 0, 1]), LimitValue::PositiveInfinity, LimitValue::NegativeInfinity),
            (p(&[0, 0, -1]), p(&[1]), LimitValue::NegativeInfinity, LimitValue::NegativeInfinity),
            (p(&[1]), p(&[1, 1]), LimitValue::Finite(r(0)), LimitValue::Finite(r(0))),
            (p(&[]), p(&[0, 1]), LimitValue::Finite(r(0)), LimitValue::Finite(r(0))),
        ];
        for (n, d, positive, negative) in cases {
            let source = ctx.rational_function(&n, &d).unwrap();
            for (end, expected) in [
                (InfinityEnd::Positive, positive),
                (InfinityEnd::Negative, negative),
            ] {
                let result = ctx.rational_infinity_limit(&source, end).unwrap();
                assert_eq!(result.value, expected);
                assert_eq!(result.verify(&ctx, &source, end), Ok(()));
                assert_eq!(result, ctx.rational_infinity_limit(&source, end).unwrap());
            }
        }
    }

    #[test]
    fn infinity_preserves_holes_and_rejects_mutations() {
        let ctx = EvaluationContext::default();
        let source = ctx.rational_function(&p(&[-1, 0, 1]), &p(&[-1, 1])).unwrap();
        let end = InfinityEnd::Negative;
        let good = ctx.rational_infinity_limit(&source, end).unwrap();
        assert_eq!(good.value, LimitValue::NegativeInfinity);
        assert_eq!(good.exclusions.len(), 1);
        assert_eq!(good.exclusions[0].tail_bound, r(2));
        let mut bad_sign = good.clone();
        bad_sign.value = LimitValue::PositiveInfinity;
        let mut bad_degree = good.clone();
        bad_degree.numerator.as_mut().unwrap().degree = 0;
        let mut bad_lead = good.clone();
        bad_lead.numerator.as_mut().unwrap().coefficient = r(-1);
        let mut bad_bound = good.clone();
        bad_bound.exclusions[0].tail_bound = r(0);
        let mut bad_domain = good.clone();
        bad_domain.exclusions.clear();
        for bad in [bad_sign, bad_degree, bad_lead, bad_bound, bad_domain] {
            assert_eq!(
                bad.verify(&ctx, &source, end),
                Err(LimitVerificationError::InvalidCertificate)
            );
        }
        assert_eq!(
            good.verify(&ctx, &source, InfinityEnd::Positive),
            Err(LimitVerificationError::InvalidCertificate)
        );
        let filled = RationalFunction::from_polynomial(p(&[1, 1]));
        assert_eq!(
            good.verify(&ctx, &filled, end),
            Err(LimitVerificationError::InvalidCertificate)
        );
        let restricted = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        assert!(matches!(
            good.verify(&restricted, &source, end),
            Err(LimitVerificationError::Evaluation(EvaluationError::Limit(_)))
        ));
    }

    #[test]
    fn standard_limits_scaling_and_directions() {
        let ctx = EvaluationContext::default();
        let variable = x();
        let x = Expr::from(variable.clone());
        let a = Expr::mul([Expr::from(3), x.clone()]);
        let cases = [
            (quotient(Expr::sin(a.clone()), x.clone()), r(3)),
            (quotient(
                Expr::add([Expr::from(1), Expr::mul([
                    Expr::from(-1),
                    Expr::cos(Expr::mul([Expr::from(2), x.clone()])),
                ])]),
                Expr::pow(x.clone(), Expr::from(2)),
            ), r(2)),
            (quotient(Expr::add([Expr::exp(a.clone()), Expr::from(-1)]), x.clone()), r(3)),
            (quotient(Expr::log(Expr::add([Expr::from(1), x.clone()])), x.clone()), r(1)),
            (quotient(Expr::sin(Expr::from(0)), x.clone()), r(0)),
            (quotient(Expr::sin(a), Expr::mul([Expr::from(-3), x.clone()])), r(-1)),
        ];
        for (source, expected) in cases {
            for direction in [LimitDirection::Left, LimitDirection::Right, LimitDirection::TwoSided] {
                let result = ctx.analytic_limit_at_zero(&source, &variable, direction).unwrap();
                assert_eq!(result.value, LimitValue::Finite(expected.clone()));
                assert_eq!(result.verify(&ctx, &source, &variable, direction), Ok(()));
                assert_eq!(result, ctx.analytic_limit_at_zero(&source, &variable, direction).unwrap());
            }
        }
    }

    #[test]
    fn analytic_refuses_unsupported_sources_and_forged_evidence() {
        let ctx = EvaluationContext::default();
        let variable = x();
        let x = Expr::from(variable.clone());
        let direction = LimitDirection::TwoSided;
        let source = quotient(Expr::sin(x.clone()), x.clone());
        let good = ctx.analytic_limit_at_zero(&source, &variable, direction).unwrap();
        let mut bad = good.clone();
        bad.argument_scale = r(2);
        assert_eq!(bad.verify(&ctx, &source, &variable, direction), Err(AnalyticLimitError::InvalidCertificate));
        bad = good.clone();
        bad.rule = StandardLimitRule::NaturalLogarithm;
        assert_eq!(bad.verify(&ctx, &source, &variable, direction), Err(AnalyticLimitError::InvalidCertificate));
        bad = good.clone();
        bad.value = LimitValue::Finite(r(2));
        assert_eq!(bad.verify(&ctx, &source, &variable, direction), Err(AnalyticLimitError::InvalidCertificate));
        assert_eq!(good.verify(&ctx, &source, &variable, LimitDirection::Left), Err(AnalyticLimitError::InvalidCertificate));

        let wrong_power = quotient(Expr::sin(x.clone()), Expr::pow(x.clone(), Expr::from(2)));
        let nested = quotient(Expr::sin(Expr::sin(x.clone())), x.clone());
        let offset = quotient(Expr::sin(Expr::add([x.clone(), Expr::from(1)])), x.clone());
        let false_cancel = source.rebuild_with_expression(source.expression().clone(), 128, 32).unwrap();
        let zero_poly = Expr::add([x.clone(), Expr::mul([Expr::from(-1), x.clone()])]);
        let hidden = bound(Expr::from(0)).divide(bound(zero_poly), 32).unwrap();
        let restricted = DomainExpression::add([false_cancel, hidden], 32).unwrap();
        for unsupported in [wrong_power, nested, offset, restricted] {
            assert!(ctx.analytic_limit_at_zero(&unsupported, &variable, direction).is_err());
        }

        let hidden = bound(Expr::from(0)).divide(bound(Expr::add([x.clone(), Expr::from(-1)])), 32).unwrap();
        let hole = DomainExpression::add([source.clone(), hidden], 32).unwrap();
        let result = ctx.analytic_limit_at_zero(&hole, &variable, direction).unwrap();
        assert_eq!(result.value, LimitValue::Finite(r(1)));
        assert_eq!(result.source.conditions(), hole.conditions());
        assert_eq!(good.verify(&ctx, &hole, &variable, direction), Err(AnalyticLimitError::InvalidCertificate));

        let restricted = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        assert!(matches!(
            good.verify(&restricted, &source, &variable, direction),
            Err(AnalyticLimitError::Evaluation(EvaluationError::Limit(_)))
        ));
    }
}
