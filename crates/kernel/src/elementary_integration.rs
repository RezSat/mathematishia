//! Checked antiderivatives of rational multiples of sin, cos and exp
//! with a nonzero rational-affine argument in one selected variable.
//!
//! The fixed integration constant is zero. Domain conditions are retained
//! prerequisites, never discharged or assumed true.

use std::error::Error;
use std::fmt;

use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ExprView, Integer, Rational, ScalarDomain,
    Symbol, definedness_obligations_in_domain,
};

use crate::budget::WorkBudget;
use crate::{
    DerivativeVerificationError, EvaluationContext, EvaluationError, PolynomialConversionError,
    RationalPolynomial, ResourceLimitKind, SymbolicDerivative, verify_symbolic_derivative,
};

/// Exact inverse-chain-rule identities in the supported domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElementaryPrimitiveRule {
    Sine,
    Cosine,
    Exponential,
}

impl ElementaryPrimitiveRule {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Sine => "calculus.integrate_affine_sine",
            Self::Cosine => "calculus.integrate_affine_cosine",
            Self::Exponential => "calculus.integrate_affine_exponential",
        }
    }
}

/// Typed rejection or independent-verification failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementaryIntegrationError {
    Evaluation(EvaluationError),
    Definedness(DefinednessError),
    DerivativeVerification(DerivativeVerificationError),
    UnsupportedScalarDomain,
    UnsupportedForm,
    UnsupportedFunction,
    UnsupportedArgument(PolynomialConversionError),
    NonAffineArgument,
    ZeroSlope,
    MissingDomainCondition,
    /// The current derivative normalization cannot establish the identity.
    UnsupportedNormalization,
    SourceMismatch,
    IncorrectRule,
    IncorrectAffineEvidence,
    IncorrectPrimitive,
    IncorrectConditions,
    IncorrectDerivative,
    IncorrectDerivativeEvidence,
}

impl From<EvaluationError> for ElementaryIntegrationError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

impl From<DefinednessError> for ElementaryIntegrationError {
    fn from(value: DefinednessError) -> Self {
        Self::Definedness(value)
    }
}

impl From<DerivativeVerificationError> for ElementaryIntegrationError {
    fn from(value: DerivativeVerificationError) -> Self {
        Self::DerivativeVerification(value)
    }
}

impl fmt::Display for ElementaryIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Definedness(error) => write!(f, "{error}"),
            Self::DerivativeVerification(error) => write!(f, "{error}"),
            Self::UnsupportedScalarDomain => {
                f.write_str("integration requires a real or complex scalar domain")
            }
            Self::UnsupportedForm => {
                f.write_str("integrand is not a single rational multiple of one supported function")
            }
            Self::UnsupportedFunction => f.write_str("only sin, cos and exp are supported"),
            Self::UnsupportedArgument(reason) => {
                write!(f, "unsupported analytic argument: {reason}")
            }
            Self::NonAffineArgument => {
                f.write_str("analytic argument is not affine in the selected variable")
            }
            Self::ZeroSlope => f.write_str("affine argument has zero slope"),
            Self::MissingDomainCondition => f.write_str("source lacks a required domain condition"),
            Self::UnsupportedNormalization => {
                f.write_str("exact derivative cannot be normalized to the integrand")
            }
            Self::SourceMismatch => f.write_str(
                "primitive claim does not match its supplied source, variable or domain",
            ),
            Self::IncorrectRule => f.write_str("primitive rule does not match the integrand"),
            Self::IncorrectAffineEvidence => {
                f.write_str("recorded rational affine data are incorrect")
            }
            Self::IncorrectPrimitive => {
                f.write_str("primitive differs from the zero-constant rule candidate")
            }
            Self::IncorrectConditions => {
                f.write_str("primitive did not retain the source conditions")
            }
            Self::IncorrectDerivative => {
                f.write_str("verified derivative differs from the integrand")
            }
            Self::IncorrectDerivativeEvidence => {
                f.write_str("derivative evidence does not belong to this primitive")
            }
        }
    }
}

impl Error for ElementaryIntegrationError {}

/// Canonical primitive with source-domain obligations and verified derivative steps.
///
/// "Zero constant" means no additive integration constant is inserted; the
/// primitive is not required to have value zero at the origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementaryAntiderivative {
    source: DomainExpression,
    variable: Symbol,
    domain: ScalarDomain,
    primitive: DomainExpression,
    rule: ElementaryPrimitiveRule,
    input_scale: Rational,
    slope: Rational,
    intercept: Rational,
    derivative_evidence: SymbolicDerivative,
}

