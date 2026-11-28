//! Checked identities and derivatives of univariate rational expressions over R.
//!
//! Equality is directional: the candidate must be defined everywhere the source
//! is defined. A simpler candidate may fill holes syntactically, but the result
//! retains the source's exclusions. No new assumptions are inferred.

use std::{error::Error, fmt};

use mathema::{Condition, DomainExpression, Expr, ExprView, Integer, Rational, Symbol};

use crate::budget::{RationalBinaryOp, WorkBudget};
use crate::rational_function::condition_order;
use crate::{
    EvaluationContext, EvaluationError, RationalFunction, RationalFunctionError,
    RationalPolynomial, ResourceLimitKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalCalculusRelation {
    Identity,
    Derivative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalCalculusError {
    Evaluation(EvaluationError),
    UnsupportedSymbol,
    UnsupportedExponent,
    UnsupportedFunction,
    UnsupportedCondition,
    DomainMismatch,
    NameLimit,
    NewExclusion,
    IncorrectIdentity,
    IncorrectCertificate,
}

impl fmt::Display for RationalCalculusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::UnsupportedSymbol => f.write_str("only the selected variable is supported"),
            Self::UnsupportedExponent => f.write_str("only integer powers are supported"),
            Self::UnsupportedFunction => f.write_str("analytic functions are unsupported"),
            Self::UnsupportedCondition => {
                f.write_str("only rational nonzero conditions are supported")
            }
            Self::DomainMismatch => {
                f.write_str("calculus source was constructed in a different scalar domain")
            }
            Self::NameLimit => f.write_str("symbol name exceeds the admission limit"),
            Self::NewExclusion => {
                f.write_str("candidate domain is not established on the source domain")
            }
            Self::IncorrectIdentity => f.write_str("exact polynomial identity does not hold"),
            Self::IncorrectCertificate => f.write_str("incorrect rational calculus certificate"),
        }
    }
}

impl Error for RationalCalculusError {}

impl From<EvaluationError> for RationalCalculusError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

/// A proposed certificate is untrusted until checked against the original
/// source, candidate, variable and relation supplied independently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalCalculusCertificate {
    pub source: DomainExpression,
    pub candidate: DomainExpression,
    pub variable: Symbol,
    pub relation: RationalCalculusRelation,
    pub value: RationalFunction,
}

fn constant(value: impl Into<Rational>) -> RationalFunction {
    RationalFunction::from_polynomial(RationalPolynomial::new(vec![value.into()]))
}

struct Checker<'a> {
    context: &'a EvaluationContext,
    variable: &'a Symbol,
    work: WorkBudget,
    remaining_nodes: usize,
}

impl<'a> Checker<'a> {
    fn new(context: &'a EvaluationContext, variable: &'a Symbol) -> Self {
        Self {
            context,
            variable,
            work: WorkBudget::new(context.limits()),
            remaining_nodes: context.limits().max_expression_nodes,
        }
    }

    fn check_name(name: &Symbol) -> Result<(), RationalCalculusError> {
        if name.name().len() > 4096 {
            return Err(RationalCalculusError::NameLimit);
        }
        Ok(())
    }

