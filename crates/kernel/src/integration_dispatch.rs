//! Exact integration dispatch over rational polynomials and certified affine
//! sine, cosine and exponential terms.
//!
//! Only top-level sums are decomposed. The result fixes the additive
//! integration constant at zero by adding the selected termwise primitives.
//! Domain conditions are obligations inherited from the source, not proofs.

use std::error::Error;
use std::fmt;

use mathema::{
    Condition, DefinednessError, DomainExpression, Expr, ExprView, ScalarDomain, Symbol,
    definedness_obligations_in_domain,
};

use crate::budget::WorkBudget;
use crate::{
    DefinitePolynomialIntegral, ElementaryAntiderivative, ElementaryIntegrationError,
    EvaluationContext, EvaluationError, IntegrationError, PolynomialConversionError,
    PolynomialPrimitive,
};

const MAX_TERMS: usize = 64;

/// A typed unsupported case, exhausted budget, or failed certificate check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationDispatchError {
    Evaluation(EvaluationError),
    Definedness(DefinednessError),
    Polynomial(IntegrationError),
    Elementary(ElementaryIntegrationError),
    UnsupportedScalarDomain,
    TooManyTerms,
    MissingDomainCondition,
    UnsupportedAnalyticDefiniteIntegral,
    RestrictedDefiniteDomain,
    SourceMismatch,
    IncorrectEvidence,
    IncorrectPrimitive,
    IncorrectConditions,
}

impl fmt::Display for IntegrationDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Definedness(error) => write!(f, "{error}"),
            Self::Polynomial(error) => write!(f, "{error}"),
            Self::Elementary(error) => write!(f, "{error}"),
            Self::UnsupportedScalarDomain => {
                f.write_str("integrating over integer scalars is unsupported")
            }
            Self::TooManyTerms => f.write_str("integrand exceeds the term budget"),
            Self::MissingDomainCondition => {
                f.write_str("integrand lacks a necessary domain obligation")
            }
            Self::UnsupportedAnalyticDefiniteIntegral => {
                f.write_str("exact analytic endpoint values are not supported")
            }
            Self::RestrictedDefiniteDomain => {
                f.write_str("definite integration with unproved domain conditions is unsupported")
            }
            Self::SourceMismatch => {
                f.write_str("integration claim does not match source, variable or domain")
            }
            Self::IncorrectEvidence => f.write_str("integration term evidence is invalid"),
            Self::IncorrectPrimitive => f.write_str("primitive does not match certified terms"),
            Self::IncorrectConditions => f.write_str("primitive lost source domain obligations"),
        }
    }
}

impl Error for IntegrationDispatchError {}

impl From<EvaluationError> for IntegrationDispatchError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl From<DefinednessError> for IntegrationDispatchError {
    fn from(error: DefinednessError) -> Self {
        Self::Definedness(error)
    }
}

impl From<IntegrationError> for IntegrationDispatchError {
    fn from(error: IntegrationError) -> Self {
        Self::Polynomial(error)
    }
}

impl From<ElementaryIntegrationError> for IntegrationDispatchError {
    fn from(error: ElementaryIntegrationError) -> Self {
        Self::Elementary(error)
    }
}

/// One term's independently checked source proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationTermEvidence {
    Polynomial {
        integrand: Expr,
        certificate: PolynomialPrimitive,
    },
    Elementary(Box<ElementaryAntiderivative>),
}

impl IntegrationTermEvidence {
    pub fn source_expression(&self) -> &Expr {
        match self {
            Self::Polynomial { integrand, .. } => integrand,
            Self::Elementary(certificate) => certificate.source().expression(),
        }
    }

    pub fn polynomial_certificate(&self) -> Option<&PolynomialPrimitive> {
        match self {
            Self::Polynomial { certificate, .. } => Some(certificate),
            Self::Elementary(_) => None,
        }
    }

    pub fn elementary_certificate(&self) -> Option<&ElementaryAntiderivative> {
        match self {
            Self::Polynomial { .. } => None,
            Self::Elementary(certificate) => Some(certificate),
        }
    }
}

/// An exact, checked zero-additive-constant primitive with its source proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchedAntiderivative {
    source: DomainExpression,
    variable: Symbol,
    domain: ScalarDomain,
    primitive: DomainExpression,
    terms: Vec<IntegrationTermEvidence>,
}