impl ElementaryAntiderivative {
    pub fn source(&self) -> &DomainExpression {
        &self.source
    }

    pub fn variable(&self) -> &Symbol {
        &self.variable
    }

    pub const fn domain(&self) -> ScalarDomain {
        self.domain
    }

    pub fn primitive(&self) -> &DomainExpression {
        &self.primitive
    }

    pub fn conditions(&self) -> &[Condition] {
        self.primitive.conditions()
    }

    pub const fn rule(&self) -> ElementaryPrimitiveRule {
        self.rule
    }

    pub fn input_scale(&self) -> &Rational {
        &self.input_scale
    }

    pub fn slope(&self) -> &Rational {
        &self.slope
    }

    pub fn intercept(&self) -> &Rational {
        &self.intercept
    }

    pub fn derivative_evidence(&self) -> &SymbolicDerivative {
        &self.derivative_evidence
    }
}

fn scalar_atom(expr: &Expr) -> Option<Rational> {
    match expr.view() {
        ExprView::Integer(value) => Some(Rational::from(value.clone())),
        ExprView::Rational(value) => Some(value.clone()),
        _ => None,
    }
}

/// Only one canonical multiplicative term; no sum, product of functions,
/// general constant expression or implicit identity rewrite is admitted.
fn classify(
    expr: &Expr,
) -> Result<(ElementaryPrimitiveRule, Rational, &Expr), ElementaryIntegrationError> {
    let (scale, function) = match expr.view() {
        ExprView::Function { .. } => (Rational::from(Integer::one()), expr),
        ExprView::Mul(factors) if factors.len() == 2 => {
            let scale =
                scalar_atom(&factors[0]).ok_or(ElementaryIntegrationError::UnsupportedForm)?;
            (scale, &factors[1])
        }
        _ => return Err(ElementaryIntegrationError::UnsupportedForm),
    };
    let ExprView::Function { name, argument } = function.view() else {
        return Err(ElementaryIntegrationError::UnsupportedForm);
    };
    let rule = match name.name() {
        "sin" => ElementaryPrimitiveRule::Sine,
        "cos" => ElementaryPrimitiveRule::Cosine,
        "exp" => ElementaryPrimitiveRule::Exponential,
        _ => return Err(ElementaryIntegrationError::UnsupportedFunction),
    };
    Ok((rule, scale, argument))
}

fn map_conversion(error: EvaluationError) -> ElementaryIntegrationError {
    match error {
        EvaluationError::Polynomial(PolynomialConversionError::DegreeLimitExceeded) => {
            ElementaryIntegrationError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::PolynomialDegree,
            ))
        }
        EvaluationError::Polynomial(error) => {
            ElementaryIntegrationError::UnsupportedArgument(error)
        }
        other => ElementaryIntegrationError::Evaluation(other),
    }
}

fn check_source(
    context: &EvaluationContext,
    source: &DomainExpression,
    domain: ScalarDomain,
    meter: &mut WorkBudget,
) -> Result<(), ElementaryIntegrationError> {
    if !matches!(domain, ScalarDomain::Real | ScalarDomain::Complex) {
        return Err(ElementaryIntegrationError::UnsupportedScalarDomain);
    }
    source.check_construction_domain(domain)?;
    context.check_expression(source.expression())?;
    if source.conditions().len() > context.limits().max_rational_conditions {
        return Err(EvaluationError::Limit(ResourceLimitKind::RationalConditions).into());
    }
    for condition in source.conditions() {
        meter.charge(1)?;
        context.check_expression(condition.expression())?;
    }
    let necessary = definedness_obligations_in_domain(
        source.expression(),
        context.limits().max_expression_nodes,
        domain,
    )?;
    if necessary
        .iter()
        .any(|condition| !source.conditions().contains(condition))
    {
        return Err(ElementaryIntegrationError::MissingDomainCondition);
    }
    Ok(())
}

fn affine_coefficients(
    context: &EvaluationContext,
    argument: &Expr,
    variable: &Symbol,
    meter: &mut WorkBudget,
) -> Result<(Rational, Rational), ElementaryIntegrationError> {
    let polynomial = meter.convert(argument, variable).map_err(map_conversion)?;
    context.check_polynomial(&polynomial)?;
    if polynomial.degree().is_some_and(|degree| degree > 1) {
        return Err(ElementaryIntegrationError::NonAffineArgument);
    }
    let zero = Rational::from(Integer::zero());
    let offset = polynomial
        .coefficients()
        .first()
        .cloned()
        .unwrap_or_else(|| zero.clone());
    let slope = polynomial.coefficients().get(1).cloned().unwrap_or(zero);
    if slope.numerator().is_zero() {
        return Err(ElementaryIntegrationError::ZeroSlope);
    }
    Ok((slope, offset))
}

