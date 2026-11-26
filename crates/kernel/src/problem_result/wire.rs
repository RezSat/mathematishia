//! Version 1 canonical JSON projection of typed requests and verified results.
//!
//! Wire values are data, never an authority to construct a checked result.

use super::*;
use crate::{ExactQuadraticRoot, RealBound};
use mathema::{ExprView, Integer};
use std::fmt;

const VERSION: usize = 1;
const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 128;
const MAX_SCALAR_BITS: u64 = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProblemWireError {
    OutputLimit,
    DepthLimit,
    ScalarLimit,
}

impl fmt::Display for ProblemWireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutputLimit => "problem JSON exceeds the output byte limit",
            Self::DepthLimit => "problem JSON exceeds the expression depth limit",
            Self::ScalarLimit => "problem JSON exceeds the scalar bit limit",
        })
    }
}

impl std::error::Error for ProblemWireError {}

type WireResult = Result<(), ProblemWireError>;

#[derive(Default)]
struct Encoder {
    text: String,
}

struct Object<'a> {
    encoder: &'a mut Encoder,
    first: bool,
}

impl Object<'_> {
    fn field(&mut self, name: &str, value: impl FnOnce(&mut Encoder) -> WireResult) -> WireResult {
        if !self.first {
            self.encoder.raw(",")?;
        }
        self.first = false;
        self.encoder.string(name)?;
        self.encoder.raw(":")?;
        value(self.encoder)
    }
}

macro_rules! object {
    ($writer:expr, $($name:literal => $value:expr),+ $(,)?) => {
        $writer.object(|obj| {
            $(obj.field($name, $value)?;)+
            Ok(())
        })
    };
}

impl Encoder {
    fn raw(&mut self, text: &str) -> WireResult {
        if text.len() > MAX_BYTES - self.text.len() {
            return Err(ProblemWireError::OutputLimit);
        }
        self.text.push_str(text);
        Ok(())
    }

    fn string(&mut self, text: &str) -> WireResult {
        self.raw("\"")?;
        for c in text.chars() {
            match c {
                '"' => self.raw("\\\"")?,
                '\\' => self.raw("\\\\")?,
                '\n' => self.raw("\\n")?,
                '\r' => self.raw("\\r")?,
                '\t' => self.raw("\\t")?,
                '\u{08}' => self.raw("\\b")?,
                '\u{0c}' => self.raw("\\f")?,
                c if c <= '\u{1f}' => self.raw(&format!("\\u{:04x}", u32::from(c)))?,
                c => self.raw(c.encode_utf8(&mut [0; 4]))?,
            }
        }
        self.raw("\"")
    }

    fn count(&mut self, value: usize) -> WireResult {
        self.raw(&value.to_string())
    }

    fn boolean(&mut self, value: bool) -> WireResult {
        self.raw(if value { "true" } else { "false" })
    }