    fn admit_expression(&mut self, expression: &Expr) -> Result<(), RationalCalculusError> {
        let limits = self.context.limits();
        let mut pending = vec![(expression, 1)];
        while let Some((node, depth)) = pending.pop() {
            self.remaining_nodes = self
                .remaining_nodes
                .checked_sub(1)
                .ok_or(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes))?;
            self.work.charge(1)?;
            if depth > limits.max_expression_depth.min(128) {
                return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionDepth).into());
            }
            match node.view() {
                ExprView::Integer(value) => {
                    if value.bit_len() > limits.max_scalar_bits {
                        return Err(EvaluationError::Limit(ResourceLimitKind::ScalarBits).into());
                    }
                }
                ExprView::Rational(value) => self.context.check_rational(value)?,
                ExprView::Symbol(name) => Self::check_name(name)?,
                ExprView::Function { name, argument } => {
                    Self::check_name(name)?;
                    pending.push((argument, depth + 1));
                }
                ExprView::Add(children) | ExprView::Mul(children) => {
                    if children.len() > self.remaining_nodes.saturating_sub(pending.len()) {
                        return Err(
                            EvaluationError::Limit(ResourceLimitKind::ExpressionNodes).into()
                        );
                    }
                    pending.extend(children.iter().map(|child| (child, depth + 1)));
                }
                ExprView::Pow { base, exponent } => {
                    pending.push((base, depth + 1));
                    pending.push((exponent, depth + 1));
                }
            }
            if pending.len() > self.remaining_nodes {
                return Err(EvaluationError::Limit(ResourceLimitKind::ExpressionNodes).into());
            }
        }
        Ok(())
    }

    fn admit(&mut self, input: &DomainExpression) -> Result<(), RationalCalculusError> {
        input
            .check_construction_domain(mathema::ScalarDomain::Real)
            .map_err(|_| RationalCalculusError::DomainMismatch)?;
        if input.conditions().len() > self.context.limits().max_rational_conditions {
            return Err(EvaluationError::Limit(ResourceLimitKind::RationalConditions).into());
        }
        self.admit_expression(input.expression())?;
        for condition in input.conditions() {
            self.admit_expression(condition.expression())?;
        }
        Ok(())
    }

    fn interpret(&mut self, expression: &Expr) -> Result<RationalFunction, RationalCalculusError> {
        use RationalBinaryOp::{Add, Divide, Multiply};

        self.work.charge(1)?;
        let value = match expression.view() {
            ExprView::Integer(value) => constant(value.clone()),
            ExprView::Rational(value) => constant(value.clone()),
            ExprView::Symbol(name) if name == self.variable => {
                RationalFunction::from_polynomial(RationalPolynomial::new(vec![
                    Rational::from(Integer::zero()),
                    Rational::from(Integer::one()),
                ]))
            }
            ExprView::Symbol(_) => return Err(RationalCalculusError::UnsupportedSymbol),
            ExprView::Function { .. } => return Err(RationalCalculusError::UnsupportedFunction),
            ExprView::Add(children) | ExprView::Mul(children) => {
                let sum = matches!(expression.view(), ExprView::Add(_));
                let (operation, initial) = if sum {
                    (Add, Integer::zero())
                } else {
                    (Multiply, Integer::one())
                };
                let mut value = constant(initial);
                for child in children {
                    let next = self.interpret(child)?;
                    value = self.work.rational_binary(&value, &next, operation)?;
                }
                value
            }
            ExprView::Pow { base, exponent } => {
                let ExprView::Integer(power) = exponent.view() else {
                    return Err(RationalCalculusError::UnsupportedExponent);
                };
                let mut count = power
                    .to_string()
                    .trim_start_matches('-')
                    .parse::<usize>()
                    .map_err(|_| EvaluationError::Limit(ResourceLimitKind::SymbolicPower))?;
                if count > self.context.limits().max_symbolic_power {
                    return Err(EvaluationError::Limit(ResourceLimitKind::SymbolicPower).into());
                }
                let mut factor = self.interpret(base)?;
                let one = constant(Integer::one());
                if count == 0 {
                    // The zero-power convention excludes a zero base.
                    let inverse = self.work.rational_binary(&one, &factor, Divide)?;
                    self.work.rational_binary(&factor, &inverse, Multiply)?
                } else {
                    let mut result = one.clone();
                    while count != 0 {
                        if count & 1 != 0 {
                            result = self.work.rational_binary(&result, &factor, Multiply)?;
                        }
                        count >>= 1;
                        if count != 0 {
                            factor = self.work.rational_binary(&factor, &factor, Multiply)?;
                        }
                    }
                    if power < &Integer::zero() {
                        self.work.rational_binary(&one, &result, Divide)?
                    } else {
                        result
                    }
                }
            }
        };
        self.context.check_rational_function(&value)?;
        Ok(value)
    }

    fn insert_exclusion(
        &mut self,
        exclusions: &mut Vec<RationalPolynomial>,
        polynomial: RationalPolynomial,
    ) -> Result<(), RationalCalculusError> {
        self.work.charge(1)?;
        if polynomial.is_zero() {
            return Err(
                EvaluationError::RationalFunction(RationalFunctionError::EmptyDomain).into(),
            );
        }
        if polynomial.degree() != Some(0) && !exclusions.contains(&polynomial) {
            if exclusions.len() >= self.context.limits().max_rational_conditions {
                return Err(EvaluationError::Limit(ResourceLimitKind::RationalConditions).into());
            }
            exclusions.push(polynomial);
        }
        Ok(())
    }

    fn normalize(
        &mut self,
        input: &DomainExpression,
    ) -> Result<RationalFunction, RationalCalculusError> {
        let value = self.interpret(input.expression())?;
        let mut exclusions = value.excluded_zero_sets().to_vec();
        for condition in input.conditions() {
            let Condition::NonZero(expression) = condition else {
                return Err(RationalCalculusError::UnsupportedCondition);
            };
            let restricted = self.interpret(expression)?;
            for exclusion in restricted.excluded_zero_sets() {
                self.insert_exclusion(&mut exclusions, exclusion.clone())?;
            }
            if restricted.numerator().is_zero() {
                return Err(
                    EvaluationError::RationalFunction(RationalFunctionError::EmptyDomain).into(),
                );
            }
            let monic = self
                .work
                .gcd(restricted.numerator(), restricted.numerator())?;
            self.insert_exclusion(&mut exclusions, monic)?;
        }
        exclusions.sort_by(condition_order);
        let normalized = RationalFunction::from_canonical(
            value.numerator().clone(),
            value.denominator().clone(),
            exclusions,
        );
        self.context.check_rational_function(&normalized)?;
        Ok(normalized)
    }

    // Removing common factors until only a constant remains proves containment
    // of the excluded complex zero sets, hence also of their real zero sets.
    // This deliberately refuses some real-only implications (e.g. x^2 + 1).
    fn check_domain(
        &mut self,
        source: &RationalFunction,
        candidate: &RationalFunction,
    ) -> Result<(), RationalCalculusError> {
        for restriction in candidate
            .excluded_zero_sets()
            .iter()
            .chain([candidate.denominator()])
        {
            let mut remaining = restriction.clone();
            for original in source
                .excluded_zero_sets()
                .iter()
                .chain([source.denominator()])
            {
                while remaining.degree().is_some_and(|degree| degree > 0) {
                    let common = self.work.gcd(&remaining, original)?;
                    if common.degree() == Some(0) {
                        break;
                    }
                    remaining = self.work.divide_exact(&remaining, &common)?;
                }
            }
            if remaining.degree().is_some_and(|degree| degree > 0) {
                return Err(RationalCalculusError::NewExclusion);
            }
        }
        Ok(())
    }

    fn check_value(
        &mut self,
        source: &RationalFunction,
        candidate: &RationalFunction,
        value: &RationalFunction,
        relation: RationalCalculusRelation,
    ) -> Result<(), RationalCalculusError> {
        self.context.check_rational_function(value)?;
        if value.excluded_zero_sets() != source.excluded_zero_sets() {
            return Err(RationalCalculusError::IncorrectCertificate);
        }
        self.check_domain(source, candidate)?;
        self.check_domain(source, value)?;

        // Verify the quotient rule by polynomial cross multiplication, without
        // invoking the rational derivative generator or trusting reduced output.
        let (numerator, denominator) = match relation {
            RationalCalculusRelation::Identity => {
                (source.numerator().clone(), source.denominator().clone())
            }
            RationalCalculusRelation::Derivative => {
                let dn = self.work.differentiate(source.numerator())?;
                let dd = self.work.differentiate(source.denominator())?;
                let left = self.work.multiply(&dn, source.denominator())?;
                let right = self.work.multiply(source.numerator(), &dd)?;
                (
                    self.work.subtract(&left, &right)?,
                    self.work
                        .multiply(source.denominator(), source.denominator())?,
                )
            }
        };
        for claim in [candidate, value] {
            let left = self.work.multiply(&numerator, claim.denominator())?;
            let right = self.work.multiply(&denominator, claim.numerator())?;
            if left != right {
                return Err(RationalCalculusError::IncorrectIdentity);
            }
        }
        Ok(())
    }
}