/// Rebuild a primitive through domain-preserving composition, including
/// restrictions inherited before constructor normalization.
fn rule_candidate(
    context: &EvaluationContext,
    source: &DomainExpression,
    domain: ScalarDomain,
    rule: ElementaryPrimitiveRule,
    ratio: (&Rational, &Rational),
    argument: &Expr,
    meter: &mut WorkBudget,
) -> Result<DomainExpression, ElementaryIntegrationError> {
    let mut factor = meter.quotient(ratio.0, ratio.1)?;
    if rule == ElementaryPrimitiveRule::Sine {
        factor = -factor;
    }
    context.check_rational(&factor)?;
    let analytic = match rule {
        ElementaryPrimitiveRule::Sine => Expr::cos(argument.clone()),
        ElementaryPrimitiveRule::Cosine => Expr::sin(argument.clone()),
        ElementaryPrimitiveRule::Exponential => Expr::exp(argument.clone()),
    };
    let limits = context.limits();
    let coefficient = DomainExpression::with_conditions(
        Expr::from(factor),
        source.conditions().iter().cloned(),
        limits.max_expression_nodes,
        limits.max_rational_conditions,
    )?;
    let function = DomainExpression::from_expr_in_domain(
        analytic,
        domain,
        limits.max_expression_nodes,
        limits.max_rational_conditions,
    )?;
    let candidate =
        DomainExpression::multiply([coefficient, function], limits.max_rational_conditions)?;
    context.check_expression(candidate.expression())?;
    if candidate.conditions() != source.conditions() {
        return Err(ElementaryIntegrationError::IncorrectConditions);
    }
    Ok(candidate)
}

fn validate_claim(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    claim: &ElementaryAntiderivative,
) -> Result<(), ElementaryIntegrationError> {
    if claim.source != *source || &claim.variable != variable || claim.domain != domain {
        return Err(ElementaryIntegrationError::SourceMismatch);
    }
    let mut meter = WorkBudget::new(context.limits());
    check_source(context, source, domain, &mut meter)?;
    let (rule, scale, argument) = classify(source.expression())?;
    if claim.rule != rule {
        return Err(ElementaryIntegrationError::IncorrectRule);
    }
    context.check_rational(&scale)?;
    let (slope, intercept) = affine_coefficients(context, argument, variable, &mut meter)?;
    if claim.input_scale != scale || claim.slope != slope || claim.intercept != intercept {
        return Err(ElementaryIntegrationError::IncorrectAffineEvidence);
    }
    let expected = rule_candidate(
        context,
        source,
        domain,
        rule,
        (&scale, &slope),
        argument,
        &mut meter,
    )?;
    context.check_expression(claim.primitive.expression())?;
    if claim.primitive.conditions() != source.conditions()
        || claim.primitive.conditions() != expected.conditions()
    {
        return Err(ElementaryIntegrationError::IncorrectConditions);
    }
    if claim.primitive.expression() != expected.expression() {
        return Err(ElementaryIntegrationError::IncorrectPrimitive);
    }

    let proof = &claim.derivative_evidence;
    if proof.original() != claim.primitive.expression()
        || proof.variable() != variable
        || proof.domain() != Some(domain)
        || proof.conditions() != claim.primitive.conditions()
    {
        return Err(ElementaryIntegrationError::IncorrectDerivativeEvidence);
    }
    if proof.steps().len() > context.limits().max_expression_nodes {
        return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes).into());
    }
    meter.charge(proof.steps().len() as u64)?;
    context.check_expression(proof.derivative())?;
    // Checks the complete postorder proof against the underlying differentiation
    // rules, without invoking its derivative generator.
    verify_symbolic_derivative(proof)?;
    if proof.derivative() != source.expression() {
        return Err(ElementaryIntegrationError::IncorrectDerivative);
    }
    Ok(())
}