impl DispatchedAntiderivative {
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

    pub fn terms(&self) -> &[IntegrationTermEvidence] {
        &self.terms
    }
}

/// Directed rational-bound integral using the exact polynomial certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchedDefiniteIntegral {
    source: DomainExpression,
    variable: Symbol,
    domain: ScalarDomain,
    certificate: DefinitePolynomialIntegral,
}

impl DispatchedDefiniteIntegral {
    pub fn source(&self) -> &DomainExpression {
        &self.source
    }

    pub fn variable(&self) -> &Symbol {
        &self.variable
    }

    pub const fn domain(&self) -> ScalarDomain {
        self.domain
    }

    pub fn certificate(&self) -> &DefinitePolynomialIntegral {
        &self.certificate
    }
}

fn check_source(
    context: &EvaluationContext,
    source: &DomainExpression,
    domain: ScalarDomain,
    work: &mut WorkBudget,
) -> Result<(), IntegrationDispatchError> {
    if domain == ScalarDomain::Integer {
        return Err(IntegrationDispatchError::UnsupportedScalarDomain);
    }
    context.check_expression(source.expression())?;
    if source.conditions().len() > context.limits().max_rational_conditions {
        return Err(EvaluationError::Limit(crate::ResourceLimitKind::RationalConditions).into());
    }
    for condition in source.conditions() {
        work.charge(1)?;
        context.check_expression(condition.expression())?;
    }
    let required = definedness_obligations_in_domain(
        source.expression(),
        context.limits().max_expression_nodes,
        domain,
    )?;
    if required
        .iter()
        .any(|condition| !source.conditions().contains(condition))
    {
        return Err(IntegrationDispatchError::MissingDomainCondition);
    }
    Ok(())
}

fn source_terms(expression: &Expr) -> &[Expr] {
    match expression.view() {
        ExprView::Add(terms) => terms,
        _ => std::slice::from_ref(expression),
    }
}

fn make_term_source(
    context: &EvaluationContext,
    source: &DomainExpression,
    term: &Expr,
    domain: ScalarDomain,
) -> Result<DomainExpression, IntegrationDispatchError> {
    let limits = context.limits();
    let history = DomainExpression::with_conditions(
        Expr::from(1),
        source.conditions().iter().cloned(),
        limits.max_expression_nodes,
        limits.max_rational_conditions,
    )?;
    let term = DomainExpression::from_expr_in_domain(
        term.clone(),
        domain,
        limits.max_expression_nodes,
        limits.max_rational_conditions,
    )?;
    let result = DomainExpression::multiply([history, term], limits.max_rational_conditions)?;
    if result.conditions() != source.conditions() {
        return Err(IntegrationDispatchError::IncorrectConditions);
    }
    Ok(result)
}

fn dispatch_term(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    term: &Expr,
) -> Result<IntegrationTermEvidence, IntegrationDispatchError> {
    match context.polynomial(term, variable) {
        Ok(polynomial) => Ok(IntegrationTermEvidence::Polynomial {
            integrand: term.clone(),
            certificate: context.integrate_polynomial(&polynomial)?,
        }),
        Err(EvaluationError::Polynomial(PolynomialConversionError::UnsupportedFunction)) => {
            let term_source = make_term_source(context, source, term, domain)?;
            Ok(IntegrationTermEvidence::Elementary(Box::new(
                context.integrate_elementary_affine(&term_source, variable, domain)?,
            )))
        }
        Err(EvaluationError::Polynomial(reason)) => {
            Err(IntegrationError::UnsupportedIntegrand(reason).into())
        }
        Err(other) => Err(other.into()),
    }
}