    fn object(&mut self, fields: impl FnOnce(&mut Object<'_>) -> WireResult) -> WireResult {
        self.raw("{")?;
        fields(&mut Object {
            encoder: self,
            first: true,
        })?;
        self.raw("}")
    }

    fn array<T>(
        &mut self,
        values: &[T],
        mut item: impl FnMut(&mut Self, &T) -> WireResult,
    ) -> WireResult {
        self.raw("[")?;
        for (index, value) in values.iter().enumerate() {
            if index != 0 {
                self.raw(",")?;
            }
            item(self, value)?;
        }
        self.raw("]")
    }

    fn integer(&mut self, value: &Integer) -> WireResult {
        if value.bit_len() > MAX_SCALAR_BITS {
            return Err(ProblemWireError::ScalarLimit);
        }
        self.string(&value.to_string())
    }

    fn rational(&mut self, value: &Rational) -> WireResult {
        object!(self,
            "numerator" => |w| w.integer(value.numerator()),
            "denominator" => |w| w.integer(value.denominator()),
        )
    }

    fn expression(&mut self, value: &Expr, depth: usize) -> WireResult {
        if depth > MAX_DEPTH {
            return Err(ProblemWireError::DepthLimit);
        }
        match value.view() {
            ExprView::Integer(value) => object!(self,
                "kind" => |w| w.string("integer"),
                "value" => |w| w.integer(value),
            ),
            ExprView::Rational(value) => object!(self,
                "kind" => |w| w.string("rational"),
                "value" => |w| w.rational(value),
            ),
            ExprView::Symbol(value) => object!(self,
                "kind" => |w| w.string("symbol"),
                "name" => |w| w.string(value.name()),
            ),
            ExprView::Add(children) | ExprView::Mul(children) => object!(self,
                "kind" => |w| w.string(if matches!(value.view(), ExprView::Add(_)) { "add" } else { "mul" }),
                "children" => |w| w.array(children, |w, child| w.expression(child, depth + 1)),
            ),
            ExprView::Pow { base, exponent } => object!(self,
                "kind" => |w| w.string("pow"),
                "base" => |w| w.expression(base, depth + 1),
                "exponent" => |w| w.expression(exponent, depth + 1),
            ),
            ExprView::Function { name, argument } => object!(self,
                "kind" => |w| w.string("function"),
                "name" => |w| w.string(name.name()),
                "argument" => |w| w.expression(argument, depth + 1),
            ),
        }
    }

    fn condition(&mut self, value: &Condition) -> WireResult {
        let kind = match value {
            Condition::NonZero(_) => "nonzero",
            Condition::Positive(_) => "positive",
            Condition::Real(_) => "real",
            Condition::Integer(_) => "integer",
            Condition::InDomain { .. } => "in_domain",
        };
        self.object(|obj| {
            obj.field("kind", |w| w.string(kind))?;
            obj.field("expression", |w| w.expression(value.expression(), 0))?;
            if let Condition::InDomain { domain, .. } = value {
                obj.field("domain", |w| w.string(domain_id(*domain)))?;
            }
            Ok(())
        })
    }

    fn domain_expression(&mut self, value: &DomainExpression) -> WireResult {
        self.object(|obj| {
            obj.field("expression", |w| w.expression(value.expression(), 0))?;
            obj.field("conditions", |w| {
                w.array(value.conditions(), Self::condition)
            })?;
            if let Some(domain) = value.construction_domain() {
                obj.field("construction_domain", |w| w.string(domain_id(domain)))?;
            }
            Ok(())
        })
    }

    fn polynomial(&mut self, value: &RationalPolynomial) -> WireResult {
        self.array(value.coefficients(), Self::rational)
    }

    fn rational_function(&mut self, value: &RationalFunction) -> WireResult {
        object!(self,
            "numerator" => |w| w.polynomial(value.numerator()),
            "denominator" => |w| w.polynomial(value.denominator()),
            "excluded_zero_sets" => |w| w.array(value.excluded_zero_sets(), Self::polynomial),
        )
    }

    fn statement(&mut self, value: &ProblemStatement) -> WireResult {
        match value {
            ProblemStatement::Equation(eq) => object!(self,
                "kind" => |w| w.string("equation"),
                "left" => |w| w.expression(eq.left(), 0),
                "right" => |w| w.expression(eq.right(), 0),
            ),
            ProblemStatement::PolynomialZero(poly) => object!(self,
                "kind" => |w| w.string("polynomial_zero"),
                "polynomial" => |w| w.polynomial(poly),
            ),
            ProblemStatement::RationalEquation { left, right } => object!(self,
                "kind" => |w| w.string("rational_equation"),
                "left" => |w| w.rational_function(left),
                "right" => |w| w.rational_function(right),
            ),
            ProblemStatement::RationalEvaluation { function, at } => object!(self,
                "kind" => |w| w.string("rational_evaluation"),
                "function" => |w| w.rational_function(function),
                "at" => |w| w.rational(at),
            ),
            ProblemStatement::Expression(source) => object!(self,
                "kind" => |w| w.string("expression"),
                "source" => |w| w.domain_expression(source),
            ),
            ProblemStatement::Limit {
                source,
                at,
                direction,
            } => object!(self,
                "kind" => |w| w.string("limit"),
                "source" => |w| w.domain_expression(source),
                "at" => |w| w.rational(at),
                "direction" => |w| w.string(direction_id(*direction)),
            ),
            ProblemStatement::SourceEquation { left, right } => object!(self,
                "kind" => |w| w.string("source_equation"),
                "left" => |w| w.domain_expression(left),
                "right" => |w| w.domain_expression(right),
            ),
            ProblemStatement::Comparison {
                left,
                right,
                operator,
            } => object!(self,
                "kind" => |w| w.string("comparison"),
                "left" => |w| w.domain_expression(left),
                "right" => |w| w.domain_expression(right),
                "operator" => |w| w.string(comparison_id(*operator)),
            ),
        }
    }

    fn request(&mut self, request: &ProblemRequest) -> WireResult {
        object!(self,
            "operation" => |w| w.string(request.operation.stable_id()),
            "statement" => |w| w.statement(&request.statement),
            "variables" => |w| w.array(&request.variables, |w, symbol| w.string(symbol.name())),
            "domain" => |w| w.string(domain_id(request.domain)),
        )
    }

    fn root(&mut self, root: &ExactQuadraticRoot) -> WireResult {
        match root {
            ExactQuadraticRoot::Rational(value) => object!(self,
                "kind" => |w| w.string("rational"),
                "value" => |w| w.rational(value),
            ),
            ExactQuadraticRoot::Algebraic {
                minimal_polynomial,
                branch,
            } => object!(self,
                "kind" => |w| w.string("algebraic"),
                "minimal_polynomial" => |w| w.polynomial(minimal_polynomial),
                "branch" => |w| w.string(branch.stable_id()),
            ),
        }
    }

    fn root_multiplicity(&mut self, value: &PolynomialRootMultiplicity) -> WireResult {
        object!(self,
            "root" => |w| w.root(value.root()),
            "multiplicity" => |w| w.count(value.multiplicity()),
        )
    }

    fn bound(&mut self, value: &RealBound) -> WireResult {
        match value {
            RealBound::NegativeInfinity => {
                object!(self, "kind" => |w| w.string("negative_infinity"))
            }
            RealBound::PositiveInfinity => {
                object!(self, "kind" => |w| w.string("positive_infinity"))
            }
            RealBound::Finite(value) => object!(self,
                "kind" => |w| w.string("finite"),
                "value" => |w| w.root(value),
            ),
        }
    }

    fn interval(&mut self, value: &RealInterval) -> WireResult {
        object!(self,
            "lower" => |w| w.bound(&value.lower),
            "upper" => |w| w.bound(&value.upper),
            "lower_closed" => |w| w.boolean(value.lower_closed),
            "upper_closed" => |w| w.boolean(value.upper_closed),
        )
    }

    fn limit_value(&mut self, value: &LimitValue) -> WireResult {
        match value {
            LimitValue::Finite(value) => object!(self,
                "kind" => |w| w.string("finite"),
                "value" => |w| w.rational(value),
            ),
            LimitValue::PositiveInfinity => {
                object!(self, "kind" => |w| w.string("positive_infinity"))
            }
            LimitValue::NegativeInfinity => {
                object!(self, "kind" => |w| w.string("negative_infinity"))
            }
            LimitValue::DoesNotExist => object!(self, "kind" => |w| w.string("does_not_exist")),
        }
    }

    fn answer(&mut self, value: &ExactAnswer) -> WireResult {
        match value {
            ExactAnswer::RationalRoots(values) => object!(self,
                "kind" => |w| w.string("rational_roots"),
                "values" => |w| w.array(values, Self::rational),
            ),
            ExactAnswer::ComplexRoots(values) => object!(self,
                "kind" => |w| w.string("complex_roots"),
                "roots" => |w| w.array(values, Self::root_multiplicity),
            ),
            ExactAnswer::Expression(value) => object!(self,
                "kind" => |w| w.string("expression"),
                "expression" => |w| w.expression(value, 0),
            ),
            ExactAnswer::Rational(value) => object!(self,
                "kind" => |w| w.string("rational"),
                "value" => |w| w.rational(value),
            ),
            ExactAnswer::Limit(value) => object!(self,
                "kind" => |w| w.string("limit"),
                "value" => |w| w.limit_value(value),
            ),
            ExactAnswer::Intervals(values) => object!(self,
                "kind" => |w| w.string("intervals"),
                "intervals" => |w| w.array(values, Self::interval),
            ),
            ExactAnswer::AllAdmissibleValues => {
                object!(self, "kind" => |w| w.string("all_admissible"))
            }
        }
    }

    fn outcome(&mut self, value: &ProblemOutcome) -> WireResult {
        match value {
            ProblemOutcome::SolvedExact(answer) | ProblemOutcome::SolvedConditional(answer) => {
                object!(self,
                    "status" => |w| w.string(if matches!(value, ProblemOutcome::SolvedExact(_)) { "complete" } else { "conditional" }),
                    "answer" => |w| w.answer(answer),
                )
            }
            ProblemOutcome::Partial { known, unresolved } => object!(self,
                "status" => |w| w.string("partial"),
                "answer" => |w| w.answer(known),
                "unresolved" => |w| w.polynomial(unresolved),
            ),
            ProblemOutcome::PartialScope { known, reason } => object!(self,
                "status" => |w| w.string("partial"),
                "answer" => |w| w.answer(known),
                "reason" => |w| w.string(unsupported_id(*reason)),
            ),
            ProblemOutcome::Unsupported(reason) => object!(self,
                "status" => |w| w.string("unsupported"),
                "reason" => |w| w.string(unsupported_id(*reason)),
            ),
            ProblemOutcome::NoSolutions => object!(self, "status" => |w| w.string("no_solutions")),
            ProblemOutcome::AllAdmissibleValues => {
                object!(self, "status" => |w| w.string("all_admissible"))
            }
            ProblemOutcome::Undefined(reason) => object!(self,
                "status" => |w| w.string("undefined"),
                "reason" => |w| w.string(undefined_id(*reason)),
            ),
            ProblemOutcome::ResourceRefusal(reason) => object!(self,
                "status" => |w| w.string("resource_refusal"),
                "reason" => |w| w.string(resource_id(*reason)),
            ),
        }
    }

    fn restriction(&mut self, value: &ProblemRestriction) -> WireResult {
        match value {
            ProblemRestriction::Condition(value) => object!(self,
                "kind" => |w| w.string("condition"),
                "condition" => |w| w.condition(value),
            ),
            ProblemRestriction::PolynomialNonZero(value) => object!(self,
                "kind" => |w| w.string("polynomial_nonzero"),
                "polynomial" => |w| w.polynomial(value),
            ),
        }
    }

    fn step(&mut self, value: &StepReference) -> WireResult {
        object!(self,
            "index" => |w| w.count(value.index),
            "rule_id" => |w| w.string(value.rule_id),
        )
    }

    fn evidence(&mut self, value: &ProblemResult) -> WireResult {
        match value.checked_by() {
            Some(proof) => object!(self,
                "checked_by" => |w| w.string(proof_id(proof)),
                "steps" => |w| w.array(value.steps(), Self::step),
            ),
            None => self.raw("null"),
        }
    }
}

fn domain_id(value: ScalarDomain) -> &'static str {
    match value {
        ScalarDomain::Integer => "Z",
        ScalarDomain::Rational => "Q",
        ScalarDomain::Real => "R",
        ScalarDomain::Complex => "C",
    }
}

