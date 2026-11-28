//! Exact equation dispatch for normalized, unrestricted Q[x] polynomials.

use std::fmt;

use mathema::{DefinednessError, DomainExpression, ScalarDomain, Symbol};

use crate::budget::WorkBudget;
use crate::{
    CertifiedSolutionSet, EvaluationContext, EvaluationError, ExactQuadraticRoot,
    PartialPolynomialRootError, PartialPolynomialRootResult, PolynomialZeroSet, QuadraticBranch,
    RationalPolynomial, SolutionCertificateError, SolutionSetCertificate, certify_solution_set,
    solve_partial_polynomial_roots, verify_partial_polynomial_roots, verify_solution_set,
};

/// The normalized source and its checked solution set in an explicit field.
/// Higher-degree inputs retain the complex factor-search evidence, even when
/// some of its roots are outside the requested field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialEquationResult {
    pub original: RationalPolynomial,
    pub certificate: SolutionSetCertificate,
    pub complex_search: Option<PartialPolynomialRootResult>,
}

impl PolynomialEquationResult {
    pub fn solutions(&self) -> &CertifiedSolutionSet {
        &self.certificate.solutions
    }

    pub fn is_complete(&self) -> bool {
        !matches!(self.solutions(), CertifiedSolutionSet::Partial { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolynomialEquationError {
    Evaluation(EvaluationError),
    Definedness(DefinednessError),
    FactorSearch(PartialPolynomialRootError),
    Certificate(SolutionCertificateError),
    UnsupportedDomain,
    IncorrectCertificate,
}

impl fmt::Display for PolynomialEquationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Definedness(error) => write!(f, "{error}"),
            Self::FactorSearch(error) => write!(f, "{error}"),
            Self::Certificate(error) => write!(f, "{error}"),
            Self::UnsupportedDomain => f.write_str("polynomial equations require Q, R or C"),
            Self::IncorrectCertificate => f.write_str("inconsistent polynomial equation evidence"),
        }
    }
}

impl std::error::Error for PolynomialEquationError {}

impl From<EvaluationError> for PolynomialEquationError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}

impl From<PartialPolynomialRootError> for PolynomialEquationError {
    fn from(error: PartialPolynomialRootError) -> Self {
        match error {
            PartialPolynomialRootError::Evaluation(error) => Self::Evaluation(error),
            other => Self::FactorSearch(other),
        }
    }
}

impl From<SolutionCertificateError> for PolynomialEquationError {
    fn from(error: SolutionCertificateError) -> Self {
        match error {
            SolutionCertificateError::Evaluation(error) => Self::Evaluation(error),
            other => Self::Certificate(other),
        }
    }
}

fn operands(
    context: &EvaluationContext,
    original: &RationalPolynomial,
    variable: &Symbol,
    domain: ScalarDomain,
) -> Result<(DomainExpression, DomainExpression), PolynomialEquationError> {
    if domain == ScalarDomain::Integer {
        return Err(PolynomialEquationError::UnsupportedDomain);
    }
    context.check_polynomial(original)?;
    WorkBudget::new(context.limits()).charge(1)?;
    let expression = original.to_expr(variable);
    context.check_expression(&expression)?;
    let limit = context.limits().max_expression_nodes;
    let left = DomainExpression::from_expr(expression, limit)
        .map_err(PolynomialEquationError::Definedness)?;
    let right = DomainExpression::from_expr(0.into(), limit)
        .map_err(PolynomialEquationError::Definedness)?;
    Ok((left, right))
}

fn in_domain(root: &ExactQuadraticRoot, domain: ScalarDomain) -> bool {
    match domain {
        ScalarDomain::Rational => root.as_rational().is_some(),
        ScalarDomain::Real => !matches!(
            root.branch(),
            Some(QuadraticBranch::NegativeImaginary | QuadraticBranch::PositiveImaginary)
        ),
        ScalarDomain::Complex => true,
        ScalarDomain::Integer => false,
    }
}

// The search retains a monic cofactor, whereas the source-bound certificate
// retains the residual scalar unit. Their zero sets must agree exactly.
fn agrees_with_search(
    context: &EvaluationContext,
    search: &PartialPolynomialRootResult,
    certificate: &SolutionSetCertificate,
) -> Result<bool, PolynomialEquationError> {
    let PolynomialZeroSet::Finite {
        roots,
        unresolved_cofactor,
        complete,
    } = search.zero_set()
    else {
        return Ok(false);
    };
    let admitted = roots
        .iter()
        .filter(|entry| in_domain(entry.root(), certificate.domain));
    match (&certificate.solutions, unresolved_cofactor) {
        (CertifiedSolutionSet::None, None) => Ok(*complete && admitted.count() == 0),
        (CertifiedSolutionSet::Finite(values), None) => Ok(*complete && admitted.eq(values.iter())),
        (
            CertifiedSolutionSet::Partial {
                roots: values,
                unresolved_cofactor: residual,
            },
            Some(cofactor),
        ) => {
            if *complete || !admitted.eq(values.iter()) {
                return Ok(false);
            }
            let (quotient, remainder) = context.divide(residual, cofactor)?;
            Ok(remainder.is_zero() && quotient.degree() == Some(0))
        }
        _ => Ok(false),
    }
}

