//! Bounded polynomial trigonometric products and polynomial substitution.

use super::*;

type Failure = ElementaryIntegrationError;

struct Input<'a> {
    multiplier: RationalPolynomial,
    argument: &'a Expr,
    polynomial: RationalPolynomial,
    rule: ElementaryPrimitiveRule,
}

fn input<'a>(
    context: &EvaluationContext,
    source: &'a DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    work: &mut WorkBudget,
) -> Result<Input<'a>, Failure> {
    check_source(context, source, domain, work)?;
    if source.construction_domain() != Some(domain) || source.has_external_history() {
        return Err(Failure::SourceMismatch);
    }
    let factors = match source.expression().view() {
        ExprView::Mul(factors) => factors,
        _ => std::slice::from_ref(source.expression()),
    };
    let mut multiplier = RationalPolynomial::new(vec![Rational::from(Integer::one())]);
    let mut analytic = None;
    for factor in factors {
        work.charge(1)?;
        if let ExprView::Function { name, argument } = factor.view() {
            let rule = match name.name() {
                "sin" => ElementaryPrimitiveRule::Sine,
                "cos" => ElementaryPrimitiveRule::Cosine,
                "exp" => ElementaryPrimitiveRule::Exponential,
                _ => return Err(Failure::UnsupportedFunction),
            };
            if analytic.replace((rule, argument)).is_some() {
                return Err(Failure::UnsupportedForm);
            }
        } else {
            let polynomial = work.convert(factor, variable).map_err(map_conversion)?;
            multiplier = work.multiply(&multiplier, &polynomial)?;
        }
    }
    let (rule, argument) = analytic.ok_or(Failure::UnsupportedForm)?;
    let polynomial = work.convert(argument, variable).map_err(map_conversion)?;
    context.check_polynomial(&multiplier)?;
    context.check_polynomial(&polynomial)?;
    if multiplier.is_zero() {
        return Err(Failure::UnsupportedForm);
    }
    Ok(Input {
        multiplier,
        argument,
        polynomial,
        rule,
    })
}

fn scale(
    work: &mut WorkBudget,
    polynomial: &RationalPolynomial,
    factor: &Rational,
) -> Result<RationalPolynomial, Failure> {
    Ok(work.multiply(polynomial, &RationalPolynomial::new(vec![factor.clone()]))?)
}

fn rebuild(
    context: &EvaluationContext,
    source: &DomainExpression,
    expression: Expr,
) -> Result<DomainExpression, Failure> {
    context.check_expression(&expression)?;
    let limits = context.limits();
    let result = source.rebuild_with_expression(
        expression,
        limits.max_expression_nodes,
        limits.max_rational_conditions,
    )?;
    if result.conditions() != source.conditions()
        || result.construction_domain() != source.construction_domain()
    {
        return Err(Failure::IncorrectConditions);
    }
    Ok(result)
}

/// One executed integration-by-parts reduction.
/// The remaining integral has the opposite trigonometric function and the
/// recorded remainder multiplier; the boundary term uses that same function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrigonometricPartsStep {
    rule: ElementaryPrimitiveRule,
    multiplier: RationalPolynomial,
    boundary: RationalPolynomial,
    remainder: RationalPolynomial,
}

impl TrigonometricPartsStep {
    pub const fn rule(&self) -> ElementaryPrimitiveRule {
        self.rule
    }

    pub fn multiplier(&self) -> &RationalPolynomial {
        &self.multiplier
    }

    pub fn boundary(&self) -> &RationalPolynomial {
        &self.boundary
    }

    pub fn remainder(&self) -> &RationalPolynomial {
        &self.remainder
    }
}

/// Source-bound primitive A(x) sin(ax+b) + B(x) cos(ax+b).
/// The trace records each reduction until the residual multiplier is zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialTrigonometricPrimitive {
    source: DomainExpression,
    variable: Symbol,
    domain: ScalarDomain,
    primitive: DomainExpression,
    slope: Rational,
    intercept: Rational,
    sine: RationalPolynomial,
    cosine: RationalPolynomial,
    steps: Vec<TrigonometricPartsStep>,
}

impl PolynomialTrigonometricPrimitive {
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

    pub fn slope(&self) -> &Rational {
        &self.slope
    }

    pub fn intercept(&self) -> &Rational {
        &self.intercept
    }

    pub fn sine_polynomial(&self) -> &RationalPolynomial {
        &self.sine
    }

