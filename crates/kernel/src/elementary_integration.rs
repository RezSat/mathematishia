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
    ResourceLimitKind, SymbolicDerivative, verify_symbolic_derivative,
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