/// Solves P(x) = 0 over Q, R or C. Input must already be an unrestricted
/// normalized polynomial, not the numerator of a domain-restricted expression.
/// Degree at most two is handled directly; higher degrees use bounded rational
/// factor search. Unhandled factors remain partial in every field, even when
/// irreducibility over Q is known. No approximate or general algebraic roots
/// are returned.
pub fn solve_polynomial_equation(
    context: &EvaluationContext,
    original: &RationalPolynomial,
    variable: &Symbol,
    domain: ScalarDomain,
) -> Result<PolynomialEquationResult, PolynomialEquationError> {
    let (left, right) = operands(context, original, variable, domain)?;
    let complex_search = if original.degree().is_some_and(|degree| degree > 2) {
        Some(solve_partial_polynomial_roots(context, original)?)
    } else {
        None
    };
    let hints = complex_search.as_ref().map_or_else(Vec::new, |search| {
        search
            .root_evidence()
            .iter()
            .map(|proof| proof.factor().clone())
            .collect()
    });
    let certificate = certify_solution_set(context, &left, &right, variable, domain, &hints)?;
    if !verify_solution_set(context, &left, &right, variable, domain, &certificate)? {
        return Err(PolynomialEquationError::IncorrectCertificate);
    }
    if let Some(search) = &complex_search
        && !agrees_with_search(context, search, &certificate)?
    {
        return Err(PolynomialEquationError::IncorrectCertificate);
    }
    Ok(PolynomialEquationResult {
        original: original.clone(),
        certificate,
        complex_search,
    })
}

/// Checks the external source, field, factor-search result and source-bound
/// root certificate. A partial claim cannot omit roots found by the supported
/// factor search. Does not invoke the equation dispatcher.
pub fn verify_polynomial_equation(
    context: &EvaluationContext,
    original: &RationalPolynomial,
    variable: &Symbol,
    domain: ScalarDomain,
    proposed: &PolynomialEquationResult,
) -> Result<bool, PolynomialEquationError> {
    let (left, right) = operands(context, original, variable, domain)?;
    if proposed.original != *original
        || !verify_solution_set(
            context,
            &left,
            &right,
            variable,
            domain,
            &proposed.certificate,
        )?
    {
        return Ok(false);
    }
    match (
        original.degree().is_some_and(|degree| degree > 2),
        &proposed.complex_search,
    ) {
        (false, None) => Ok(true),
        (true, Some(search)) => {
            if !verify_partial_polynomial_roots(context, original, search)? {
                return Ok(false);
            }
            agrees_with_search(context, search, &proposed.certificate)
        }
        _ => Ok(false),
    }
}

use mathema::{Integer, Rational};

use crate::{
    AlgebraicRootResult, AlgebraicRootSet, PolynomialRootMultiplicity, verify_algebraic_roots,
};

const MAX_SUBSTITUTION_POWER: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternExclusion {
    Nonrational,
    Nonreal,
    NegativeEven,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolynomialLiftEvidence {
    Excluded(PatternExclusion),
    Quadratic(AlgebraicRootResult),
    FactorSearch(PartialPolynomialRootResult),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialLift {
    pub factor: RationalPolynomial,
    pub multiplicity: usize,
    pub evidence: PolynomialLiftEvidence,
}

/// Source-bound evidence for P(x) = Q(x^power) and the preimages of every
/// outer factor. The legacy rational equation certificate is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialPatternResult {
    pub left: DomainExpression,
    pub right: DomainExpression,
    pub variable: Symbol,
    pub domain: ScalarDomain,
    pub original: RationalPolynomial,
    pub power: usize,
    pub outer: AlgebraicRootResult,
    pub lifts: Vec<PolynomialLift>,
    pub solutions: CertifiedSolutionSet,
}

impl PolynomialPatternResult {
    pub const fn method_id(&self) -> &'static str {
        "polynomial.power_substitution"
    }

    pub fn is_complete(&self) -> bool {
        !matches!(self.solutions, CertifiedSolutionSet::Partial { .. })
    }
}

fn pattern_zero() -> Rational {
    Rational::from(Integer::zero())
}

fn pattern_one() -> Rational {
    Rational::from(Integer::one())
}

fn total_polynomial(
    context: &EvaluationContext,
    left: &DomainExpression,
    right: &DomainExpression,
    variable: &Symbol,
    domain: ScalarDomain,
) -> Result<RationalPolynomial, PolynomialEquationError> {
    if domain == ScalarDomain::Integer {
        return Err(PolynomialEquationError::UnsupportedDomain);
    }
    for source in [left, right] {
        if source.construction_domain() != Some(domain)
            || source.has_external_history()
            || !source.conditions().is_empty()
        {
            return Err(SolutionCertificateError::UnsupportedDomain.into());
        }
    }
    let lhs = context.polynomial(left.expression(), variable)?;
    let rhs = context.polynomial(right.expression(), variable)?;
    let residual = WorkBudget::new(context.limits()).subtract(&lhs, &rhs)?;
    context.check_polynomial(&residual)?;
    Ok(residual)
}