/// Checks a proposed identity or derivative in Q(x), interpreted over the reals.
/// Both expressions must use the selected variable, rational constants and
/// integer powers. The returned rational value retains the original domain,
/// including historical exclusions. Caller-provided construction history is
/// essential: restrictions erased before constructing DomainExpression cannot
/// be recovered. All normalization and proof arithmetic share one work budget.
///
/// A future general-power checker must bind the original base and exponent and
/// their independently checked derivatives, establish real differentiability
/// and a positive base throughout the admitted source domain, then check the
/// logarithmic derivative identity u^v * (v' * log(u) + v * u' / u)
/// without introducing new exclusions. Positivity implies nonzero; neither
/// is inferred from the presence of a log node. This extension requires W1's
/// domain-provenance support and an independent analytic identity checker.
/// Noninteger powers (including x^x), log and exp are not admitted here.
pub fn certify_rational_calculus(
    context: &EvaluationContext,
    source: &DomainExpression,
    candidate: &DomainExpression,
    variable: &Symbol,
    relation: RationalCalculusRelation,
) -> Result<RationalCalculusCertificate, RationalCalculusError> {
    let mut checker = Checker::new(context, variable);
    Checker::check_name(variable)?;
    checker.admit(source)?;
    checker.admit(candidate)?;
    let original = checker.normalize(source)?;
    let proposed = checker.normalize(candidate)?;
    let value = match relation {
        RationalCalculusRelation::Identity => original.clone(),
        RationalCalculusRelation::Derivative => checker.work.differentiate_rational(&original)?,
    };
    checker.check_value(&original, &proposed, &value, relation)?;
    Ok(RationalCalculusCertificate {
        source: source.clone(),
        candidate: candidate.clone(),
        variable: variable.clone(),
        relation,
        value,
    })
}