fn term_primitive(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    term: &Expr,
    proof: &IntegrationTermEvidence,
    work: &mut WorkBudget,
) -> Result<DomainExpression, IntegrationDispatchError> {
    work.charge(1)?;
    if proof.source_expression() != term {
        return Err(IntegrationDispatchError::IncorrectEvidence);
    }
    let limits = context.limits();
    let candidate = match proof {
        IntegrationTermEvidence::Polynomial {
            integrand,
            certificate,
        } => {
            let input = context.polynomial(integrand, variable)?;
            if &input != certificate.integrand() {
                return Err(IntegrationDispatchError::IncorrectEvidence);
            }
            context.verify_polynomial_primitive(certificate)?;
            DomainExpression::with_conditions(
                certificate.primitive().to_expr(variable),
                source.conditions().iter().cloned(),
                limits.max_expression_nodes,
                limits.max_rational_conditions,
            )?
        }
        IntegrationTermEvidence::Elementary(certificate) => {
            let term_source = make_term_source(context, source, term, domain)?;
            context.verify_elementary_antiderivative(
                &term_source,
                variable,
                domain,
                certificate,
            )?;
            work.charge(certificate.derivative_evidence().steps().len() as u64)?;
            certificate.primitive().clone()
        }
    };
    context.check_expression(candidate.expression())?;
    if candidate.conditions() != source.conditions() {
        return Err(IntegrationDispatchError::IncorrectConditions);
    }
    Ok(candidate)
}

fn verify_claim(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
    claim: &DispatchedAntiderivative,
) -> Result<(), IntegrationDispatchError> {
    if claim.source != *source || &claim.variable != variable || claim.domain != domain {
        return Err(IntegrationDispatchError::SourceMismatch);
    }
    let mut work = WorkBudget::new(context.limits());
    check_source(context, source, domain, &mut work)?;
    let source_terms = source_terms(source.expression());
    if source_terms.len() > MAX_TERMS {
        return Err(IntegrationDispatchError::TooManyTerms);
    }
    if source_terms.len() != claim.terms.len() {
        return Err(IntegrationDispatchError::IncorrectEvidence);
    }
    work.charge(source_terms.len() as u64)?;
    let mut primitives = Vec::with_capacity(source_terms.len());
    for (term, proof) in source_terms.iter().zip(&claim.terms) {
        primitives.push(term_primitive(
            context, source, variable, domain, term, proof, &mut work,
        )?);
    }
    let expected = DomainExpression::add(primitives, context.limits().max_rational_conditions)?;
    context.check_expression(expected.expression())?;
    context.check_expression(claim.primitive.expression())?;
    if claim.primitive.conditions() != source.conditions()
        || claim.primitive.conditions() != expected.conditions()
    {
        return Err(IntegrationDispatchError::IncorrectConditions);
    }
    if claim.primitive.expression() != expected.expression() {
        return Err(IntegrationDispatchError::IncorrectPrimitive);
    }
    Ok(())
}

fn definite_polynomial(
    context: &EvaluationContext,
    source: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
) -> Result<crate::RationalPolynomial, IntegrationDispatchError> {
    check_source(
        context,
        source,
        domain,
        &mut WorkBudget::new(context.limits()),
    )?;
    if !source.conditions().is_empty() {
        return Err(IntegrationDispatchError::RestrictedDefiniteDomain);
    }
    match context.polynomial(source.expression(), variable) {
        Ok(polynomial) => Ok(polynomial),
        Err(EvaluationError::Polynomial(PolynomialConversionError::UnsupportedFunction)) => {
            Err(IntegrationDispatchError::UnsupportedAnalyticDefiniteIntegral)
        }
        Err(EvaluationError::Polynomial(reason)) => {
            Err(IntegrationError::UnsupportedIntegrand(reason).into())
        }
        Err(other) => Err(other.into()),
    }
}

impl EvaluationContext {
    /// Integrates a polynomial or a top-level sum of rational polynomials and
    /// certified rational-affine sin/cos/exp terms. An unexpanded product of a
    /// sum, a mixed-function product, log, tan and nonlinear analytic arguments
    /// remain unsupported. Conditions are unproved, retained prerequisites.
    ///
    /// This is an exact formal antiderivative in the selected real, complex or
    /// rational scalar domain. Analytic terms require real or complex scalars.
    pub fn integrate_expression_dispatch(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
    ) -> Result<DispatchedAntiderivative, IntegrationDispatchError> {
        let mut work = WorkBudget::new(self.limits());
        check_source(self, source, domain, &mut work)?;
        let terms = source_terms(source.expression());
        if terms.len() > MAX_TERMS {
            return Err(IntegrationDispatchError::TooManyTerms);
        }
        work.charge(terms.len() as u64)?;
        let mut proofs = Vec::with_capacity(terms.len());
        for term in terms {
            proofs.push(dispatch_term(self, source, variable, domain, term)?);
        }
        let mut primitives = Vec::with_capacity(terms.len());
        for (term, proof) in terms.iter().zip(&proofs) {
            primitives.push(term_primitive(
                self, source, variable, domain, term, proof, &mut work,
            )?);
        }
        let primitive = DomainExpression::add(primitives, self.limits().max_rational_conditions)?;
        self.check_expression(primitive.expression())?;
        let result = DispatchedAntiderivative {
            source: source.clone(),
            variable: variable.clone(),
            domain,
            primitive,
            terms: proofs,
        };
        verify_claim(self, source, variable, domain, &result)?;
        Ok(result)
    }