fn substitution(
    original: &RationalPolynomial,
    work: &mut WorkBudget,
) -> Result<Option<(usize, RationalPolynomial)>, EvaluationError> {
    let Some(degree) = original.degree() else {
        return Ok(None);
    };
    if degree <= 2 || degree > 2 * MAX_SUBSTITUTION_POWER {
        return Ok(None);
    }
    for power in 2..=MAX_SUBSTITUTION_POWER {
        work.charge((degree + 1) as u64)?;
        if degree.is_multiple_of(power)
            && degree / power <= 2
            && original
                .coefficients()
                .iter()
                .enumerate()
                .all(|(i, c)| i.is_multiple_of(power) || c.numerator().is_zero())
        {
            return Ok(Some((
                power,
                RationalPolynomial::new(
                    original
                        .coefficients()
                        .iter()
                        .step_by(power)
                        .cloned()
                        .collect(),
                ),
            )));
        }
    }
    Ok(None)
}

fn compose_power(factor: &RationalPolynomial, power: usize) -> RationalPolynomial {
    let mut coefficients = vec![pattern_zero(); factor.degree().unwrap_or(0) * power + 1];
    for (index, coefficient) in factor.coefficients().iter().enumerate() {
        coefficients[index * power] = coefficient.clone();
    }
    RationalPolynomial::new(coefficients)
}

type OuterFactor = (RationalPolynomial, usize, Option<PatternExclusion>);

// The verified outer proof supplies rational linear factors or an irreducible
// quadratic pair. A rational or real x cannot have a nonrational or nonreal
// integer power, respectively.
fn outer_factors(
    outer: &AlgebraicRootResult,
    power: usize,
    domain: ScalarDomain,
) -> Option<Vec<OuterFactor>> {
    let AlgebraicRootSet::Finite(roots) = outer.roots() else {
        return None;
    };
    if roots.is_empty() || roots.len() > 2 {
        return None;
    }
    if let Some(minimal) = roots[0].minimal_polynomial() {
        let exclusion = match domain {
            ScalarDomain::Rational => Some(PatternExclusion::Nonrational),
            ScalarDomain::Real if !in_domain(&roots[0], domain) => Some(PatternExclusion::Nonreal),
            _ => None,
        };
        return Some(vec![(minimal.clone(), 1, exclusion)]);
    }
    let multiplicity = if outer.original().degree() == Some(2) && roots.len() == 1 {
        2
    } else {
        1
    };
    roots
        .iter()
        .map(|root| {
            let value = root.as_rational()?;
            let exclusion = if domain != ScalarDomain::Complex
                && power.is_multiple_of(2)
                && value < &pattern_zero()
            {
                Some(PatternExclusion::NegativeEven)
            } else {
                None
            };
            Some((
                RationalPolynomial::new(vec![-value, pattern_one()]),
                multiplicity,
                exclusion,
            ))
        })
        .collect()
}

fn collect_pattern_roots(
    roots: &mut Vec<PolynomialRootMultiplicity>,
    values: &[PolynomialRootMultiplicity],
    multiplicity: usize,
    domain: ScalarDomain,
) -> bool {
    for entry in values
        .iter()
        .filter(|entry| in_domain(entry.root(), domain))
    {
        if roots.iter().any(|existing| existing.root() == entry.root()) {
            return false;
        }
        roots.push(PolynomialRootMultiplicity::new(
            entry.root().clone(),
            entry.multiplicity() * multiplicity,
        ));
    }
    true
}

fn checked_lifts(
    context: &EvaluationContext,
    factors: &[OuterFactor],
    power: usize,
    domain: ScalarDomain,
    lifts: &[PolynomialLift],
) -> Result<Option<CertifiedSolutionSet>, PolynomialEquationError> {
    if factors.len() != lifts.len() {
        return Ok(None);
    }
    let mut work = WorkBudget::new(context.limits());
    let mut roots = Vec::new();
    let mut unresolved = RationalPolynomial::new(vec![pattern_one()]);
    for ((factor, multiplicity, exclusion), lift) in factors.iter().zip(lifts) {
        work.charge(1)?;
        if lift.factor != *factor || lift.multiplicity != *multiplicity {
            return Ok(None);
        }
        let composed = compose_power(factor, power);
        context.check_polynomial(&composed)?;
        if let Some(reason) = exclusion {
            if lift.evidence != PolynomialLiftEvidence::Excluded(*reason) {
                return Ok(None);
            }
            continue;
        }
        match &lift.evidence {
            PolynomialLiftEvidence::Quadratic(proof) => {
                if composed.degree() != Some(2) || proof.original() != &composed {
                    return Ok(None);
                }
                crate::algebraic::checked_algebraic_roots(&composed, &mut work)?;
                if verify_algebraic_roots(proof).is_err() {
                    return Ok(None);
                }
                let AlgebraicRootSet::Finite(values) = proof.roots() else {
                    return Ok(None);
                };
                let inner_multiplicity = if values.len() == 1 { 2 } else { 1 };
                let values: Vec<_> = values
                    .iter()
                    .map(|root| PolynomialRootMultiplicity::new(root.clone(), inner_multiplicity))
                    .collect();
                if !collect_pattern_roots(&mut roots, &values, *multiplicity, domain) {
                    return Ok(None);
                }
            }
            PolynomialLiftEvidence::FactorSearch(search) => {
                if composed.degree().is_none_or(|degree| degree <= 2)
                    || !verify_partial_polynomial_roots(context, &composed, search)?
                {
                    return Ok(None);
                }
                let PolynomialZeroSet::Finite {
                    roots: values,
                    unresolved_cofactor,
                    ..
                } = search.zero_set()
                else {
                    return Ok(None);
                };
                if !collect_pattern_roots(&mut roots, values, *multiplicity, domain) {
                    return Ok(None);
                }
                if let Some(cofactor) = unresolved_cofactor {
                    for _ in 0..*multiplicity {
                        unresolved = work.multiply(&unresolved, cofactor)?;
                    }
                }
            }
            PolynomialLiftEvidence::Excluded(_) => return Ok(None),
        }
    }
    roots.sort_by(|a, b| match (a.root(), b.root()) {
        (ExactQuadraticRoot::Rational(a), ExactQuadraticRoot::Rational(b)) => a.cmp(b),
        (ExactQuadraticRoot::Rational(_), _) => std::cmp::Ordering::Less,
        (_, ExactQuadraticRoot::Rational(_)) => std::cmp::Ordering::Greater,
        (
            ExactQuadraticRoot::Algebraic {
                minimal_polynomial: a,
                branch: ab,
            },
            ExactQuadraticRoot::Algebraic {
                minimal_polynomial: b,
                branch: bb,
            },
        ) => a
            .coefficients()
            .cmp(b.coefficients())
            .then_with(|| ab.stable_id().cmp(bb.stable_id())),
    });
    Ok(Some(if unresolved.degree() != Some(0) {
        CertifiedSolutionSet::Partial {
            roots,
            unresolved_cofactor: unresolved,
        }
    } else if roots.is_empty() {
        CertifiedSolutionSet::None
    } else {
        CertifiedSolutionSet::Finite(roots)
    }))
}

