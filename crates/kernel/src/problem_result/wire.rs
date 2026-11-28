//! Version 1 canonical JSON projection of typed requests and verified results.
//!
//! Wire values are data, never an authority to construct a checked result.

use super::*;
use crate::step_records::{
    StepClaim, StepConclusion, StepCondition, StepEvidence, StepEvidenceSource, StepLimits,
    StepRecord, StepRecordError, StepRecords, verify_step_records,
};
use crate::{
    AbsoluteAffine, AbsoluteCase, AbsoluteExpression, AbsoluteInterval, AbsoluteSign,
    ExactQuadraticRoot, GroebnerPair, GroebnerReduction, MultivariatePolynomial, RealBound,
    SpecialAngleOutcome, VerifiedCondition,
};
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
    StepVerification(StepRecordError),
}

impl fmt::Display for ProblemWireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutputLimit => "problem JSON exceeds the output byte limit",
            Self::DepthLimit => "problem JSON exceeds the expression depth limit",
            Self::ScalarLimit => "problem JSON exceeds the scalar bit limit",
            Self::StepVerification(error) => return write!(f, "{error}"),
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

    fn absolute_affine(&mut self, value: &AbsoluteAffine) -> WireResult {
        object!(self,
            "slope" => |w| w.rational(&value.slope),
            "constant" => |w| w.rational(&value.constant),
        )
    }

    fn absolute_expression(&mut self, value: &AbsoluteExpression) -> WireResult {
        object!(self,
            "variable" => |w| w.string(value.variable.name()),
            "base" => |w| w.absolute_affine(&value.base),
            "layers" => |w| w.array(&value.layers, Self::absolute_affine),
        )
    }

    fn absolute_interval(&mut self, value: &AbsoluteInterval) -> WireResult {
        object!(self,
            "lower" => |w| match &value.lower {
                Some(lower) => w.rational(lower),
                None => w.raw("null"),
            },
            "upper" => |w| match &value.upper {
                Some(upper) => w.rational(upper),
                None => w.raw("null"),
            },
        )
    }

    fn absolute_case(&mut self, value: &AbsoluteCase) -> WireResult {
        object!(self,
            "interval" => |w| w.absolute_interval(&value.interval),
            "signs" => |w| w.array(&value.signs, |w, sign| w.string(match sign {
                AbsoluteSign::Nonnegative => "nonnegative",
                AbsoluteSign::Nonpositive => "nonpositive",
            })),
            "solution" => |w| match &value.solution {
                Some(solution) => w.absolute_interval(solution),
                None => w.raw("null"),
            },
        )
    }

    fn special_angle(&mut self, value: &SpecialAngleCertificate) -> WireResult {
        object!(self,
            "pi_multiple" => |w| w.rational(value.angle.coefficient()),
            "function" => |w| w.string(trig_id(value.function)),
            "domain" => |w| w.string(domain_id(value.domain)),
            "tick" => |w| w.count(usize::from(value.tick)),
            "outcome" => |w| match &value.outcome {
                SpecialAngleOutcome::Exact(root) => object!(w,
                    "kind" => |w| w.string("exact"),
                    "value" => |w| w.root(root),
                ),
                SpecialAngleOutcome::UndefinedTangentPole => object!(w,
                    "kind" => |w| w.string("undefined_tangent_pole"),
                ),
            },
        )
    }

    fn multivariate(&mut self, value: &MultivariatePolynomial) -> WireResult {
        object!(self,
            "variables" => |w| w.array(value.variables(), |w, symbol| w.string(symbol.name())),
            "terms" => |w| {
                w.raw("[")?;
                for (index, (powers, coefficient)) in value.terms().iter().enumerate() {
                    if index != 0 {
                        w.raw(",")?;
                    }
                    object!(w,
                        "powers" => |w| w.array(powers, |w, power| w.count(*power as usize)),
                        "coefficient" => |w| w.rational(coefficient),
                    )?;
                }
                w.raw("]")
            },
        )
    }

    fn reduction(&mut self, value: &GroebnerReduction) -> WireResult {
        object!(self,
            "quotients" => |w| w.array(&value.quotients, Self::multivariate),
            "remainder" => |w| w.multivariate(&value.remainder),
            "steps" => |w| w.count(value.steps),
        )
    }

    fn groebner_pair(&mut self, value: &GroebnerPair) -> WireResult {
        object!(self,
            "left" => |w| w.count(value.left),
            "right" => |w| w.count(value.right),
            "lcm" => |w| w.array(&value.lcm, |w, power| w.count(*power as usize)),
            "reduction" => |w| w.reduction(&value.reduction),
        )
    }

    fn groebner(&mut self, value: &GroebnerCertificate) -> WireResult {
        object!(self,
            "variables" => |w| w.array(&value.variables, |w, symbol| w.string(symbol.name())),
            "basis" => |w| w.array(&value.basis, Self::multivariate),
            "representations" => |w| w.array(&value.representations, |w, row| {
                w.array(row, Self::multivariate)
            }),
            "generator_reductions" => |w| w.array(&value.generator_reductions, Self::reduction),
            "pairs" => |w| w.array(&value.pairs, Self::groebner_pair),
        )
    }

    fn statement(&mut self, value: &ProblemStatement) -> WireResult {
        match value {
            ProblemStatement::Absolute(equation) => object!(self,
                "kind" => |w| w.string("absolute_affine"),
                "domain" => |w| w.string(domain_id(equation.domain)),
                "left" => |w| w.absolute_expression(&equation.left),
                "right" => |w| w.absolute_expression(&equation.right),
            ),
            ProblemStatement::SpecialAngle { angle, function } => object!(self,
                "kind" => |w| w.string("special_angle"),
                "pi_multiple" => |w| w.rational(angle.coefficient()),
                "function" => |w| w.string(trig_id(*function)),
            ),
            ProblemStatement::TrigZeros(function) => object!(self,
                "kind" => |w| w.string("trig_zeros"),
                "function" => |w| w.string(trig_id(*function)),
            ),
            ProblemStatement::Ideal { generators, target } => object!(self,
                "kind" => |w| w.string("ideal"),
                "generators" => |w| w.array(generators, Self::domain_expression),
                "target" => |w| match target {
                    Some(target) => w.domain_expression(target),
                    None => w.raw("null"),
                },
            ),
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
            ExactAnswer::Absolute(value) => object!(self,
                "kind" => |w| w.string("absolute_solution_union"),
                "solutions" => |w| w.array(&value.solutions, Self::absolute_interval),
                "cases" => |w| w.array(&value.cases, Self::absolute_case),
            ),
            ExactAnswer::SpecialAngle(value) => object!(self,
                "kind" => |w| w.string("special_angle"),
                "certificate" => |w| w.special_angle(value),
            ),
            ExactAnswer::TrigZeros(values) => object!(self,
                "kind" => |w| w.string("trig_zeros"),
                "certificates" => |w| w.array(values, Self::special_angle),
            ),
            ExactAnswer::Groebner(value) => object!(self,
                "kind" => |w| w.string("groebner_basis"),
                "certificate" => |w| w.groebner(value),
            ),
            ExactAnswer::Membership { basis, membership } => {
                let (member, reduction) = match membership.as_ref() {
                    IdealMembership::Member(reduction) => (true, reduction),
                    IdealMembership::NonMember(reduction) => (false, reduction),
                };
                object!(self,
                    "kind" => |w| w.string("ideal_membership"),
                    "member" => |w| w.boolean(member),
                    "basis_certificate" => |w| w.groebner(basis),
                    "reduction" => |w| w.reduction(reduction),
                )
            }
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
            Some(proof) => self.object(|obj| {
                obj.field("checked_by", |w| w.string(proof_id(proof)))?;
                obj.field("steps", |w| w.array(value.steps(), Self::step))?;
                if let Some(detail) = value.evidence_detail() {
                    obj.field("detail", |w| w.answer(detail))?;
                }
                Ok(())
            }),
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

fn trig_id(value: TrigFunction) -> &'static str {
    match value {
        TrigFunction::Sin => "sin",
        TrigFunction::Cos => "cos",
        TrigFunction::Tan => "tan",
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
        UnsupportedReason::PeriodicCompletenessUnavailable => "periodic_completeness_unavailable",
    }
}

fn undefined_id(value: UndefinedReason) -> &'static str {
    match value {
        UndefinedReason::ExcludedPoint => "excluded_point",
        UndefinedReason::TangentPole => "tangent_pole",
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
        CheckedBy::SourceBoundRationalEquation => "source_bound_rational_equation",
        CheckedBy::SymbolicDerivativeRules => "symbolic_derivative_rules",
        CheckedBy::RationalEvaluation => "rational_evaluation",
        CheckedBy::IntegrationTerms => "integration_terms",
        CheckedBy::PolynomialIdentity => "polynomial_identity",
        CheckedBy::RationalLimitFactorization => "rational_limit_factorization",
        CheckedBy::RationalInequalitySigns => "rational_inequality_signs",
        CheckedBy::RationalQuadraticEquation => "rational_quadratic_equation",
        CheckedBy::AnalyticConstantRules => "analytic_constant_rules",
        CheckedBy::RationalLiteral => "rational_literal",
        CheckedBy::AbsoluteSignCases => "absolute_sign_cases",
        CheckedBy::SpecialAngleIdentity => "special_angle_identity",
        CheckedBy::TrigZeroSubstitution => "trig_zero_substitution",
        CheckedBy::GroebnerCriterion => "groebner_criterion",
        CheckedBy::IdealReduction => "ideal_reduction",
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

impl Encoder {
    fn equation(&mut self, value: &Equation) -> WireResult {
        object!(self,
            "left" => |w| w.expression(value.left(), 0),
            "right" => |w| w.expression(value.right(), 0),
        )
    }

    fn step_claim(&mut self, value: &StepClaim) -> WireResult {
        match value {
            StepClaim::Equal { before, after } => object!(self,
                "kind" => |w| w.string("equal"),
                "before" => |w| w.expression(before, 0),
                "after" => |w| w.expression(after, 0),
            ),
            StepClaim::Equivalent {
                before,
                after,
                variable,
            } => object!(self,
                "kind" => |w| w.string("equivalent"),
                "before" => |w| w.equation(before),
                "after" => |w| w.equation(after),
                "variable" => |w| w.string(variable.name()),
            ),
            StepClaim::DerivativeOf {
                before,
                after,
                variable,
            } => object!(self,
                "kind" => |w| w.string("derivative_of"),
                "before" => |w| w.expression(before, 0),
                "after" => |w| w.expression(after, 0),
                "variable" => |w| w.string(variable.name()),
            ),
            StepClaim::PolynomialDivision {
                dividend,
                divisor,
                quotient,
                remainder,
            } => object!(self,
                "kind" => |w| w.string("polynomial_division"),
                "dividend" => |w| w.polynomial(dividend),
                "divisor" => |w| w.polynomial(divisor),
                "quotient" => |w| w.polynomial(quotient),
                "remainder" => |w| w.polynomial(remainder),
            ),
            StepClaim::MultivariateDivision {
                dividend,
                ordered_divisors,
                quotients,
                remainder,
            } => object!(self,
                "kind" => |w| w.string("multivariate_division"),
                "dividend" => |w| w.multivariate(dividend),
                "ordered_divisors" => |w| w.array(ordered_divisors, Self::multivariate),
                "quotients" => |w| w.array(quotients, Self::multivariate),
                "remainder" => |w| w.multivariate(remainder),
            ),
        }
    }

    fn step_condition(&mut self, value: &StepCondition) -> WireResult {
        match value {
            StepCondition::Source(index) => object!(self,
                "kind" => |w| w.string("source"),
                "index" => |w| w.count(*index),
            ),
            StepCondition::Proven(VerifiedCondition::Nonzero(value)) => object!(self,
                "kind" => |w| w.string("proven_nonzero"),
                "value" => |w| w.rational(value),
            ),
        }
    }

    fn step_source(&mut self, value: StepEvidenceSource) -> WireResult {
        let (kind, index) = match value {
            StepEvidenceSource::Distribution(index) => ("distribution", Some(index)),
            StepEvidenceSource::Linear(index) => ("linear", Some(index)),
            StepEvidenceSource::Derivative(index) => ("derivative", Some(index)),
            StepEvidenceSource::PolynomialDivisionFinal => ("polynomial_division_final", None),
            StepEvidenceSource::MultivariateDivisionFinal => ("multivariate_division_final", None),
        };
        self.object(|obj| {
            obj.field("kind", |w| w.string(kind))?;
            if let Some(index) = index {
                obj.field("index", |w| w.count(index))?;
            }
            Ok(())
        })
    }

    fn step_record(&mut self, value: &StepRecord) -> WireResult {
        object!(self,
            "index" => |w| w.count(value.index),
            "rule_id" => |w| w.string(value.rule.stable_id()),
            "claim" => |w| w.step_claim(&value.claim),
            "premises" => |w| w.array(&value.premises, |w, index| w.count(*index)),
            "conditions" => |w| w.array(&value.conditions, Self::step_condition),
            "evidence" => |w| w.step_source(value.evidence),
        )
    }

    fn step_conclusion(&mut self, value: &StepConclusion) -> WireResult {
        match value {
            StepConclusion::Distribution {
                original,
                result,
                complete,
            } => object!(self,
                "kind" => |w| w.string("distribution"),
                "original" => |w| w.expression(original, 0),
                "result" => |w| w.expression(result, 0),
                "complete" => |w| w.boolean(*complete),
            ),
            StepConclusion::Linear {
                original,
                variable,
                solution,
            } => object!(self,
                "kind" => |w| w.string("linear"),
                "original" => |w| w.equation(original),
                "variable" => |w| w.string(variable.name()),
                "solution" => |w| match solution {
                    LinearSolution::Unique(root) => object!(w,
                        "kind" => |w| w.string("unique"),
                        "value" => |w| w.rational(root),
                    ),
                    LinearSolution::NoSolution => object!(w, "kind" => |w| w.string("no_solution")),
                    LinearSolution::AllRationals => object!(w, "kind" => |w| w.string("all_rationals")),
                },
            ),
            StepConclusion::Derivative {
                original,
                variable,
                result,
            } => object!(self,
                "kind" => |w| w.string("derivative"),
                "original" => |w| w.expression(original, 0),
                "variable" => |w| w.string(variable.name()),
                "result" => |w| w.expression(result, 0),
            ),
            StepConclusion::FinalDivision => {
                object!(self, "kind" => |w| w.string("final_division"))
            }
        }
    }
}

impl StepRecords {
    /// Projects only records verified against the supplied native evidence.
    /// The output is not a standalone proof or a trusted deserialization format.
    pub fn to_checked_json(
        &self,
        evidence: StepEvidence<'_>,
        limits: StepLimits,
    ) -> Result<String, ProblemWireError> {
        verify_step_records(evidence, self, limits).map_err(ProblemWireError::StepVerification)?;
        let mut writer = Encoder::default();
        object!(writer,
            "schema_version" => |w| w.count(VERSION),
            "kind" => |w| w.string("checked_step_records"),
            "domain" => |w| w.string(domain_id(self.domain)),
            "source_conditions" => |w| w.array(&self.source_conditions, Encoder::condition),
            "steps" => |w| w.array(&self.steps, Encoder::step_record),
            "conclusion" => |w| w.step_conclusion(&self.conclusion),
        )?;
        Ok(writer.text)
    }
}

impl crate::step_records::division_trace::OrderedDivisionTrace {
    /// Rechecks the original ordered division before exposing actual transitions.
    pub fn to_checked_json(
        &self,
        dividend: &MultivariatePolynomial,
        divisors: &[MultivariatePolynomial],
        limits: StepLimits,
    ) -> Result<String, ProblemWireError> {
        use crate::step_records::division_trace::{DivisionReductionRule, verify_ordered_division};
        verify_ordered_division(dividend, divisors, self, limits)
            .map_err(ProblemWireError::StepVerification)?;
        let mut writer = Encoder::default();
        object!(writer,
            "schema_version" => |w| w.count(VERSION),
            "kind" => |w| w.string("checked_ordered_division"),
            "domain" => |w| w.string("Q"),
            "dividend" => |w| w.multivariate(dividend),
            "ordered_divisors" => |w| w.array(divisors, Encoder::multivariate),
            "steps" => |w| w.array(&self.steps, |w, step| {
                object!(w,
                    "index" => |w| w.count(step.index),
                    "premise" => |w| match step.premise {
                        Some(index) => w.count(index),
                        None => w.raw("null"),
                    },
                    "rule_id" => |w| w.string(step.rule.stable_id()),
                    "divisor_index" => |w| match step.rule {
                        DivisionReductionRule::Reduce { divisor_index } => w.count(divisor_index),
                        DivisionReductionRule::MoveToRemainder => w.raw("null"),
                    },
                    "before" => |w| w.multivariate(&step.before),
                    "after" => |w| w.multivariate(&step.after),
                    "factor" => |w| w.multivariate(&step.factor),
                )
            }),
            "quotients" => |w| w.array(&self.quotients, Encoder::multivariate),
            "remainder" => |w| w.multivariate(&self.remainder),
        )?;
        Ok(writer.text)
    }
}

impl super::methods::MethodEnvelope {
    /// Separate version-1 envelope. The legacy result JSON is unchanged.
    pub fn to_json(&self) -> Result<String, ProblemWireError> {
        let mut writer = Encoder::default();
        writer.method_envelope(self)?;
        Ok(writer.text)
    }
}

impl Encoder {
    fn method_envelope(&mut self, value: &super::methods::MethodEnvelope) -> WireResult {
        use super::methods::{AlternativeError, AlternativeStatus};
        object!(self,
            "schema_version" => |w| w.count(VERSION),
            "kind" => |w| w.string("checked_method_results"),
            "primary_method" => |w| match value.primary_method() {
                Some(method) => w.string(method.stable_id()),
                None => w.raw("null"),
            },
            "primary" => |w| w.raw(&value.primary().to_json()?),
            "alternative" => |w| match value.alternative() {
                AlternativeStatus::NotRequested => object!(w,
                    "status" => |w| w.string("not_requested"),
                ),
                AlternativeStatus::NotApplicable => object!(w,
                    "status" => |w| w.string("not_applicable"),
                ),
                AlternativeStatus::PrimaryNotComplete => object!(w,
                    "status" => |w| w.string("primary_not_complete"),
                ),
                AlternativeStatus::Failed(error) => object!(w,
                    "status" => |w| w.string("failed"),
                    "reason" => |w| w.string(match error {
                        AlternativeError::Problem(_) => "native_error",
                        AlternativeError::ResourceRefusal(_) => "resource_refusal",
                        AlternativeError::NotApplicable => "not_applicable",
                        AlternativeError::RepeatedMethod => "repeated_method",
                        AlternativeError::Incomplete => "incomplete",
                        AlternativeError::DifferentResult => "different_result",
                    }),
                    "resource" => |w| match error {
                        AlternativeError::ResourceRefusal(kind) => w.string(resource_id(*kind)),
                        _ => w.raw("null"),
                    },
                ),
                AlternativeStatus::Verified(alternative) => object!(w,
                    "status" => |w| w.string("verified"),
                    "method" => |w| w.string(alternative.method().stable_id()),
                    "result" => |w| w.raw(&alternative.result().to_json()?),
                    "factorization" => |w| {
                        let factorization = alternative.factorization();
                        object!(w,
                            "unit" => |w| w.rational(factorization.unit()),
                            "factors" => |w| w.array(factorization.factors(), |w, factor| {
                                object!(w,
                                    "polynomial" => |w| w.polynomial(factor.polynomial()),
                                    "multiplicity" => |w| w.count(factor.multiplicity()),
                                )
                            }),
                        )
                    },
                ),
            },
        )
    }
}

#[cfg(test)]
mod method_wire_tests {
    use super::*;

    #[test]
    fn method_envelope_obeys_aggregate_output_limit() {
        let request = ProblemRequest {
            operation: ProblemOperation::PolynomialEquation,
            statement: ProblemStatement::PolynomialZero(RationalPolynomial::new(vec![
                Rational::from(Integer::from(1)),
                Rational::from(Integer::from(-2)),
                Rational::from(Integer::from(1)),
            ])),
            variables: vec![Symbol::new("x").unwrap()],
            domain: ScalarDomain::Complex,
        };
        let checked =
            ProblemResult::dispatch_with_methods(&EvaluationContext::default(), &request, true)
                .unwrap();
        let bytes = checked.to_json().unwrap().len();
        let mut writer = Encoder {
            text: " ".repeat(MAX_BYTES - bytes + 1),
        };
        assert_eq!(
            writer.method_envelope(&checked),
            Err(ProblemWireError::OutputLimit)
        );
    }
}

mod school;
pub use school::{
    SchoolError, checked_infinity_json, checked_periodic_json, checked_piecewise_json,
    checked_school_json, checked_sqrt_json, checked_trig_identity_json,
};