/// Independently checks the certificate against external source and candidate.
/// Does not call the certificate generator or the rational derivative generator.
pub fn verify_rational_calculus(
    context: &EvaluationContext,
    source: &DomainExpression,
    candidate: &DomainExpression,
    variable: &Symbol,
    relation: RationalCalculusRelation,
    certificate: &RationalCalculusCertificate,
) -> Result<(), RationalCalculusError> {
    let mut checker = Checker::new(context, variable);
    Checker::check_name(variable)?;
    Checker::check_name(&certificate.variable)?;
    checker.admit(source)?;
    checker.admit(candidate)?;
    // Structural equality is bounded by the already admitted external trees:
    // unequal shapes stop comparison, and equal subtrees have admitted size.
    if certificate.source != *source
        || certificate.candidate != *candidate
        || certificate.variable != *variable
        || certificate.relation != relation
    {
        return Err(RationalCalculusError::IncorrectCertificate);
    }
    let original = checker.normalize(source)?;
    let proposed = checker.normalize(candidate)?;
    checker.check_value(&original, &proposed, &certificate.value, relation)
}

impl Checker<'_> {
    fn polynomial_expression(
        &mut self,
        polynomial: &RationalPolynomial,
    ) -> Result<Expr, RationalCalculusError> {
        self.context.check_polynomial(polynomial)?;
        let variable = Expr::from(self.variable.clone());
        let mut terms = Vec::new();
        for (degree, coefficient) in polynomial.coefficients().iter().enumerate() {
            self.work.charge(4)?;
            if coefficient.numerator().is_zero() {
                continue;
            }
            let coefficient = Expr::from(coefficient.clone());
            let term = match degree {
                0 => coefficient,
                1 => Expr::mul([coefficient, variable.clone()]),
                _ => {
                    if degree > self.context.limits().max_symbolic_power {
                        return Err(EvaluationError::Limit(ResourceLimitKind::SymbolicPower).into());
                    }
                    Expr::mul([coefficient, Expr::pow(variable.clone(), Expr::from(degree))])
                }
            };
            terms.push(term);
        }
        let result = Expr::add(terms);
        self.context.check_expression(&result)?;
        Ok(result)
    }

    fn rational_expression(
        &mut self,
        value: &RationalFunction,
    ) -> Result<Expr, RationalCalculusError> {
        let numerator = self.polynomial_expression(value.numerator())?;
        let denominator = self.polynomial_expression(value.denominator())?;
        if denominator == Expr::from(1) {
            return Ok(numerator);
        }
        Ok(Expr::mul([
            numerator,
            Expr::pow(denominator, Expr::from(-1)),
        ]))
    }

    fn quotient_expression(
        &mut self,
        source: &RationalFunction,
    ) -> Result<Expr, RationalCalculusError> {
        let dn = self.work.differentiate(source.numerator())?;
        let dd = self.work.differentiate(source.denominator())?;
        let dn = self.polynomial_expression(&dn)?;
        let dd = self.polynomial_expression(&dd)?;
        let n = self.polynomial_expression(source.numerator())?;
        let d = self.polynomial_expression(source.denominator())?;
        if d == Expr::from(1) {
            return Ok(dn);
        }
        Ok(Expr::mul([
            Expr::add([
                Expr::mul([dn, d.clone()]),
                Expr::mul([Expr::from(-1), n, dd]),
            ]),
            Expr::pow(d, Expr::from(-2)),
        ]))
    }

    fn inherit(
        &self,
        source: &DomainExpression,
        expression: Expr,
    ) -> Result<DomainExpression, RationalCalculusError> {
        use mathema::DefinednessError;

        self.context.check_expression(&expression)?;
        let limits = self.context.limits();
        source
            .rebuild_with_expression(
                expression,
                limits.max_expression_nodes,
                limits.max_rational_conditions,
            )
            .map_err(|error| match error {
                DefinednessError::NodeLimitExceeded => {
                    EvaluationError::Limit(ResourceLimitKind::ExpressionNodes).into()
                }
                DefinednessError::ConditionLimitExceeded => {
                    EvaluationError::Limit(ResourceLimitKind::RationalConditions).into()
                }
                _ => RationalCalculusError::IncorrectCertificate,
            })
    }
}