impl EvaluationContext {
    /// Select a bounded repeated-power substitution for a total, domain-bound
    /// polynomial equation. None means no applicable pattern, not no roots.
    /// Restricted or unbound source histories are refused.
    pub fn solve_pattern_polynomial_equation(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
    ) -> Result<Option<PolynomialPatternResult>, PolynomialEquationError> {
        let original = total_polynomial(self, left, right, variable, domain)?;
        let mut work = WorkBudget::new(self.limits());
        let Some((power, reduced)) = substitution(&original, &mut work)? else {
            return Ok(None);
        };
        let outer = self.algebraic_roots(&reduced)?;
        let factors = outer_factors(&outer, power, domain)
            .ok_or(PolynomialEquationError::IncorrectCertificate)?;
        let mut lifts = Vec::new();
        for (factor, multiplicity, exclusion) in &factors {
            let composed = compose_power(factor, power);
            let evidence = if let Some(reason) = exclusion {
                PolynomialLiftEvidence::Excluded(*reason)
            } else if composed.degree() == Some(2) {
                PolynomialLiftEvidence::Quadratic(self.algebraic_roots(&composed)?)
            } else {
                PolynomialLiftEvidence::FactorSearch(solve_partial_polynomial_roots(
                    self, &composed,
                )?)
            };
            lifts.push(PolynomialLift {
                factor: factor.clone(),
                multiplicity: *multiplicity,
                evidence,
            });
        }
        let solutions = checked_lifts(self, &factors, power, domain, &lifts)?
            .ok_or(PolynomialEquationError::IncorrectCertificate)?;
        let result = PolynomialPatternResult {
            left: left.clone(),
            right: right.clone(),
            variable: variable.clone(),
            domain,
            original,
            power,
            outer,
            lifts,
            solutions,
        };
        if !self.verify_pattern_polynomial_equation(left, right, variable, domain, &result)? {
            return Err(PolynomialEquationError::IncorrectCertificate);
        }
        Ok(Some(result))
    }

    /// Independently check a distinct direct factorization of the original
    /// polynomial, only when the substitution method did not itself use a
    /// factor search and both methods prove the same complete solution set.
    pub fn polynomial_pattern_factor_alternative(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        pattern: &PolynomialPatternResult,
    ) -> Result<Option<PolynomialEquationResult>, PolynomialEquationError> {
        if !self.verify_pattern_polynomial_equation(left, right, variable, domain, pattern)? {
            return Err(PolynomialEquationError::IncorrectCertificate);
        }
        if !pattern.is_complete()
            || pattern
                .lifts
                .iter()
                .any(|lift| matches!(lift.evidence, PolynomialLiftEvidence::FactorSearch(_)))
        {
            return Ok(None);
        }
        let alternative = solve_polynomial_equation(self, &pattern.original, variable, domain)?;
        if !alternative.is_complete() || alternative.solutions() != &pattern.solutions {
            return Ok(None);
        }
        if !verify_polynomial_equation(self, &pattern.original, variable, domain, &alternative)? {
            return Err(PolynomialEquationError::IncorrectCertificate);
        }
        Ok(Some(alternative))
    }