fn direction_id(value: LimitDirection) -> &'static str {
    match value {
        LimitDirection::Left => "left",
        LimitDirection::Right => "right",
        LimitDirection::TwoSided => "two-sided",
    }
}

fn comparison_id(value: ComparisonOperator) -> &'static str {
    match value {
        ComparisonOperator::Less => "less",
        ComparisonOperator::LessOrEqual => "less_or_equal",
        ComparisonOperator::Greater => "greater",
        ComparisonOperator::GreaterOrEqual => "greater_or_equal",
    }
}

fn unsupported_id(value: UnsupportedReason) -> &'static str {
    match value {
        UnsupportedReason::Domain => "domain",
        UnsupportedReason::SeveralVariables => "several_variables",
        UnsupportedReason::AlgorithmNotAvailable => "algorithm_not_available",
        UnsupportedReason::ExpressionOutsideAlgorithm => "expression_outside_algorithm",
        UnsupportedReason::DegreeOutsideAlgorithm => "degree_outside_algorithm",
        UnsupportedReason::RewriteStepLimit => "rewrite_step_limit",
    }
}

fn undefined_id(value: UndefinedReason) -> &'static str {
    match value {
        UndefinedReason::ExcludedPoint => "excluded_point",
        UndefinedReason::ContradictedDomainCondition => "contradicted_domain_condition",
    }
}

