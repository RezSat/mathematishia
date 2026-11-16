//! Exact polynomial antiderivatives and fundamental-theorem verification.
//!
//! A polynomial over Q has an everywhere-defined primitive. Each returned
//! primitive chooses constant term zero; arbitrary constants are not encoded.

use std::error::Error;
use std::fmt;

use mathema::{DomainExpression, Integer, Rational, Symbol};

use crate::budget::WorkBudget;
use crate::{
    EvaluationContext, EvaluationError, PolynomialConversionError, RationalPolynomial,
    ResourceLimitKind,
};

/// Rejection of an unsupported domain, exhausted budget, or false calculus claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationError {
    Evaluation(EvaluationError),
    UnsupportedIntegrand(PolynomialConversionError),
    RestrictedDomain,
    NoncanonicalConstant,
    IncorrectDerivative,
    IncorrectFundamentalTheorem,
    IncorrectTermwiseValue,
}

impl From<EvaluationError> for IntegrationError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::UnsupportedIntegrand(error) => write!(f, "unsupported integrand: {error}"),
            Self::RestrictedDomain => {
                f.write_str("polynomial integration does not support source-domain exclusions")
            }
            Self::NoncanonicalConstant => f.write_str("primitive must have zero constant term"),
            Self::IncorrectDerivative => f.write_str("derivative does not equal the integrand"),
            Self::IncorrectFundamentalTheorem => {
                f.write_str("definite integral differs from upper-minus-lower primitive values")
            }
            Self::IncorrectTermwiseValue => {
                f.write_str("definite integral differs from exact termwise integration")
            }
        }
    }
}

impl Error for IntegrationError {}

/// A certified canonical antiderivative in Q[x], with constant term zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialPrimitive {
    integrand: RationalPolynomial,
    primitive: RationalPolynomial,
}

impl PolynomialPrimitive {
    pub fn integrand(&self) -> &RationalPolynomial {
        &self.integrand
    }

    pub fn primitive(&self) -> &RationalPolynomial {
        &self.primitive
    }
}

/// A directed integral over rational endpoints, with an inspectable primitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitePolynomialIntegral {
    primitive: PolynomialPrimitive,
    lower: Rational,
    upper: Rational,
    value: Rational,
}

impl DefinitePolynomialIntegral {
    pub fn primitive(&self) -> &PolynomialPrimitive {
        &self.primitive
    }

    pub fn lower(&self) -> &Rational {
        &self.lower
    }

    pub fn upper(&self) -> &Rational {
        &self.upper
    }

    pub fn value(&self) -> &Rational {
        &self.value
    }
}

fn primitive_checked(
    context: &EvaluationContext,
    input: &RationalPolynomial,
    work: &mut WorkBudget,
) -> Result<PolynomialPrimitive, IntegrationError> {
    context.check_polynomial(input)?;
    // The primitive must fit the same degree bound as other checked polynomials.
    if input
        .degree()
        .is_some_and(|degree| degree >= context.limits().max_polynomial_degree)
    {
        return Err(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree).into());
    }

    let mut coefficients = Vec::with_capacity(input.coefficients().len().saturating_add(1));
    if !input.is_zero() {
        coefficients.push(Rational::from(Integer::zero()));
        for (degree, coefficient) in input.coefficients().iter().enumerate() {
            let divisor = Rational::from(Integer::from(degree + 1));
            coefficients.push(work.quotient(coefficient, &divisor)?);
        }
    }
    let primitive = RationalPolynomial::new(coefficients);
    context.check_polynomial(&primitive)?;
    Ok(PolynomialPrimitive {
        integrand: input.clone(),
        primitive,
    })
}

fn verify_primitive_checked(
    context: &EvaluationContext,
    claim: &PolynomialPrimitive,
    work: &mut WorkBudget,
) -> Result<(), IntegrationError> {
    context.check_polynomial(&claim.integrand)?;
    context.check_polynomial(&claim.primitive)?;
    if claim
        .primitive
        .coefficients()
        .first()
        .is_some_and(|constant| !constant.numerator().is_zero())
    {
        return Err(IntegrationError::NoncanonicalConstant);
    }
    // Independent verification: formal differentiation, never reintegration.
    if work.differentiate(&claim.primitive)? != claim.integrand {
        return Err(IntegrationError::IncorrectDerivative);
    }
    Ok(())
}

fn verify_definite_checked(
    context: &EvaluationContext,
    claim: &DefinitePolynomialIntegral,
    work: &mut WorkBudget,
) -> Result<(), IntegrationError> {
    verify_primitive_checked(context, &claim.primitive, work)?;
    context.check_rational(&claim.lower)?;
    context.check_rational(&claim.upper)?;
    context.check_rational(&claim.value)?;

    // Fundamental theorem: compute both endpoint substitutions independently.
    let low = work.evaluate(claim.primitive.primitive(), &claim.lower)?;
    let high = work.evaluate(claim.primitive.primitive(), &claim.upper)?;
    let difference = work.subtract_scalar(&high, &low)?;
    if difference != claim.value {
        return Err(IntegrationError::IncorrectFundamentalTheorem);
    }

    // Separate coefficient-by-coefficient computation of the same integral.
    // This does not reuse the claimed primitive or Horner's method.
    let mut low_power = Rational::from(Integer::one());
    let mut high_power = Rational::from(Integer::one());
    let mut sum = Rational::from(Integer::zero());
    for (degree, coefficient) in claim
        .primitive
        .integrand()
        .coefficients()
        .iter()
        .enumerate()
    {
        low_power = work.multiply_scalar(&low_power, &claim.lower)?;
        high_power = work.multiply_scalar(&high_power, &claim.upper)?;
        let power_difference = work.subtract_scalar(&high_power, &low_power)?;
        let divisor = Rational::from(Integer::from(degree + 1));
        let weight = work.quotient(coefficient, &divisor)?;
        let term = work.multiply_scalar(&weight, &power_difference)?;
        sum = work.add_scalar(&sum, &term)?;
    }
    if sum != claim.value {
        return Err(IntegrationError::IncorrectTermwiseValue);
    }
    Ok(())
}