impl EvaluationContext {
    /// Integrates exactly one rational multiple of sin, cos or exp of a
    /// rational-affine, nonconstant argument. No additive integration constant
    /// is represented. Conditions remain unproved and are copied unchanged.
    ///
    /// For a structurally noncanonical affine expression, an independent
    /// derivative may be mathematically equal but not structurally normalized
    /// to the source; in that case this method reports UnsupportedNormalization.
    pub fn integrate_elementary_affine(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
    ) -> Result<ElementaryAntiderivative, ElementaryIntegrationError> {
        let mut meter = WorkBudget::new(self.limits());
        check_source(self, source, domain, &mut meter)?;
        let (rule, scale, argument) = classify(source.expression())?;
        self.check_rational(&scale)?;
        let (slope, intercept) = affine_coefficients(self, argument, variable, &mut meter)?;
        let primitive = rule_candidate(
            self,
            source,
            domain,
            rule,
            (&scale, &slope),
            argument,
            &mut meter,
        )?;
        let derivative_evidence =
            self.differentiate_domain_expression(&primitive, variable, domain)?;
        if derivative_evidence.derivative() != source.expression() {
            return Err(ElementaryIntegrationError::UnsupportedNormalization);
        }
        let result = ElementaryAntiderivative {
            source: source.clone(),
            variable: variable.clone(),
            domain,
            primitive,
            rule,
            input_scale: scale,
            slope,
            intercept,
            derivative_evidence,
        };
        validate_claim(self, source, variable, domain, &result)?;
        Ok(result)
    }

    /// Independently verifies an antiderivative against the separately
    /// supplied source, selected variable and scalar domain.
    pub fn verify_elementary_antiderivative(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        claim: &ElementaryAntiderivative,
    ) -> Result<(), ElementaryIntegrationError> {
        validate_claim(self, source, variable, domain, claim)
    }
}


/// The maximum admitted degree of the rational polynomial multiplier.
const MAX_EXPONENTIAL_POLYNOMIAL_DEGREE: usize = 16;

/// A checked primitive Q(x) exp(ax+b) for P(x) exp(ax+b).
///
/// The certificate retains the source domain and verifies the product and
/// chain rule identity Q' + aQ = P by exact polynomial arithmetic. It does
/// not assert that inherited domain obligations hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialExponentialPrimitive {
    source: DomainExpression,
    variable: Symbol,
    domain: ScalarDomain,
    primitive: DomainExpression,
    input_polynomial: RationalPolynomial,
    primitive_polynomial: RationalPolynomial,
    slope: Rational,
    intercept: Rational,
}

impl PolynomialExponentialPrimitive {
    pub fn source(&self) -> &DomainExpression {
        &self.source
    }

    pub fn variable(&self) -> &Symbol {
        &self.variable
    }

    pub const fn domain(&self) -> ScalarDomain {
        self.domain
    }

    pub fn primitive(&self) -> &DomainExpression {
        &self.primitive
    }

    pub fn conditions(&self) -> &[Condition] {
        self.primitive.conditions()
    }

    pub fn input_polynomial(&self) -> &RationalPolynomial {
        &self.input_polynomial
    }

    pub fn primitive_polynomial(&self) -> &RationalPolynomial {
        &self.primitive_polynomial
    }

    pub fn slope(&self) -> &Rational {
        &self.slope
    }

    pub fn intercept(&self) -> &Rational {
        &self.intercept
    }
}

struct ExponentialInput<'a> {
    polynomial: RationalPolynomial,
    argument: &'a Expr,
    slope: Rational,
    intercept: Rational,
}

fn exponential_budget(context: &EvaluationContext) -> WorkBudget {
    let mut limits = context.limits();
    limits.max_polynomial_degree = limits
        .max_polynomial_degree
        .min(MAX_EXPONENTIAL_POLYNOMIAL_DEGREE);
    WorkBudget::new(limits)
}

fn exponential_input<'a>(
    context: &EvaluationContext,
    source: &'a DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    meter: &mut WorkBudget,
) -> Result<ExponentialInput<'a>, ElementaryIntegrationError> {
    check_source(context, source, domain, meter)?;
    if source.construction_domain() != Some(domain) {
        return Err(ElementaryIntegrationError::SourceMismatch);
    }
    let factors = match source.expression().view() {
        ExprView::Mul(factors) => factors,
        _ => std::slice::from_ref(source.expression()),
    };
    let mut polynomial = RationalPolynomial::new(vec![Rational::from(Integer::one())]);
    let mut exponential = None;
    for factor in factors {
        meter.charge(1)?;
        match factor.view() {
            ExprView::Function { name, argument } if name.name() == "exp" => {
                if exponential.replace(argument).is_some() {
                    return Err(ElementaryIntegrationError::UnsupportedForm);
                }
            }
            _ => {
                let term = meter.convert(factor, variable).map_err(map_conversion)?;
                polynomial = meter.multiply(&polynomial, &term)?;
            }
        }
    }
    let argument = exponential.ok_or(ElementaryIntegrationError::UnsupportedForm)?;
    context.check_polynomial(&polynomial)?;
    let (slope, intercept) = affine_coefficients(context, argument, variable, meter)?;
    Ok(ExponentialInput {
        polynomial,
        argument,
        slope,
        intercept,
    })
}