    /// Check the external source, exact substitution identity, outer root
    /// proof, every lifted factor and multiplicity, and domain classification.
    /// Does not call the pattern solver or trust the claimed solution list.
    pub fn verify_pattern_polynomial_equation(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        result: &PolynomialPatternResult,
    ) -> Result<bool, PolynomialEquationError> {
        let original = total_polynomial(self, left, right, variable, domain)?;
        if result.left != *left
            || result.right != *right
            || result.variable != *variable
            || result.domain != domain
            || result.original != original
            || !(2..=MAX_SUBSTITUTION_POWER).contains(&result.power)
            || result
                .outer
                .original()
                .degree()
                .is_none_or(|d| !(1..=2).contains(&d))
        {
            return Ok(false);
        }
        let mut work = WorkBudget::new(self.limits());
        let Some((power, reduced)) = substitution(&original, &mut work)? else {
            return Ok(false);
        };
        if result.power != power
            || result.outer.original() != &reduced
            || compose_power(&reduced, power) != original
        {
            return Ok(false);
        }
        crate::algebraic::checked_algebraic_roots(&reduced, &mut work)?;
        if verify_algebraic_roots(&result.outer).is_err() {
            return Ok(false);
        }
        let Some(factors) = outer_factors(&result.outer, power, domain) else {
            return Ok(false);
        };
        Ok(
            checked_lifts(self, &factors, power, domain, &result.lifts)?.as_ref()
                == Some(&result.solutions),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchoolFactorRule {
    CommonPower,
    GroupedBlocks,
    DifferenceOfSquares,
}

impl SchoolFactorRule {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::CommonPower => "polynomial.common_power",
            Self::GroupedBlocks => "polynomial.grouped_blocks",
            Self::DifferenceOfSquares => "polynomial.difference_of_squares",
        }
    }
}

/// Exact factor identity, followed by source-bound root classification.
/// Repeated monic factors occur once per multiplicity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchoolFactorResult {
    pub rule: SchoolFactorRule,
    pub unit: Rational,
    pub factors: Vec<RationalPolynomial>,
    pub original: RationalPolynomial,
    pub certificate: SolutionSetCertificate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchoolPolynomialResult {
    Substitution(Box<PolynomialPatternResult>),
    Factored(Box<SchoolFactorResult>),
}

impl SchoolPolynomialResult {
    pub fn solutions(&self) -> &CertifiedSolutionSet {
        match self {
            Self::Substitution(result) => &result.solutions,
            Self::Factored(result) => &result.certificate.solutions,
        }
    }

    pub fn method_id(&self) -> &'static str {
        match self {
            Self::Substitution(result) => result.method_id(),
            Self::Factored(result) => result.rule.stable_id(),
        }
    }
}

type SchoolFactors = (SchoolFactorRule, Rational, Vec<RationalPolynomial>);

fn school_factors(
    original: &RationalPolynomial,
    work: &mut WorkBudget,
) -> Result<Option<SchoolFactors>, EvaluationError> {
    let Some(degree) = original.degree() else {
        return Ok(None);
    };
    if !(3..=2 * MAX_SUBSTITUTION_POWER).contains(&degree) {
        return Ok(None);
    }
    let coefficients = original.coefficients();
    let unit = coefficients[degree].clone();
    let monic = coefficients
        .iter()
        .map(|coefficient| work.quotient(coefficient, &unit))
        .collect::<Result<Vec<_>, _>>()?;
    let order = monic
        .iter()
        .position(|coefficient| !coefficient.numerator().is_zero())
        .expect("nonzero leading coefficient");
    if order > 0 && degree - order <= 2 {
        let variable = RationalPolynomial::new(vec![pattern_zero(), pattern_one()]);
        let mut factors = vec![variable; order];
        if order < degree {
            factors.push(RationalPolynomial::new(monic[order..].to_vec()));
        }
        return Ok(Some((SchoolFactorRule::CommonPower, unit, factors)));
    }
    // Group ax^3 + bx^2 + cx + d by powers of x^2:
    // after normalization, (x + b/a)(x^2 + c/a).
    if degree == 3 && work.multiply_scalar(&monic[1], &monic[2])? == monic[0] {
        return Ok(Some((
            SchoolFactorRule::GroupedBlocks,
            unit,
            vec![
                RationalPolynomial::new(vec![monic[2].clone(), pattern_one()]),
                RationalPolynomial::new(vec![monic[1].clone(), pattern_zero(), pattern_one()]),
            ],
        )));
    }
    // Determine the only possible monic quadratic square from the cubic
    // and quadratic coefficients, then check the linear coefficient.
    if degree == 4 {
        let two = Rational::from(Integer::from(2));
        let b = work.quotient(&monic[3], &two)?;
        let b_squared = work.multiply_scalar(&b, &b)?;
        let c_numerator = work.subtract_scalar(&monic[2], &b_squared)?;
        let c = work.quotient(&c_numerator, &two)?;
        let bc = work.multiply_scalar(&b, &c)?;
        if work.multiply_scalar(&two, &bc)? == monic[1] {
            let c_squared = work.multiply_scalar(&c, &c)?;
            let delta = work.subtract_scalar(&c_squared, &monic[0])?;
            let bits = delta
                .numerator()
                .bit_len()
                .saturating_add(delta.denominator().bit_len());
            work.charge(
                bits.saturating_mul(bits)
                    .saturating_div(64)
                    .saturating_add(1),
            )?;
            if let Some(square_root) = delta.exact_sqrt() {
                let lower = work.subtract_scalar(&c, &square_root)?;
                let upper = work.add_scalar(&c, &square_root)?;
                return Ok(Some((
                    SchoolFactorRule::DifferenceOfSquares,
                    unit,
                    vec![
                        RationalPolynomial::new(vec![lower, b.clone(), pattern_one()]),
                        RationalPolynomial::new(vec![upper, b, pattern_one()]),
                    ],
                )));
            }
        }
    }
    Ok(None)
}