impl EvaluationContext {
    /// Differentiates a bounded univariate rational source over the reals.
    ///
    /// The first certificate applies the quotient rule to the normalized
    /// source, with its original domain; the second checks the reduction of
    /// that actual expression. The first certificate's value is the reduced
    /// derivative with the original canonical exclusions. The second
    /// candidate is the simplified expression with inherited source history.
    pub fn differentiate_rational_domain_expression(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
    ) -> Result<[RationalCalculusCertificate; 2], RationalCalculusError> {
        let mut checker = Checker::new(self, variable);
        Checker::check_name(variable)?;
        checker.admit(source)?;
        let original = checker.normalize(source)?;
        let value = checker.work.differentiate_rational(&original)?;
        let raw = checker.quotient_expression(&original)?;
        let raw = checker.inherit(source, raw)?;
        checker.admit(&raw)?;
        let raw_value = checker.normalize(&raw)?;
        checker.check_value(
            &original,
            &raw_value,
            &value,
            RationalCalculusRelation::Derivative,
        )?;

        let reduced = checker.rational_expression(&value)?;
        let reduced = checker.inherit(&raw, reduced)?;
        checker.admit(&reduced)?;
        let reduced_value = checker.normalize(&reduced)?;
        checker.check_value(
            &raw_value,
            &reduced_value,
            &raw_value,
            RationalCalculusRelation::Identity,
        )?;

        Ok([
            RationalCalculusCertificate {
                source: source.clone(),
                candidate: raw.clone(),
                variable: variable.clone(),
                relation: RationalCalculusRelation::Derivative,
                value,
            },
            RationalCalculusCertificate {
                source: raw,
                candidate: reduced,
                variable: variable.clone(),
                relation: RationalCalculusRelation::Identity,
                value: raw_value,
            },
        ])
    }