fn map_expression_error(error: EvaluationError) -> IntegrationError {
    match error {
        EvaluationError::Polynomial(
            reason @ (PolynomialConversionError::OtherSymbol
            | PolynomialConversionError::UnsupportedPower),
        ) => IntegrationError::UnsupportedIntegrand(reason),
        EvaluationError::Polynomial(PolynomialConversionError::DegreeLimitExceeded) => {
            IntegrationError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::PolynomialDegree,
            ))
        }
        other => IntegrationError::Evaluation(other),
    }
}

impl EvaluationContext {
    /// Integrates an exact Q[x] polynomial with integration constant fixed at zero.
    ///
    /// The output degree must not exceed max_polynomial_degree. Generation and
    /// independent derivative verification share a per-call work budget.
    pub fn integrate_polynomial(
        &self,
        integrand: &RationalPolynomial,
    ) -> Result<PolynomialPrimitive, IntegrationError> {
        let mut work = WorkBudget::new(self.limits());
        let result = primitive_checked(self, integrand, &mut work)?;
        verify_primitive_checked(self, &result, &mut work)?;
        Ok(result)
    }

    /// Integrates a domain-aware expression only if it denotes an unrestricted
    /// rational polynomial in the specified variable. No conditions are dropped.
    pub fn integrate_domain_expression(
        &self,
        integrand: &DomainExpression,
        variable: &Symbol,
    ) -> Result<PolynomialPrimitive, IntegrationError> {
        if !integrand.conditions().is_empty() {
            return Err(IntegrationError::RestrictedDomain);
        }
        let polynomial = self
            .polynomial(integrand.expression(), variable)
            .map_err(map_expression_error)?;
        self.integrate_polynomial(&polynomial)
    }

    /// Evaluates the directed integral from lower to upper by exact endpoint
    /// substitution, then checks the derivative and a separate termwise sum.
    pub fn integrate_polynomial_between(
        &self,
        integrand: &RationalPolynomial,
        lower: &Rational,
        upper: &Rational,
    ) -> Result<DefinitePolynomialIntegral, IntegrationError> {
        self.check_rational(lower)?;
        self.check_rational(upper)?;
        let mut work = WorkBudget::new(self.limits());
        let primitive = primitive_checked(self, integrand, &mut work)?;
        let low = work.evaluate(primitive.primitive(), lower)?;
        let high = work.evaluate(primitive.primitive(), upper)?;
        let value = work.subtract_scalar(&high, &low)?;
        self.check_rational(&value)?;
        let result = DefinitePolynomialIntegral {
            primitive,
            lower: lower.clone(),
            upper: upper.clone(),
            value,
        };
        verify_definite_checked(self, &result, &mut work)?;
        Ok(result)
    }

    /// Domain-aware counterpart of integrate_polynomial_between.
    pub fn integrate_domain_expression_between(
        &self,
        integrand: &DomainExpression,
        variable: &Symbol,
        lower: &Rational,
        upper: &Rational,
    ) -> Result<DefinitePolynomialIntegral, IntegrationError> {
        if !integrand.conditions().is_empty() {
            return Err(IntegrationError::RestrictedDomain);
        }
        let polynomial = self
            .polynomial(integrand.expression(), variable)
            .map_err(map_expression_error)?;
        self.integrate_polynomial_between(&polynomial, lower, upper)
    }

    /// Independently checks a claimed primitive by exact differentiation.
    pub fn verify_polynomial_primitive(
        &self,
        claim: &PolynomialPrimitive,
    ) -> Result<(), IntegrationError> {
        verify_primitive_checked(self, claim, &mut WorkBudget::new(self.limits()))
    }

    /// Checks both the fundamental-theorem substitution and a termwise integral.
    pub fn verify_definite_polynomial_integral(
        &self,
        claim: &DefinitePolynomialIntegral,
    ) -> Result<(), IntegrationError> {
        verify_definite_checked(self, claim, &mut WorkBudget::new(self.limits()))
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    fn q(value: i64) -> Rational {
        Rational::from(Integer::from(value))
    }

    #[test]
    fn rejects_forged_primitives_and_constants() {
        let context = EvaluationContext::default();
        let input = RationalPolynomial::new(vec![q(1), q(2)]);
        let mut result = context.integrate_polynomial(&input).unwrap();
        result.primitive = RationalPolynomial::new(vec![q(4), q(1), q(1)]);
        assert_eq!(
            context.verify_polynomial_primitive(&result),
            Err(IntegrationError::NoncanonicalConstant)
        );
        result.primitive = RationalPolynomial::new(vec![q(0), q(1), q(2)]);
        assert_eq!(
            context.verify_polynomial_primitive(&result),
            Err(IntegrationError::IncorrectDerivative)
        );
    }

    #[test]
    fn rejects_forged_definite_values_and_bounds() {
        let context = EvaluationContext::default();
        let input = RationalPolynomial::new(vec![q(3)]);
        let mut claim = context
            .integrate_polynomial_between(&input, &q(-1), &q(2))
            .unwrap();
        claim.value = q(20);
        assert_eq!(
            context.verify_definite_polynomial_integral(&claim),
            Err(IntegrationError::IncorrectFundamentalTheorem)
        );
        claim.value = q(9);
        claim.upper = q(3);
        assert_eq!(
            context.verify_definite_polynomial_integral(&claim),
            Err(IntegrationError::IncorrectFundamentalTheorem)
        );
    }
}
