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
            let monic = self.work.gcd(restricted.numerator(), restricted.numerator())?;
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
                    self.work.multiply(source.denominator(), source.denominator())?,
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