    /// Checks the original source, every term's underlying certificate and
    /// the exact recomposition of the selected termwise primitive.
    pub fn verify_dispatched_antiderivative(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        claim: &DispatchedAntiderivative,
    ) -> Result<(), IntegrationDispatchError> {
        verify_claim(self, source, variable, domain, claim)
    }

    /// Evaluates only exact Q[x] definite integrals at rational endpoints.
    /// Analytic endpoint evaluation is intentionally not approximated.
    pub fn integrate_expression_dispatch_between(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        lower: &mathema::Rational,
        upper: &mathema::Rational,
    ) -> Result<DispatchedDefiniteIntegral, IntegrationDispatchError> {
        let polynomial = definite_polynomial(self, source, variable, domain)?;
        let certificate = self.integrate_polynomial_between(&polynomial, lower, upper)?;
        let result = DispatchedDefiniteIntegral {
            source: source.clone(),
            variable: variable.clone(),
            domain,
            certificate,
        };
        self.verify_dispatched_definite_integral(source, variable, domain, lower, upper, &result)?;
        Ok(result)
    }

    /// Independently checks the source polynomial, bounds and its existing
    /// formal-derivative, fundamental-theorem and termwise exact evidence.
    pub fn verify_dispatched_definite_integral(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        lower: &mathema::Rational,
        upper: &mathema::Rational,
        claim: &DispatchedDefiniteIntegral,
    ) -> Result<(), IntegrationDispatchError> {
        if claim.source != *source || &claim.variable != variable || claim.domain != domain {
            return Err(IntegrationDispatchError::SourceMismatch);
        }
        let polynomial = definite_polynomial(self, source, variable, domain)?;
        self.check_rational(lower)?;
        self.check_rational(upper)?;
        if claim.certificate.primitive().integrand() != &polynomial
            || claim.certificate.lower() != lower
            || claim.certificate.upper() != upper
        {
            return Err(IntegrationDispatchError::IncorrectEvidence);
        }
        self.verify_definite_polynomial_integral(&claim.certificate)?;
        Ok(())
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    #[test]
    fn changed_source_formula_and_term_proofs_are_rejected() {
        let ctx = EvaluationContext::default();
        let variable = Symbol::new("x").unwrap();
        let x = Expr::from(variable.clone());
        let source = DomainExpression::from_expr_in_domain(
            Expr::add([Expr::pow(x.clone(), Expr::from(2)), Expr::sin(x.clone())]),
            ScalarDomain::Real,
            256,
            32,
        )
        .unwrap();
        let certified = ctx
            .integrate_expression_dispatch(&source, &variable, ScalarDomain::Real)
            .unwrap();

        let mut wrong_primitive = certified.clone();
        wrong_primitive.primitive =
            DomainExpression::from_expr_in_domain(x.clone(), ScalarDomain::Real, 256, 32).unwrap();
        assert_eq!(
            ctx.verify_dispatched_antiderivative(
                &source,
                &variable,
                ScalarDomain::Real,
                &wrong_primitive,
            ),
            Err(IntegrationDispatchError::IncorrectPrimitive),
        );

        let mut wrong_source = certified.clone();
        wrong_source.source =
            DomainExpression::from_expr_in_domain(x, ScalarDomain::Real, 256, 32).unwrap();
        assert_eq!(
            ctx.verify_dispatched_antiderivative(
                &source,
                &variable,
                ScalarDomain::Real,
                &wrong_source,
            ),
            Err(IntegrationDispatchError::SourceMismatch),
        );

        let mut wrong_proofs = certified.clone();
        wrong_proofs.terms.swap(0, 1);
        assert_eq!(
            ctx.verify_dispatched_antiderivative(
                &source,
                &variable,
                ScalarDomain::Real,
                &wrong_proofs,
            ),
            Err(IntegrationDispatchError::IncorrectEvidence),
        );
    }
}