    pub fn cosine_polynomial(&self) -> &RationalPolynomial {
        &self.cosine
    }

    pub fn steps(&self) -> &[TrigonometricPartsStep] {
        &self.steps
    }
}

fn opposite(rule: ElementaryPrimitiveRule) -> Result<ElementaryPrimitiveRule, Failure> {
    match rule {
        ElementaryPrimitiveRule::Sine => Ok(ElementaryPrimitiveRule::Cosine),
        ElementaryPrimitiveRule::Cosine => Ok(ElementaryPrimitiveRule::Sine),
        ElementaryPrimitiveRule::Exponential => Err(Failure::UnsupportedForm),
    }
}

fn trig_expression(
    variable: &Symbol,
    argument: &Expr,
    sine: &RationalPolynomial,
    cosine: &RationalPolynomial,
) -> Expr {
    Expr::add([
        Expr::mul([sine.to_expr(variable), Expr::sin(argument.clone())]),
        Expr::mul([cosine.to_expr(variable), Expr::cos(argument.clone())]),
    ])
}

impl EvaluationContext {
    /// Executes repeated integration by parts using exact polynomial
    /// coefficient arithmetic, bounded to degree 16 and context limits.
    pub fn integrate_polynomial_trigonometric(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
    ) -> Result<PolynomialTrigonometricPrimitive, Failure> {
        let mut work = exponential_budget(self);
        let input = input(self, source, variable, domain, &mut work)?;
        let (slope, intercept) = affine_coefficients(self, input.argument, variable, &mut work)?;
        let mut rule = input.rule;
        opposite(rule)?;
        let one = Rational::from(Integer::one());
        let inverse = work.quotient(&one, &slope)?;
        let mut multiplier = input.multiplier;
        let mut sine = RationalPolynomial::zero();
        let mut cosine = RationalPolynomial::zero();
        let mut steps = Vec::new();
        while !multiplier.is_zero() {
            work.charge(1)?;
            let next_rule = opposite(rule)?;
            let factor = if rule == ElementaryPrimitiveRule::Sine {
                -&inverse
            } else {
                inverse.clone()
            };
            let boundary = scale(&mut work, &multiplier, &factor)?;
            let derivative = work.differentiate(&boundary)?;
            let remainder = work.subtract(&RationalPolynomial::zero(), &derivative)?;
            if next_rule == ElementaryPrimitiveRule::Sine {
                sine = work.add(&sine, &boundary)?;
            } else {
                cosine = work.add(&cosine, &boundary)?;
            }
            steps.push(TrigonometricPartsStep {
                rule,
                multiplier,
                boundary,
                remainder: remainder.clone(),
            });
            rule = next_rule;
            multiplier = remainder;
        }
        let primitive = rebuild(
            self,
            source,
            trig_expression(variable, input.argument, &sine, &cosine),
        )?;
        let claim = PolynomialTrigonometricPrimitive {
            source: source.clone(),
            variable: variable.clone(),
            domain,
            primitive,
            slope,
            intercept,
            sine,
            cosine,
            steps,
        };
        self.verify_polynomial_trigonometric_primitive(source, variable, domain, &claim)?;
        Ok(claim)
    }