fn school_rule_shape(rule: SchoolFactorRule, factors: &[RationalPolynomial]) -> bool {
    match rule {
        SchoolFactorRule::CommonPower => {
            let Some((last, preceding)) = factors.split_last() else {
                return false;
            };
            let variable = RationalPolynomial::new(vec![pattern_zero(), pattern_one()]);
            !preceding.is_empty()
                && preceding.iter().all(|factor| factor == &variable)
                && matches!(last.degree(), Some(1 | 2))
        }
        SchoolFactorRule::GroupedBlocks => {
            factors.len() == 2
                && factors[0].degree() == Some(1)
                && factors[1].degree() == Some(2)
                && factors[1].coefficients()[1].numerator().is_zero()
        }
        SchoolFactorRule::DifferenceOfSquares => {
            factors.len() == 2
                && factors[0].degree() == Some(2)
                && factors[1].degree() == Some(2)
                && factors[0].coefficients()[1] == factors[1].coefficients()[1]
        }
    }
}

impl EvaluationContext {
    /// Try a repeated-power substitution first, then bounded elementary
    /// factorizations. None means the caller may use the general solver.
    /// The original operands must be total and domain-bound.
    pub fn solve_school_polynomial_equation(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
    ) -> Result<Option<SchoolPolynomialResult>, PolynomialEquationError> {
        if let Some(pattern) =
            self.solve_pattern_polynomial_equation(left, right, variable, domain)?
        {
            return Ok(Some(SchoolPolynomialResult::Substitution(Box::new(
                pattern,
            ))));
        }
        let original = total_polynomial(self, left, right, variable, domain)?;
        let mut work = WorkBudget::new(self.limits());
        let Some((rule, unit, factors)) = school_factors(&original, &mut work)? else {
            return Ok(None);
        };
        let certificate = certify_solution_set(self, left, right, variable, domain, &factors)?;
        let result = SchoolPolynomialResult::Factored(Box::new(SchoolFactorResult {
            rule,
            unit,
            factors,
            original,
            certificate,
        }));
        if !self.verify_school_polynomial_equation(left, right, variable, domain, &result)? {
            return Err(PolynomialEquationError::IncorrectCertificate);
        }
        Ok(Some(result))
    }

    /// Verify the claimed strategy's identity and root proof without invoking
    /// the strategy selector or a generic factor search.
    pub fn verify_school_polynomial_equation(
        &self,
        left: &DomainExpression,
        right: &DomainExpression,
        variable: &Symbol,
        domain: ScalarDomain,
        result: &SchoolPolynomialResult,
    ) -> Result<bool, PolynomialEquationError> {
        let SchoolPolynomialResult::Factored(result) = result else {
            if let SchoolPolynomialResult::Substitution(pattern) = result {
                return self
                    .verify_pattern_polynomial_equation(left, right, variable, domain, pattern);
            }
            return Ok(false);
        };
        let original = total_polynomial(self, left, right, variable, domain)?;
        let Some(degree) = original.degree() else {
            return Ok(false);
        };
        if result.original != original
            || !(3..=2 * MAX_SUBSTITUTION_POWER).contains(&degree)
            || result.unit != original.coefficients()[degree]
            || result.factors.len() > degree
            || !school_rule_shape(result.rule, &result.factors)
            || result.certificate.factors.len() != result.factors.len()
            || !matches!(
                result.certificate.solutions,
                CertifiedSolutionSet::None | CertifiedSolutionSet::Finite(_)
            )
        {
            return Ok(false);
        }
        let mut work = WorkBudget::new(self.limits());
        let mut product = RationalPolynomial::new(vec![result.unit.clone()]);
        for (factor, proof) in result.factors.iter().zip(&result.certificate.factors) {
            if !matches!(factor.degree(), Some(1 | 2))
                || factor.coefficients().last() != Some(&pattern_one())
                || proof.original() != factor
            {
                return Ok(false);
            }
            product = work.multiply(&product, factor)?;
        }
        if product != original {
            return Ok(false);
        }
        Ok(verify_solution_set(
            self,
            left,
            right,
            variable,
            domain,
            &result.certificate,
        )?)
    }
}

#[cfg(test)]
mod pattern_tests {
    use super::*;