fn resource_id(value: ResourceLimitKind) -> &'static str {
    match value {
        ResourceLimitKind::ExpressionNodes => "expression_nodes",
        ResourceLimitKind::ExpressionDepth => "expression_depth",
        ResourceLimitKind::ScalarBits => "scalar_bits",
        ResourceLimitKind::PolynomialDegree => "polynomial_degree",
        ResourceLimitKind::DistributionSteps => "distribution_steps",
        ResourceLimitKind::WorkUnits => "work_units",
        ResourceLimitKind::IntermediateScalarBits => "intermediate_scalar_bits",
        ResourceLimitKind::SymbolicPower => "symbolic_power",
        ResourceLimitKind::RationalConditions => "rational_conditions",
    }
}

fn proof_id(value: CheckedBy) -> &'static str {
    match value {
        CheckedBy::LinearEquivalence => "linear_equivalence",
        CheckedBy::QuadraticRootCriterion => "quadratic_root_criterion",
        CheckedBy::RationalEquationDomain => "rational_equation_domain",
        CheckedBy::PartialComplexFactorization => "partial_complex_factorization",
        CheckedBy::SourceBoundPolynomial => "source_bound_polynomial",
        CheckedBy::SymbolicDerivativeRules => "symbolic_derivative_rules",
        CheckedBy::RationalEvaluation => "rational_evaluation",
        CheckedBy::IntegrationTerms => "integration_terms",
        CheckedBy::PolynomialIdentity => "polynomial_identity",
        CheckedBy::RationalLimitFactorization => "rational_limit_factorization",
        CheckedBy::RationalInequalitySigns => "rational_inequality_signs",
        CheckedBy::RationalQuadraticEquation => "rational_quadratic_equation",
        CheckedBy::AnalyticConstantRules => "analytic_constant_rules",
        CheckedBy::RationalLiteral => "rational_literal",
    }
}

impl ProblemRequest {
    /// Canonical JSON envelope of the typed input, not a serialized proof.
    pub fn to_json(&self) -> Result<String, ProblemWireError> {
        let mut writer = Encoder::default();
        object!(writer,
            "schema_version" => |w| w.count(VERSION),
            "request" => |w| w.request(self),
        )?;
        Ok(writer.text)
    }
}

impl ProblemResult {
    /// Versioned deterministic JSON. Exact values are never converted to floats.
    pub fn to_json(&self) -> Result<String, ProblemWireError> {
        let mut writer = Encoder::default();
        object!(writer,
            "schema_version" => |w| w.count(VERSION),
            "request" => |w| w.request(self.request()),
            "result_domain" => |w| w.string(domain_id(self.result_domain())),
            "outcome" => |w| w.outcome(self.outcome()),
            "restrictions" => |w| w.array(self.restrictions(), Encoder::restriction),
            "evidence" => |w| w.evidence(self),
        )?;
        Ok(writer.text)
    }
}