    /// Verifies every recorded reduction and independently checks the final
    /// product/chain-rule coefficients A' - aB and B' + aA.
    pub fn verify_polynomial_trigonometric_primitive(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        claim: &PolynomialTrigonometricPrimitive,
    ) -> Result<(), Failure> {
        if claim.source != *source || &claim.variable != variable || claim.domain != domain {
            return Err(Failure::SourceMismatch);
        }
        let mut work = exponential_budget(self);
        let input = input(self, source, variable, domain, &mut work)?;
        let (slope, intercept) = affine_coefficients(self, input.argument, variable, &mut work)?;
        if claim.slope != slope || claim.intercept != intercept {
            return Err(Failure::IncorrectAffineEvidence);
        }
        let zero = RationalPolynomial::zero();
        let mut sine = zero.clone();
        let mut cosine = zero.clone();
        let mut rule = input.rule;
        opposite(rule)?;
        let mut remaining = &input.multiplier;
        if claim.steps.len() > MAX_EXPONENTIAL_POLYNOMIAL_DEGREE + 1 {
            return Err(Failure::IncorrectDerivativeEvidence);
        }
        for step in &claim.steps {
            work.charge(1)?;
            if remaining.is_zero() || step.rule != rule || &step.multiplier != remaining {
                return Err(Failure::IncorrectDerivativeEvidence);
            }
            let signed_slope = if rule == ElementaryPrimitiveRule::Sine {
                -&slope
            } else {
                slope.clone()
            };
            if scale(&mut work, &step.boundary, &signed_slope)? != *remaining {
                return Err(Failure::IncorrectDerivativeEvidence);
            }
            let derivative = work.differentiate(&step.boundary)?;
            if work.add(&derivative, &step.remainder)? != zero {
                return Err(Failure::IncorrectDerivativeEvidence);
            }
            rule = opposite(rule)?;
            if rule == ElementaryPrimitiveRule::Sine {
                sine = work.add(&sine, &step.boundary)?;
            } else {
                cosine = work.add(&cosine, &step.boundary)?;
            }
            remaining = &step.remainder;
        }
        if !remaining.is_zero() || sine != claim.sine || cosine != claim.cosine {
            return Err(Failure::IncorrectDerivativeEvidence);
        }
        self.check_polynomial(&claim.sine)?;
        self.check_polynomial(&claim.cosine)?;
        let sine_derivative = work.differentiate(&claim.sine)?;
        let cosine_derivative = work.differentiate(&claim.cosine)?;
        let scaled_cosine = scale(&mut work, &claim.cosine, &slope)?;
        let scaled_sine = scale(&mut work, &claim.sine, &slope)?;
        let actual_sine = work.subtract(&sine_derivative, &scaled_cosine)?;
        let actual_cosine = work.add(&cosine_derivative, &scaled_sine)?;
        let (expected_sine, expected_cosine) = if input.rule == ElementaryPrimitiveRule::Sine {
            (&input.multiplier, &zero)
        } else {
            (&zero, &input.multiplier)
        };
        if &actual_sine != expected_sine || &actual_cosine != expected_cosine {
            return Err(Failure::IncorrectDerivative);
        }
        let expected = rebuild(
            self,
            source,
            trig_expression(variable, input.argument, &claim.sine, &claim.cosine),
        )?;
        self.check_expression(claim.primitive.expression())?;
        if claim.primitive != expected {
            return Err(Failure::IncorrectPrimitive);
        }
        Ok(())
    }
}

/// Exact substitution evidence: multiplier = scale * derivative(inner).
/// The source and primitive retain the same construction domain and conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialSubstitutionPrimitive {
    source: DomainExpression,
    variable: Symbol,
    domain: ScalarDomain,
    primitive: DomainExpression,
    rule: ElementaryPrimitiveRule,
    inner: RationalPolynomial,
    derivative: RationalPolynomial,
    scale: Rational,
}

impl PolynomialSubstitutionPrimitive {
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

    pub const fn rule(&self) -> ElementaryPrimitiveRule {
        self.rule
    }

    pub fn inner(&self) -> &RationalPolynomial {
        &self.inner
    }

    pub fn inner_derivative(&self) -> &RationalPolynomial {
        &self.derivative
    }

    pub fn scale(&self) -> &Rational {
        &self.scale
    }
}

fn substitution_candidate(input: &Input<'_>, factor: &Rational) -> Expr {
    let (coefficient, function) = match input.rule {
        ElementaryPrimitiveRule::Sine => (-factor, Expr::cos(input.argument.clone())),
        ElementaryPrimitiveRule::Cosine => (factor.clone(), Expr::sin(input.argument.clone())),
        ElementaryPrimitiveRule::Exponential => (factor.clone(), Expr::exp(input.argument.clone())),
    };
    Expr::mul([Expr::from(coefficient), function])
}

impl EvaluationContext {
    /// Integrates a rational multiple of g'(x) times sin(g), cos(g) or exp(g)
    /// for a bounded nonconstant rational polynomial g. No division by g'
    /// occurs, so zeros of the inner derivative remain admissible.
    pub fn integrate_polynomial_substitution(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
    ) -> Result<PolynomialSubstitutionPrimitive, Failure> {
        let mut work = exponential_budget(self);
        let input = input(self, source, variable, domain, &mut work)?;
        let derivative = work.differentiate(&input.polynomial)?;
        let leading = derivative.coefficients().last().ok_or(Failure::ZeroSlope)?;
        let factor = work.quotient(
            input.multiplier.coefficients().last().ok_or(Failure::UnsupportedForm)?,
            leading,
        )?;
        if scale(&mut work, &derivative, &factor)? != input.multiplier {
            return Err(Failure::UnsupportedForm);
        }
        let primitive = rebuild(self, source, substitution_candidate(&input, &factor))?;
        let claim = PolynomialSubstitutionPrimitive {
            source: source.clone(),
            variable: variable.clone(),
            domain,
            primitive,
            rule: input.rule,
            inner: input.polynomial,
            derivative,
            scale: factor,
        };
        self.verify_polynomial_substitution_primitive(source, variable, domain, &claim)?;
        Ok(claim)
    }