fn exponential_candidate(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    argument: &Expr,
    polynomial: &RationalPolynomial,
) -> Result<DomainExpression, ElementaryIntegrationError> {
    context.check_polynomial(polynomial)?;
    let expression = Expr::mul([polynomial.to_expr(variable), Expr::exp(argument.clone())]);
    context.check_expression(&expression)?;
    let limits = context.limits();
    let result = source.rebuild_with_expression(
        expression,
        limits.max_expression_nodes,
        limits.max_rational_conditions,
    )?;
    if result.conditions() != source.conditions() {
        return Err(ElementaryIntegrationError::IncorrectConditions);
    }
    Ok(result)
}

impl EvaluationContext {
    /// Integrates one bounded Q[x] multiplier times exp(ax+b), where a and b
    /// are rational and a is nonzero. Only explicitly bound real or complex
    /// sources are admitted. Degree is limited to 16 (or the smaller context
    /// limit); work and scalar sizes also obey the context limits.
    ///
    /// No additive constant is inserted. This is not a general integration
    /// algorithm, nor does it evaluate analytic definite integrals.
    pub fn integrate_polynomial_exponential(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
    ) -> Result<PolynomialExponentialPrimitive, ElementaryIntegrationError> {
        let mut meter = exponential_budget(self);
        let input = exponential_input(self, source, variable, domain, &mut meter)?;
        let coefficients = input.polynomial.coefficients();
        let zero = Rational::from(Integer::zero());
        let mut result = vec![zero.clone(); coefficients.len()];
        let mut next = zero;
        for (power, coefficient) in coefficients.iter().enumerate().rev() {
            let derivative = meter.multiply_scalar(
                &next,
                &Rational::from(Integer::from(power + 1)),
            )?;
            let residual = meter.subtract_scalar(coefficient, &derivative)?;
            next = meter.quotient(&residual, &input.slope)?;
            self.check_rational(&next)?;
            result[power] = next.clone();
        }
        let primitive_polynomial = RationalPolynomial::new(result);
        let primitive = exponential_candidate(
            self,
            source,
            variable,
            input.argument,
            &primitive_polynomial,
        )?;
        let claim = PolynomialExponentialPrimitive {
            source: source.clone(),
            variable: variable.clone(),
            domain,
            primitive,
            input_polynomial: input.polynomial,
            primitive_polynomial,
            slope: input.slope,
            intercept: input.intercept,
        };
        self.verify_polynomial_exponential_primitive(source, variable, domain, &claim)?;
        Ok(claim)
    }

    /// Checks the source factorization, affine argument, exact polynomial
    /// derivative identity and primitive expression without rerunning the
    /// coefficient recurrence used by the generator.
    pub fn verify_polynomial_exponential_primitive(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        claim: &PolynomialExponentialPrimitive,
    ) -> Result<(), ElementaryIntegrationError> {
        if claim.source != *source || &claim.variable != variable || claim.domain != domain {
            return Err(ElementaryIntegrationError::SourceMismatch);
        }
        let mut meter = exponential_budget(self);
        let input = exponential_input(self, source, variable, domain, &mut meter)?;
        if claim.input_polynomial != input.polynomial
            || claim.slope != input.slope
            || claim.intercept != input.intercept
        {
            return Err(ElementaryIntegrationError::IncorrectAffineEvidence);
        }
        let polynomial = &claim.primitive_polynomial;
        if polynomial
            .degree()
            .is_some_and(|degree| degree > MAX_EXPONENTIAL_POLYNOMIAL_DEGREE)
        {
            return Err(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree).into());
        }
        self.check_polynomial(polynomial)?;
        let derivative = meter.differentiate(polynomial)?;
        let slope = RationalPolynomial::new(vec![input.slope]);
        let scaled = meter.multiply(&slope, polynomial)?;
        if meter.add(&derivative, &scaled)? != input.polynomial {
            return Err(ElementaryIntegrationError::IncorrectDerivative);
        }
        let expected = exponential_candidate(self, source, variable, input.argument, polynomial)?;
        self.check_expression(claim.primitive.expression())?;
        if claim.primitive.conditions() != source.conditions()
            || claim.primitive.construction_domain() != Some(domain)
        {
            return Err(ElementaryIntegrationError::IncorrectConditions);
        }
        if claim.primitive != expected {
            return Err(ElementaryIntegrationError::IncorrectPrimitive);
        }
        Ok(())
    }
}