    /// Checks the ordered quotient/reduction trace against an external source.
    /// In particular, a correct final field value alone cannot establish that
    /// the stated intermediate step or its inherited restrictions are valid.
    pub fn verify_rational_domain_derivative(
        &self,
        source: &DomainExpression,
        variable: &Symbol,
        trace: &[RationalCalculusCertificate; 2],
    ) -> Result<(), RationalCalculusError> {
        let [first, second] = trace;
        let mut checker = Checker::new(self, variable);
        Checker::check_name(variable)?;
        checker.admit(source)?;
        checker.admit(&first.candidate)?;
        checker.admit(&second.candidate)?;
        if first.source != *source
            || second.source != first.candidate
            || first.variable != *variable
            || second.variable != *variable
            || first.relation != RationalCalculusRelation::Derivative
            || second.relation != RationalCalculusRelation::Identity
        {
            return Err(RationalCalculusError::IncorrectCertificate);
        }

        let original = checker.normalize(source)?;
        let raw = checker.quotient_expression(&original)?;
        if first.candidate != checker.inherit(source, raw)? {
            return Err(RationalCalculusError::IncorrectCertificate);
        }
        let raw_value = checker.normalize(&first.candidate)?;
        checker.check_value(
            &original,
            &raw_value,
            &first.value,
            RationalCalculusRelation::Derivative,
        )?;

        let reduced = checker.rational_expression(&first.value)?;
        if second.candidate != checker.inherit(&first.candidate, reduced)? {
            return Err(RationalCalculusError::IncorrectCertificate);
        }
        let reduced_value = checker.normalize(&second.candidate)?;
        checker.check_value(
            &raw_value,
            &reduced_value,
            &second.value,
            RationalCalculusRelation::Identity,
        )
    }
}

#[cfg(test)]
mod derivative_trace_tests {
    use super::*;
    use crate::ResourceLimits;

    fn variable() -> Symbol {
        Symbol::new("x").unwrap()
    }

    fn x() -> Expr {
        Expr::from(variable())
    }

    fn shift(n: i32) -> Expr {
        Expr::add([x(), Expr::from(n)])
    }

    fn source(expression: Expr) -> DomainExpression {
        DomainExpression::from_expr(expression, 128).unwrap()
    }