    fn p(coefficients: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            coefficients
                .iter()
                .map(|&value| Rational::from(Integer::from(value)))
                .collect(),
        )
    }

    fn source(poly: &RationalPolynomial, domain: ScalarDomain) -> DomainExpression {
        DomainExpression::from_expr_in_domain(
            poly.to_expr(&Symbol::new("x").unwrap()),
            domain,
            2048,
            32,
        )
        .unwrap()
    }

    fn rational(value: i64, multiplicity: usize) -> PolynomialRootMultiplicity {
        PolynomialRootMultiplicity::new(
            ExactQuadraticRoot::Rational(Rational::from(Integer::from(value))),
            multiplicity,
        )
    }

    fn solve(poly: &RationalPolynomial, domain: ScalarDomain) -> PolynomialPatternResult {
        let context = EvaluationContext::default();
        let x = Symbol::new("x").unwrap();
        let left = source(poly, domain);
        let right = source(&p(&[]), domain);
        let result = context
            .solve_pattern_polynomial_equation(&left, &right, &x, domain)
            .unwrap()
            .unwrap();
        assert!(
            context
                .verify_pattern_polynomial_equation(&left, &right, &x, domain, &result)
                .unwrap()
        );
        assert_eq!(
            context
                .solve_pattern_polynomial_equation(&left, &right, &x, domain)
                .unwrap(),
            Some(result.clone())
        );
        result
    }

    #[test]
    fn biquadratic_and_alternative_factorization() {
        let original = p(&[4, 0, -5, 0, 1]);
        let result = solve(&original, ScalarDomain::Real);
        assert_eq!(result.power, 2);
        assert_eq!(result.outer.original(), &p(&[4, -5, 1]));
        assert_eq!(
            result.solutions,
            CertifiedSolutionSet::Finite(vec![
                rational(-2, 1),
                rational(-1, 1),
                rational(1, 1),
                rational(2, 1),
            ])
        );
        let context = EvaluationContext::default();
        let x = Symbol::new("x").unwrap();
        let alternative = context
            .polynomial_pattern_factor_alternative(
                &result.left,
                &result.right,
                &x,
                ScalarDomain::Real,
                &result,
            )
            .unwrap()
            .unwrap();
        assert_eq!(result.method_id(), "polynomial.power_substitution");
        assert_eq!(alternative.solutions(), &result.solutions);
        assert!(
            verify_polynomial_equation(&context, &original, &x, ScalarDomain::Real, &alternative)
                .unwrap()
        );
    }

    #[test]
    fn domain_exclusions_and_complex_lifts() {
        let original = p(&[-16, 0, 0, 0, 1]);
        let real = solve(&original, ScalarDomain::Real);
        assert_eq!(
            real.solutions,
            CertifiedSolutionSet::Finite(vec![rational(-2, 1), rational(2, 1)])
        );
        assert!(matches!(
            real.lifts[0].evidence,
            PolynomialLiftEvidence::Excluded(PatternExclusion::NegativeEven)
        ));
        let complex = solve(&original, ScalarDomain::Complex);
        let CertifiedSolutionSet::Finite(roots) = complex.solutions else {
            panic!("expected all four roots");
        };
        assert_eq!(roots.len(), 4);
        assert_eq!(roots[2].root().minimal_polynomial(), Some(&p(&[4, 0, 1])));
        assert_eq!(
            roots[2].root().branch(),
            Some(QuadraticBranch::NegativeImaginary)
        );
        assert_eq!(
            roots[3].root().branch(),
            Some(QuadraticBranch::PositiveImaginary)
        );
        let no_real = solve(&p(&[1, 0, 0, 0, 1]), ScalarDomain::Real);
        assert_eq!(no_real.solutions, CertifiedSolutionSet::None);
        assert!(matches!(
            no_real.lifts[0].evidence,
            PolynomialLiftEvidence::Excluded(PatternExclusion::Nonreal)
        ));
        assert_eq!(
            solve(&p(&[-2, 0, 0, 0, 1]), ScalarDomain::Rational).solutions,
            CertifiedSolutionSet::None
        );
    }

    #[test]
    fn multiplicities_and_zero_preimage() {
        assert_eq!(
            solve(&p(&[1, 0, -2, 0, 1]), ScalarDomain::Real).solutions,
            CertifiedSolutionSet::Finite(vec![rational(-1, 2), rational(1, 2)])
        );
        assert_eq!(
            solve(&p(&[0, 0, 0, 0, 1]), ScalarDomain::Complex).solutions,
            CertifiedSolutionSet::Finite(vec![rational(0, 4)])
        );
    }

    #[test]
    fn cubic_lifts_and_unresolved_cofactor() {
        let result = solve(&p(&[8, 0, 0, -9, 0, 0, 1]), ScalarDomain::Real);
        assert_eq!(result.power, 3);
        assert_eq!(
            result.solutions,
            CertifiedSolutionSet::Finite(vec![rational(1, 1), rational(2, 1)])
        );
        let partial = solve(&p(&[-2, 0, 0, 1]), ScalarDomain::Real);
        assert_eq!(
            partial.solutions,
            CertifiedSolutionSet::Partial {
                roots: vec![],
                unresolved_cofactor: p(&[-2, 0, 0, 1]),
            }
        );
        assert!(!partial.is_complete());
    }

    #[test]
    fn rejects_forged_evidence_and_mismatched_source() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        let x = Symbol::new("x").unwrap();
        let left = source(&p(&[-16, 0, 0, 0, 1]), domain);
        let right = source(&p(&[]), domain);
        let result = solve(&p(&[-16, 0, 0, 0, 1]), domain);
        let mut forged = result.clone();
        forged.lifts[0].multiplicity = 2;
        assert!(
            !context
                .verify_pattern_polynomial_equation(&left, &right, &x, domain, &forged)
                .unwrap()
        );
        let mut forged = result.clone();
        forged.solutions = CertifiedSolutionSet::None;
        assert!(
            !context
                .verify_pattern_polynomial_equation(&left, &right, &x, domain, &forged)
                .unwrap()
        );
        let mut forged = result.clone();
        forged.power = usize::MAX;
        assert!(
            !context
                .verify_pattern_polynomial_equation(&left, &right, &x, domain, &forged)
                .unwrap()
        );
        assert!(
            !context
                .verify_pattern_polynomial_equation(&right, &left, &x, domain, &result)
                .unwrap()
        );
    }

    #[test]
    fn bounded_selection_and_restricted_sources() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        let x = Symbol::new("x").unwrap();
        let zero = source(&p(&[]), domain);
        let one = source(&p(&[1]), domain);
        let variable = source(&p(&[0, 1]), domain);
        let restricted = variable.clone().divide(variable, 32).unwrap();
        let left = source(&p(&[1, 1, 0, 0, 1]), domain);
        assert!(
            context
                .solve_pattern_polynomial_equation(&left, &zero, &x, domain)
                .unwrap()
                .is_none()
        );
        assert!(
            context
                .solve_pattern_polynomial_equation(&restricted, &one, &x, domain)
                .is_err()
        );
        let unbound = DomainExpression::from_expr(0.into(), 2048).unwrap();
        assert!(
            context
                .solve_pattern_polynomial_equation(&left, &unbound, &x, domain)
                .is_err()
        );
        let above_bound = source(&p(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 1]), domain);
        assert!(
            context
                .solve_pattern_polynomial_equation(&above_bound, &zero, &x, domain)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn resource_refusals_and_nested_radicals() {
        use crate::{ResourceLimitKind, ResourceLimits};

        let domain = ScalarDomain::Real;
        let x = Symbol::new("x").unwrap();
        let left = source(&p(&[4, 0, -5, 0, 1]), domain);
        let right = source(&p(&[]), domain);
        let valid = solve(&p(&[4, 0, -5, 0, 1]), domain);
        let limited = EvaluationContext::new(ResourceLimits {
            max_work_units: 0,
            ..ResourceLimits::default()
        });
        let error = PolynomialEquationError::Evaluation(EvaluationError::Limit(
            ResourceLimitKind::WorkUnits,
        ));
        assert_eq!(
            limited.solve_pattern_polynomial_equation(&left, &right, &x, domain),
            Err(error)
        );
        assert_eq!(
            limited.verify_pattern_polynomial_equation(&left, &right, &x, domain, &valid),
            Err(error)
        );
        let nested = solve(&p(&[-2, 0, 0, 0, 1]), domain);
        assert_eq!(
            nested.solutions,
            CertifiedSolutionSet::Partial {
                roots: vec![],
                unresolved_cofactor: p(&[-2, 0, 0, 0, 1]),
            }
        );
    }

    #[test]
    fn selects_elementary_factorizations_before_general_search() {
        let context = EvaluationContext::default();
        let domain = ScalarDomain::Real;
        let x = Symbol::new("x").unwrap();
        let right = source(&p(&[]), domain);
        for (poly, method, expected) in [
            (
                p(&[-12, -4, 3, 1]),
                "polynomial.grouped_blocks",
                vec![rational(-3, 1), rational(-2, 1), rational(2, 1)],
            ),
            (
                p(&[0, 0, 0, -1, 0, 1]),
                "polynomial.common_power",
                vec![rational(-1, 1), rational(0, 3), rational(1, 1)],
            ),
        ] {
            let left = source(&poly, domain);
            let result = context
                .solve_school_polynomial_equation(&left, &right, &x, domain)
                .unwrap()
                .unwrap();
            assert_eq!(result.method_id(), method);
            assert_eq!(result.solutions(), &CertifiedSolutionSet::Finite(expected));
            assert!(
                context
                    .verify_school_polynomial_equation(&left, &right, &x, domain, &result)
                    .unwrap()
            );
        }
        let left = source(&p(&[-5, 4, 5, 2, 1]), domain);
        let result = context
            .solve_school_polynomial_equation(&left, &right, &x, domain)
            .unwrap()
            .unwrap();
        assert_eq!(result.method_id(), "polynomial.difference_of_squares");
        let SchoolPolynomialResult::Factored(factored) = &result else {
            panic!("expected a difference of squares");
        };
        assert_eq!(factored.factors, vec![p(&[-1, 1, 1]), p(&[5, 1, 1])]);
        let CertifiedSolutionSet::Finite(roots) = result.solutions() else {
            panic!("expected two real roots");
        };
        assert_eq!(roots.len(), 2);
        assert!(
            roots
                .iter()
                .all(|root| root.root().minimal_polynomial() == Some(&p(&[-1, 1, 1])))
        );
        let mut forged = result.clone();
        let SchoolPolynomialResult::Factored(factored) = &mut forged else {
            unreachable!();
        };
        factored.factors[0] = p(&[-2, 1, 1]);
        assert!(
            !context
                .verify_school_polynomial_equation(&left, &right, &x, domain, &forged)
                .unwrap()
        );

        let left = source(&p(&[-16, 0, 0, 0, 1]), domain);
        let result = context
            .solve_school_polynomial_equation(&left, &right, &x, domain)
            .unwrap()
            .unwrap();
        assert_eq!(result.method_id(), "polynomial.power_substitution");

        let left = source(&p(&[1, 2, 0, 1]), domain);
        assert!(
            context
                .solve_school_polynomial_equation(&left, &right, &x, domain)
                .unwrap()
                .is_none()
        );
    }
}