#[cfg(test)]
mod exponential_tests {
    use super::*;
    use crate::ResourceLimits;

    fn x() -> Expr {
        Expr::from(variable())
    }

    fn variable() -> Symbol {
        Symbol::new("x").unwrap()
    }

    fn q(n: i64, d: i64) -> Rational {
        Rational::new(Integer::from(n), Integer::from(d)).unwrap()
    }

    fn source(expression: Expr, domain: ScalarDomain) -> DomainExpression {
        DomainExpression::from_expr_in_domain(expression, domain, 256, 32).unwrap()
    }

    fn affine(a: Rational, b: Rational) -> Expr {
        Expr::add([Expr::mul([Expr::from(a), x()]), Expr::from(b)])
    }

    fn product(polynomial: Expr, argument: Expr) -> Expr {
        Expr::mul([polynomial, Expr::exp(argument)])
    }

    fn checked(
        expression: Expr,
        domain: ScalarDomain,
        coefficients: &[(i64, i64)],
    ) -> PolynomialExponentialPrimitive {
        let context = EvaluationContext::default();
        let input = source(expression, domain);
        let result = context
            .integrate_polynomial_exponential(&input, &variable(), domain)
            .unwrap();
        let expected = RationalPolynomial::new(
            coefficients.iter().map(|&(n, d)| q(n, d)).collect(),
        );
        assert_eq!(result.primitive_polynomial(), &expected);
        assert_eq!(result.source(), &input);
        assert_eq!(result.variable(), &variable());
        assert_eq!(result.domain(), domain);
        assert_eq!(result.conditions(), input.conditions());
        context
            .verify_polynomial_exponential_primitive(&input, &variable(), domain, &result)
            .unwrap();
        result
    }

    #[test]
    fn integrates_x_exp_x() {
        let result = checked(
            product(x(), x()),
            ScalarDomain::Real,
            &[(-1, 1), (1, 1)],
        );
        assert_eq!(
            result.primitive().expression(),
            &product(Expr::add([x(), Expr::from(-1)]), x())
        );
    }

    #[test]
    fn integrates_x_squared_exp_two_x() {
        checked(
            product(Expr::pow(x(), Expr::from(2)), affine(q(2, 1), q(0, 1))),
            ScalarDomain::Real,
            &[(1, 4), (-1, 2), (1, 2)],
        );
    }

    #[test]
    fn integrates_negative_fractional_slope_with_offset() {
        let result = checked(
            product(x(), affine(q(-1, 2), q(3, 2))),
            ScalarDomain::Complex,
            &[(-4, 1), (-2, 1)],
        );
        assert_eq!(result.slope(), &q(-1, 2));
        assert_eq!(result.intercept(), &q(3, 2));
    }

    #[test]
    fn integrates_factored_polynomial_without_expanding_analytic_terms() {
        let polynomial = Expr::mul([
            Expr::add([x(), Expr::from(1)]),
            Expr::add([x(), Expr::from(-1)]),
        ]);
        let result = checked(
            product(polynomial, x()),
            ScalarDomain::Real,
            &[(1, 1), (-2, 1), (1, 1)],
        );
        assert_eq!(
            result.input_polynomial(),
            &RationalPolynomial::new(vec![q(-1, 1), q(0, 1), q(1, 1)])
        );
    }

    #[test]
    fn integrates_rational_coefficients_and_constant_multiplier() {
        checked(
            product(Expr::mul([Expr::from(q(2, 3)), x()]), x()),
            ScalarDomain::Real,
            &[(-2, 3), (2, 3)],
        );
        checked(
            Expr::exp(affine(q(3, 1), q(2, 1))),
            ScalarDomain::Real,
            &[(1, 3)],
        );
    }