    #[test]
    fn reciprocal_and_nonconstant_quotient_have_checked_steps() {
        let context = EvaluationContext::default();
        let reciprocal = source(Expr::pow(x(), Expr::from(-1)));
        let trace = context
            .differentiate_rational_domain_expression(&reciprocal, &variable())
            .unwrap();
        let expected = Expr::mul([
            Expr::from(-1),
            Expr::pow(Expr::pow(x(), Expr::from(2)), Expr::from(-1)),
        ]);
        assert_eq!(trace[1].candidate.expression(), &expected);
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(0))),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(-1))),
            Ok(Rational::from(Integer::from(-1)))
        );
        assert_eq!(
            context.verify_rational_domain_derivative(&reciprocal, &variable(), &trace),
            Ok(())
        );
        let quotient = source(Expr::mul([shift(1), Expr::pow(shift(-1), Expr::from(-1))]));
        let trace = context
            .differentiate_rational_domain_expression(&quotient, &variable())
            .unwrap();
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(2))),
            Ok(Rational::from(Integer::from(-2)))
        );
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(1))),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        assert_eq!(
            context.verify_rational_domain_derivative(&quotient, &variable(), &trace),
            Ok(())
        );
        assert_eq!(
            context.differentiate_rational_domain_expression(&quotient, &variable()),
            Ok(trace)
        );
    }

    #[test]
    fn canceled_factors_and_zero_products_retain_source_history() {
        let context = EvaluationContext::default();
        let canceled = source(Expr::mul([
            Expr::add([Expr::pow(x(), Expr::from(2)), Expr::from(-1)]),
            Expr::pow(shift(-1), Expr::from(-1)),
        ]));
        let trace = context
            .differentiate_rational_domain_expression(&canceled, &variable())
            .unwrap();
        assert_eq!(trace[1].candidate.expression(), &Expr::from(1));
        assert_eq!(trace[1].candidate.conditions(), canceled.conditions());
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(1))),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(0))),
            Ok(Rational::from(Integer::from(1)))
        );
        assert_eq!(
            context.verify_rational_domain_derivative(&canceled, &variable(), &trace),
            Ok(())
        );

        let zero = DomainExpression::multiply(
            [source(0.into()), source(Expr::pow(x(), Expr::from(-1)))],
            16,
        )
        .unwrap();
        let trace = context
            .differentiate_rational_domain_expression(&zero, &variable())
            .unwrap();
        assert_eq!(trace[1].candidate.expression(), &Expr::from(0));
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(0))),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        assert_eq!(
            context.verify_rational_domain_derivative(&zero, &variable(), &trace),
            Ok(())
        );
    }

    #[test]
    fn forged_steps_and_conditions_do_not_verify() {
        let context = EvaluationContext::default();
        let input = source(Expr::pow(x(), Expr::from(-1)));
        let trace = context
            .differentiate_rational_domain_expression(&input, &variable())
            .unwrap();

        let mut forged = trace.clone();
        forged.swap(0, 1);
        assert!(
            context
                .verify_rational_domain_derivative(&input, &variable(), &forged)
                .is_err()
        );
        let mut forged = trace.clone();
        forged[0].value = constant(99);
        assert!(
            context
                .verify_rational_domain_derivative(&input, &variable(), &forged)
                .is_err()
        );
        let mut forged = trace.clone();
        forged[1].candidate = source(1.into());
        assert!(
            context
                .verify_rational_domain_derivative(&input, &variable(), &forged)
                .is_err()
        );

        let mut forged = trace.clone();
        forged[1].candidate = DomainExpression::with_conditions(
            trace[1].candidate.expression().clone(),
            [Condition::NonZero(shift(-1))],
            128,
            16,
        )
        .unwrap();
        assert!(
            context
                .verify_rational_domain_derivative(&input, &variable(), &forged)
                .is_err()
        );
        let wrong_source = source(Expr::pow(x(), Expr::from(-2)));
        assert!(
            context
                .verify_rational_domain_derivative(&wrong_source, &variable(), &trace)
                .is_err()
        );
    }

    #[test]
    fn zero_power_and_unsupported_domains_and_limits() {
        let context = EvaluationContext::default();
        let zero_power = source(Expr::pow(x(), Expr::from(0)));
        let trace = context
            .differentiate_rational_domain_expression(&zero_power, &variable())
            .unwrap();
        assert_eq!(trace[1].candidate.expression(), &Expr::from(0));
        assert_eq!(
            trace[0].value.evaluate(&Rational::from(Integer::from(0))),
            Err(RationalFunctionError::UndefinedAtPoint)
        );
        let unsupported = source(Expr::from(Symbol::new("y").unwrap()));
        assert_eq!(
            context.differentiate_rational_domain_expression(&unsupported, &variable()),
            Err(RationalCalculusError::UnsupportedSymbol)
        );
        let complex =
            DomainExpression::from_expr_in_domain(x(), mathema::ScalarDomain::Complex, 128, 16)
                .unwrap();
        assert_eq!(
            context.differentiate_rational_domain_expression(&complex, &variable()),
            Err(RationalCalculusError::DomainMismatch)
        );
        let limited = EvaluationContext::new(ResourceLimits {
            max_work_units: 1,
            ..ResourceLimits::default()
        });
        assert!(matches!(
            limited.differentiate_rational_domain_expression(&zero_power, &variable()),
            Err(RationalCalculusError::Evaluation(EvaluationError::Limit(
                ResourceLimitKind::WorkUnits
            )))
        ));
        assert!(
            limited
                .verify_rational_domain_derivative(&zero_power, &variable(), &trace)
                .is_err()
        );
    }
}