    /// Checks the exact inner derivative, factor identity and inverse chain
    /// rule against the supplied source, without rerunning factor selection.
    pub fn verify_polynomial_substitution_primitive(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        claim: &PolynomialSubstitutionPrimitive,
    ) -> Result<(), Failure> {
        if claim.source != *source || &claim.variable != variable || claim.domain != domain {
            return Err(Failure::SourceMismatch);
        }
        let mut work = exponential_budget(self);
        let input = input(self, source, variable, domain, &mut work)?;
        if claim.rule != input.rule || claim.inner != input.polynomial {
            return Err(Failure::IncorrectRule);
        }
        self.check_rational(&claim.scale)?;
        let derivative = work.differentiate(&input.polynomial)?;
        if derivative.is_zero()
            || derivative != claim.derivative
            || scale(&mut work, &derivative, &claim.scale)? != input.multiplier
        {
            return Err(Failure::IncorrectDerivative);
        }
        let expected = rebuild(self, source, substitution_candidate(&input, &claim.scale))?;
        self.check_expression(claim.primitive.expression())?;
        if claim.primitive != expected {
            return Err(Failure::IncorrectPrimitive);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceLimits;

    fn variable() -> Symbol {
        Symbol::new("x").unwrap()
    }

    fn x() -> Expr {
        Expr::from(variable())
    }

    fn source(expression: Expr) -> DomainExpression {
        DomainExpression::from_expr_in_domain(expression, ScalarDomain::Real, 256, 32).unwrap()
    }

    fn q(n: i64, d: i64) -> Rational {
        Rational::new(Integer::from(n), Integer::from(d)).unwrap()
    }

    fn trig(expression: Expr) -> PolynomialTrigonometricPrimitive {
        let context = EvaluationContext::default();
        let source = source(expression);
        let claim = context
            .integrate_polynomial_trigonometric(&source, &variable(), ScalarDomain::Real)
            .unwrap();
        context
            .verify_polynomial_trigonometric_primitive(&source, &variable(), ScalarDomain::Real, &claim)
            .unwrap();
        claim
    }

    #[test]
    fn sine_product() {
        let claim = trig(Expr::mul([x(), Expr::sin(x())]));
        assert_eq!(claim.sine.coefficients(), &[q(1, 1)]);
        assert_eq!(claim.cosine.coefficients(), &[q(0, 1), q(-1, 1)]);
        assert_eq!(claim.steps.len(), 2);
        assert!(claim.steps[1].remainder.is_zero());
    }

    #[test]
    fn cosine_product() {
        let claim = trig(Expr::mul([
            x(),
            Expr::cos(Expr::mul([Expr::from(2), x()])),
        ]));
        assert_eq!(claim.sine.coefficients(), &[q(0, 1), q(1, 2)]);
        assert_eq!(claim.cosine.coefficients(), &[q(1, 4)]);
    }

    #[test]
    fn quadratic_sine_product() {
        let claim = trig(Expr::mul([Expr::pow(x(), Expr::from(2)), Expr::sin(x())]));
        assert_eq!(claim.sine.coefficients(), &[q(0, 1), q(2, 1)]);
        assert_eq!(claim.cosine.coefficients(), &[q(2, 1), q(0, 1), q(-1, 1)]);
        assert_eq!(claim.steps.len(), 3);
    }

    #[test]
    fn negative_slope_and_offset() {
        let argument = Expr::add([Expr::mul([Expr::from(-2), x()]), Expr::from(3)]);
        let claim = trig(Expr::mul([x(), Expr::cos(argument)]));
        assert_eq!(claim.sine.coefficients(), &[q(0, 1), q(-1, 2)]);
        assert_eq!(claim.cosine.coefficients(), &[q(1, 4)]);
        assert_eq!(claim.intercept, q(3, 1));
    }

    #[test]
    fn substitution_cosine_and_exponential() {
        let context = EvaluationContext::default();
        let inner = Expr::pow(x(), Expr::from(2));
        for (function, expected) in [
            (Expr::cos(inner.clone()), Expr::sin(inner.clone())),
            (Expr::exp(inner.clone()), Expr::exp(inner.clone())),
            (Expr::sin(inner.clone()), Expr::mul([Expr::from(-1), Expr::cos(inner.clone())])),
        ] {
            let input = source(Expr::mul([Expr::from(2), x(), function]));
            let claim = context
                .integrate_polynomial_substitution(&input, &variable(), ScalarDomain::Real)
                .unwrap();
            assert_eq!(claim.primitive.expression(), &expected);
            assert_eq!(claim.scale, q(1, 1));
            assert!(claim.primitive.conditions().is_empty());
        }
    }

    #[test]
    fn historical_hole_is_preserved() {
        let hole = DomainExpression::multiply(
            [source(Expr::from(0)), source(Expr::pow(x(), Expr::from(-1)))],
            32,
        )
        .unwrap();
        let input = DomainExpression::add(
            [hole, source(Expr::mul([x(), Expr::sin(x())]))],
            32,
        )
        .unwrap();
        let context = EvaluationContext::default();
        let claim = context
            .integrate_polynomial_trigonometric(&input, &variable(), ScalarDomain::Real)
            .unwrap();
        assert_eq!(claim.primitive.conditions(), &[Condition::NonZero(x())]);
    }

    #[test]
    fn refuses_missing_factor_and_unsupported_products() {
        let context = EvaluationContext::default();
        let inner = Expr::pow(x(), Expr::from(2));
        for expression in [
            Expr::cos(inner.clone()),
            Expr::mul([x(), Expr::sin(x()), Expr::cos(inner)]),
            Expr::mul([Expr::pow(x(), Expr::from(-1)), Expr::sin(x())]),
            Expr::mul([x(), Expr::cos(Expr::from(3))]),
        ] {
            let input = source(expression);
            assert!(context
                .integrate_polynomial_substitution(&input, &variable(), ScalarDomain::Real)
                .is_err());
            assert!(context
                .integrate_polynomial_trigonometric(&input, &variable(), ScalarDomain::Real)
                .is_err());
        }
    }

    #[test]
    fn rejects_forged_steps_and_primitive() {
        let context = EvaluationContext::default();
        let input = source(Expr::mul([x(), Expr::sin(x())]));
        let claim = context
            .integrate_polynomial_trigonometric(&input, &variable(), ScalarDomain::Real)
            .unwrap();
        let mut forged = claim.clone();
        forged.steps[0].boundary = RationalPolynomial::zero();
        assert!(context
            .verify_polynomial_trigonometric_primitive(&input, &variable(), ScalarDomain::Real, &forged)
            .is_err());
        let mut forged = claim.clone();
        forged.steps.pop();
        assert!(context
            .verify_polynomial_trigonometric_primitive(&input, &variable(), ScalarDomain::Real, &forged)
            .is_err());
        let mut forged = claim;
        forged.primitive = source(x());
        assert!(context
            .verify_polynomial_trigonometric_primitive(&input, &variable(), ScalarDomain::Real, &forged)
            .is_err());
    }

    #[test]
    fn rejects_forged_substitution_factor() {
        let context = EvaluationContext::default();
        let input = source(Expr::mul([x(), Expr::cos(Expr::pow(x(), Expr::from(2)))]));
        let mut claim = context
            .integrate_polynomial_substitution(&input, &variable(), ScalarDomain::Real)
            .unwrap();
        assert_eq!(claim.scale, q(1, 2));
        claim.scale = q(1, 1);
        assert!(context
            .verify_polynomial_substitution_primitive(&input, &variable(), ScalarDomain::Real, &claim)
            .is_err());
    }

    #[test]
    fn bounded_and_deterministic() {
        let expression = Expr::mul([Expr::pow(x(), Expr::from(16)), Expr::sin(x())]);
        assert_eq!(trig(expression.clone()), trig(expression));
        let context = EvaluationContext::default();
        let input = source(Expr::mul([Expr::pow(x(), Expr::from(17)), Expr::sin(x())]));
        assert!(context
            .integrate_polynomial_trigonometric(&input, &variable(), ScalarDomain::Real)
            .is_err());
        let context = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        let input = source(Expr::mul([x(), Expr::sin(x())]));
        assert!(context
            .integrate_polynomial_trigonometric(&input, &variable(), ScalarDomain::Real)
            .is_err());
    }
}