    #[test]
    fn accepts_degree_boundary_deterministically() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        let input = source(product(Expr::pow(x(), Expr::from(16)), x()), domain);
        let first = context
            .integrate_polynomial_exponential(&input, &variable(), domain)
            .unwrap();
        let second = context
            .integrate_polynomial_exponential(&input, &variable(), domain)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.primitive_polynomial().degree(), Some(16));
        assert_eq!(first.primitive_polynomial().coefficients()[16], q(1, 1));
    }

    #[test]
    fn preserves_historical_hole_and_rejects_erased_conditions() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        let hole = DomainExpression::multiply(
            [
                source(Expr::from(0), domain),
                source(Expr::pow(x(), Expr::from(-1)), domain),
            ],
            32,
        )
        .unwrap();
        let input = DomainExpression::add([hole, source(product(x(), x()), domain)], 32).unwrap();
        let mut result = context
            .integrate_polynomial_exponential(&input, &variable(), domain)
            .unwrap();
        assert_eq!(result.conditions(), &[Condition::NonZero(x())]);
        assert_eq!(result.primitive().construction_domain(), Some(domain));
        result.primitive = source(result.primitive().expression().clone(), domain);
        assert_eq!(
            context.verify_polynomial_exponential_primitive(&input, &variable(), domain, &result),
            Err(ElementaryIntegrationError::IncorrectConditions)
        );
    }

    #[test]
    fn rejects_nonlinear_and_zero_slope_arguments() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        for (argument, error) in [
            (
                Expr::pow(x(), Expr::from(2)),
                ElementaryIntegrationError::NonAffineArgument,
            ),
            (Expr::from(3), ElementaryIntegrationError::ZeroSlope),
        ] {
            let input = source(product(x(), argument), domain);
            assert_eq!(
                context.integrate_polynomial_exponential(&input, &variable(), domain),
                Err(error)
            );
        }
    }

    #[test]
    fn rejects_other_symbols_mixed_functions_and_reciprocals() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        for expression in [
            product(Expr::from(Symbol::new("y").unwrap()), x()),
            product(Expr::sin(x()), x()),
            product(Expr::exp(Expr::add([x(), Expr::from(1)])), x()),
            product(Expr::pow(x(), Expr::from(-1)), x()),
            Expr::add([x(), Expr::exp(x())]),
        ] {
            let input = source(expression, domain);
            assert!(
                context
                    .integrate_polynomial_exponential(&input, &variable(), domain)
                    .is_err()
            );
        }
    }

    #[test]
    fn rejects_unbound_and_reinterpreted_sources() {
        let context = EvaluationContext::default();
        let unbound = DomainExpression::from_expr(x(), 256).unwrap();
        assert_eq!(
            context.integrate_polynomial_exponential(&unbound, &variable(), ScalarDomain::Real),
            Err(ElementaryIntegrationError::SourceMismatch)
        );
        let real = source(product(x(), x()), ScalarDomain::Real);
        assert!(
            context
                .integrate_polynomial_exponential(&real, &variable(), ScalarDomain::Complex)
                .is_err()
        );
        assert_eq!(
            context.integrate_polynomial_exponential(&real, &variable(), ScalarDomain::Rational),
            Err(ElementaryIntegrationError::UnsupportedScalarDomain)
        );
    }

    #[test]
    fn rejects_degree_and_work_limits() {
        let domain = ScalarDomain::Real;
        let input = source(product(Expr::pow(x(), Expr::from(17)), x()), domain);
        assert_eq!(
            EvaluationContext::default().integrate_polynomial_exponential(
                &input,
                &variable(),
                domain
            ),
            Err(ElementaryIntegrationError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::PolynomialDegree
            )))
        );
        let work = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        let input = source(product(x(), x()), domain);
        assert_eq!(
            work.integrate_polynomial_exponential(&input, &variable(), domain),
            Err(ElementaryIntegrationError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::WorkUnits
            )))
        );
    }

    #[test]
    fn rejects_forged_coefficient_identity_even_with_matching_expression() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        let input = source(product(x(), x()), domain);
        let mut claim = context
            .integrate_polynomial_exponential(&input, &variable(), domain)
            .unwrap();
        claim.primitive_polynomial = RationalPolynomial::new(vec![q(0, 1), q(1, 1)]);
        claim.primitive = source(product(x(), x()), domain);
        assert_eq!(
            context.verify_polynomial_exponential_primitive(&input, &variable(), domain, &claim),
            Err(ElementaryIntegrationError::IncorrectDerivative)
        );
    }

    #[test]
    fn rejects_forged_affine_data_primitive_and_source() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        let input = source(product(x(), x()), domain);
        let claim = context
            .integrate_polynomial_exponential(&input, &variable(), domain)
            .unwrap();
        let mut forged = claim.clone();
        forged.slope = q(2, 1);
        assert_eq!(
            context.verify_polynomial_exponential_primitive(&input, &variable(), domain, &forged),
            Err(ElementaryIntegrationError::IncorrectAffineEvidence)
        );
        let mut forged = claim.clone();
        forged.primitive = source(Expr::exp(x()), domain);
        assert_eq!(
            context.verify_polynomial_exponential_primitive(&input, &variable(), domain, &forged),
            Err(ElementaryIntegrationError::IncorrectPrimitive)
        );
        let other = source(product(x(), affine(q(1, 1), q(2, 1))), domain);
        assert_eq!(
            context.verify_polynomial_exponential_primitive(&other, &variable(), domain, &claim),
            Err(ElementaryIntegrationError::SourceMismatch)
        );
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    fn q(n: i64) -> Rational {
        Rational::from(Integer::from(n))
    }

    fn sample() -> (
        EvaluationContext,
        DomainExpression,
        Symbol,
        ElementaryAntiderivative,
    ) {
        let context = EvaluationContext::default();
        let x = Symbol::new("x").unwrap();
        let original = DomainExpression::from_expr_in_domain(
            Expr::sin(Expr::add([Expr::from(x.clone()), Expr::from(3)])),
            ScalarDomain::Real,
            128,
            32,
        )
        .unwrap();
        let result = context
            .integrate_elementary_affine(&original, &x, ScalarDomain::Real)
            .unwrap();
        (context, original, x, result)
    }

    #[test]
    fn rejects_modified_formula_rule_coefficients_and_source() {
        let (ctx, original, variable, result) = sample();
        let mut forged = result.clone();
        forged.primitive = DomainExpression::from_expr_in_domain(
            Expr::sin(Expr::from(variable.clone())),
            ScalarDomain::Real,
            128,
            32,
        )
        .unwrap();
        assert_eq!(
            ctx.verify_elementary_antiderivative(&original, &variable, ScalarDomain::Real, &forged),
            Err(ElementaryIntegrationError::IncorrectPrimitive)
        );
        let mut forged = result.clone();
        forged.rule = ElementaryPrimitiveRule::Exponential;
        assert_eq!(
            ctx.verify_elementary_antiderivative(&original, &variable, ScalarDomain::Real, &forged),
            Err(ElementaryIntegrationError::IncorrectRule)
        );
        let mut forged = result.clone();
        forged.slope = q(3);
        assert_eq!(
            ctx.verify_elementary_antiderivative(&original, &variable, ScalarDomain::Real, &forged),
            Err(ElementaryIntegrationError::IncorrectAffineEvidence)
        );
        let mut forged = result.clone();
        forged.source = DomainExpression::from_expr_in_domain(
            Expr::cos(Expr::from(variable.clone())),
            ScalarDomain::Real,
            128,
            32,
        )
        .unwrap();
        assert_eq!(
            ctx.verify_elementary_antiderivative(&original, &variable, ScalarDomain::Real, &forged),
            Err(ElementaryIntegrationError::SourceMismatch)
        );
    }

    #[test]
    fn rejects_forged_proof_and_condition_erasure() {
        let (ctx, source, variable, result) = sample();
        let mut forged = result.clone();
        forged.derivative_evidence = ctx
            .differentiate_domain_expression(
                &DomainExpression::from_expr_in_domain(
                    Expr::cos(Expr::from(variable.clone())),
                    ScalarDomain::Real,
                    128,
                    32,
                )
                .unwrap(),
                &variable,
                ScalarDomain::Real,
            )
            .unwrap();
        assert_eq!(
            ctx.verify_elementary_antiderivative(&source, &variable, ScalarDomain::Real, &forged),
            Err(ElementaryIntegrationError::IncorrectDerivativeEvidence)
        );

        let historical = DomainExpression::with_conditions(
            Expr::from(1),
            [Condition::NonZero(Expr::from(variable.clone()))],
            128,
            32,
        )
        .unwrap();
        let conditional_source = DomainExpression::multiply(
            [
                historical,
                DomainExpression::from_expr_in_domain(
                    source.expression().clone(),
                    ScalarDomain::Real,
                    128,
                    32,
                )
                .unwrap(),
            ],
            32,
        )
        .unwrap();
        let mut certified = ctx
            .integrate_elementary_affine(&conditional_source, &variable, ScalarDomain::Real)
            .unwrap();
        certified.primitive = DomainExpression::from_expr_in_domain(
            certified.primitive.expression().clone(),
            ScalarDomain::Real,
            128,
            32,
        )
        .unwrap();
        assert_eq!(
            ctx.verify_elementary_antiderivative(
                &conditional_source,
                &variable,
                ScalarDomain::Real,
                &certified
            ),
            Err(ElementaryIntegrationError::IncorrectConditions)
        );
    }

    #[test]
    fn classification_refuses_unknown_function_names() {
        let x = Expr::from(Symbol::new("x").unwrap());
        let arbitrary = Expr::function(Symbol::new("unknown").unwrap(), x);
        assert_eq!(
            classify(&arbitrary),
            Err(ElementaryIntegrationError::UnsupportedFunction)
        );
    }
}
