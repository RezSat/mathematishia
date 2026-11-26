//! Bounded Buchberger bases in Q[x_1, ..., x_n], with lexicographic monomials.
//! The basis need not be reduced or monic. A certificate proves equality of
//! ideals, not equality of solution sets on an arbitrary restricted domain.

use std::error::Error;
use std::fmt;

use mathema::{DomainExpression, Integer, Rational, Symbol};

use crate::{MultivariateError, MultivariateExprError, MultivariateLimits, MultivariatePolynomial};

const MAX_GENERATORS: usize = 8;
const MAX_BASIS: usize = 12;
const MAX_VARIABLE_NAME_BYTES: usize = 256;

const HARD_LIMITS: MultivariateLimits = MultivariateLimits {
    max_variables: 3,
    max_terms: 128,
    max_input_terms: 1024,
    max_total_degree: 24,
    max_scalar_bits: 512,
    max_work_units: 250_000,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroebnerError {
    GeneratorLimit,
    BasisLimit,
    VariableNameLimit,
    RestrictedDomain { input: usize },
    InvalidCertificate,
    Polynomial(MultivariateError),
    Expression(MultivariateExprError),
}

impl fmt::Display for GroebnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GeneratorLimit => f.write_str("Groebner generator count limit exceeded"),
            Self::BasisLimit => f.write_str("Groebner basis count limit exceeded"),
            Self::VariableNameLimit => f.write_str("Groebner variable name limit exceeded"),
            Self::RestrictedDomain { input } => {
                write!(f, "Groebner input {input} has original domain restrictions")
            }
            Self::InvalidCertificate => f.write_str("invalid Groebner certificate"),
            Self::Polynomial(error) => write!(f, "{error}"),
            Self::Expression(error) => write!(f, "{error}"),
        }
    }
}
impl Error for GroebnerError {}
impl From<MultivariateError> for GroebnerError {
    fn from(error: MultivariateError) -> Self {
        Self::Polynomial(error)
    }
}
impl From<MultivariateExprError> for GroebnerError {
    fn from(error: MultivariateExprError) -> Self {
        Self::Expression(error)
    }
}

/// Untrusted division evidence. Quotients follow basis order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroebnerReduction {
    pub quotients: Vec<MultivariatePolynomial>,
    pub remainder: MultivariatePolynomial,
    pub steps: usize,
}

/// One final-basis S-pair, in increasing right-index then left-index order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroebnerPair {
    pub left: usize,
    pub right: usize,
    pub lcm: Vec<u32>,
    pub reduction: GroebnerReduction,
}

/// Untrusted evidence for both ideal inclusions and Buchberger's criterion.
/// Row i of representations expresses basis[i] in the input generator order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroebnerCertificate {
    pub variables: Vec<Symbol>,
    pub basis: Vec<MultivariatePolynomial>,
    pub representations: Vec<Vec<MultivariatePolynomial>>,
    pub generator_reductions: Vec<GroebnerReduction>,
    pub pairs: Vec<GroebnerPair>,
}

/// Query evidence. Trust a result only after successful computation or verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdealMembership {
    Member(GroebnerReduction),
    NonMember(GroebnerReduction),
}

struct Work {
    zero: MultivariatePolynomial,
    limits: MultivariateLimits,
    remaining: u64,
}

impl Work {
    fn new(variables: &[Symbol], limits: MultivariateLimits) -> Result<Self, GroebnerError> {
        let limits = MultivariateLimits {
            max_variables: limits.max_variables.min(HARD_LIMITS.max_variables),
            max_terms: limits.max_terms.min(HARD_LIMITS.max_terms),
            max_input_terms: limits.max_input_terms.min(HARD_LIMITS.max_input_terms),
            max_total_degree: limits.max_total_degree.min(HARD_LIMITS.max_total_degree),
            max_scalar_bits: limits.max_scalar_bits.min(HARD_LIMITS.max_scalar_bits),
            max_work_units: limits.max_work_units.min(HARD_LIMITS.max_work_units),
        };
        if variables.len() > limits.max_variables {
            return Err(MultivariateError::VariableLimit.into());
        }
        if variables
            .iter()
            .any(|s| s.name().len() > MAX_VARIABLE_NAME_BYTES)
        {
            return Err(GroebnerError::VariableNameLimit);
        }
        Ok(Self {
            zero: MultivariatePolynomial::zero(variables.to_vec(), limits)?,
            limits,
            remaining: limits.max_work_units,
        })
    }

    fn charge(&mut self, units: usize) -> Result<(), GroebnerError> {
        self.remaining = self
            .remaining
            .checked_sub(units as u64)
            .ok_or(MultivariateError::WorkLimit)?;
        Ok(())
    }

    fn admit(&mut self, p: &MultivariatePolynomial) -> Result<(), GroebnerError> {
        if p.variables() != self.zero.variables() {
            return Err(MultivariateError::VariableMismatch.into());
        }
        if p.terms().len() > self.limits.max_terms {
            return Err(MultivariateError::TermLimit.into());
        }
        if p.terms().len() > self.limits.max_input_terms {
            return Err(MultivariateError::InputTermLimit.into());
        }
        self.charge(p.terms().len() + 1)?;
        for (powers, coefficient) in p.terms() {
            self.degree(powers)?;
            self.bits(
                coefficient.numerator().bit_len(),
                coefficient.denominator().bit_len(),
            )?;
        }
        Ok(())
    }

    fn degree(&self, powers: &[u32]) -> Result<(), GroebnerError> {
        let degree = powers.iter().try_fold(0_u32, |sum, &n| {
            sum.checked_add(n).ok_or(MultivariateError::TotalDegreeLimit)
        })?;
        if degree > self.limits.max_total_degree {
            return Err(MultivariateError::TotalDegreeLimit.into());
        }
        Ok(())
    }

    fn bits(&self, numerator: u64, denominator: u64) -> Result<(), GroebnerError> {
        if numerator > self.limits.max_scalar_bits || denominator > self.limits.max_scalar_bits {
            return Err(MultivariateError::ScalarBitLimit.into());
        }
        Ok(())
    }

    fn scalar_quotient(&mut self, a: &Rational, b: &Rational) -> Result<Rational, GroebnerError> {
        self.charge(1)?;
        self.bits(
            a.numerator()
                .bit_len()
                .saturating_add(b.denominator().bit_len()),
            a.denominator()
                .bit_len()
                .saturating_add(b.numerator().bit_len()),
        )?;
        Rational::new(a.numerator() * b.denominator(), a.denominator() * b.numerator())
            .map_err(|_| GroebnerError::InvalidCertificate)
    }

    fn term(
        &mut self,
        powers: &[u32],
        value: Rational,
    ) -> Result<MultivariatePolynomial, GroebnerError> {
        self.charge(3)?;
        self.degree(powers)?;
        Ok(MultivariatePolynomial::try_new(
            self.zero.variables().to_vec(),
            [(powers.iter().map(|&n| n as i32).collect(), value)],
            self.limits,
        )?)
    }

    fn one(&mut self) -> Result<MultivariatePolynomial, GroebnerError> {
        let powers = vec![0; self.zero.variables().len()];
        self.term(&powers, Rational::from(Integer::one()))
    }

    fn add(
        &mut self,
        a: &MultivariatePolynomial,
        b: &MultivariatePolynomial,
    ) -> Result<MultivariatePolynomial, GroebnerError> {
        self.charge(3 * (a.terms().len() + b.terms().len()) + 1)?;
        Ok(a.add(b, self.limits)?)
    }

    fn subtract(
        &mut self,
        a: &MultivariatePolynomial,
        b: &MultivariatePolynomial,
    ) -> Result<MultivariatePolynomial, GroebnerError> {
        self.charge(3 * (a.terms().len() + b.terms().len()) + 1)?;
        Ok(a.subtract(b, self.limits)?)
    }

    fn multiply(
        &mut self,
        a: &MultivariatePolynomial,
        b: &MultivariatePolynomial,
    ) -> Result<MultivariatePolynomial, GroebnerError> {
        self.charge(5 * a.terms().len() * b.terms().len() + a.terms().len() + b.terms().len() + 1)?;
        Ok(a.multiply(b, self.limits)?)
    }

    fn generators(&mut self, generators: &[MultivariatePolynomial]) -> Result<(), GroebnerError> {
        if generators.len() > MAX_GENERATORS {
            return Err(GroebnerError::GeneratorLimit);
        }
        self.charge(1)?;
        for p in generators {
            self.admit(p)?;
        }
        Ok(())
    }

    // Reserve a bounded conversion allowance for each input, leaving the
    // remaining work for certificate construction or verification.
    fn convert(
        &mut self,
        inputs: &[&DomainExpression],
    ) -> Result<Vec<MultivariatePolynomial>, GroebnerError> {
        if inputs.len() > MAX_GENERATORS + 1 {
            return Err(GroebnerError::GeneratorLimit);
        }
        let allowance = self.remaining / (2 * inputs.len().max(1) as u64);
        let limits = MultivariateLimits {
            max_work_units: allowance,
            ..self.limits
        };
        let mut result = Vec::with_capacity(inputs.len());
        for (index, input) in inputs.iter().enumerate() {
            if !input.conditions().is_empty() {
                return Err(GroebnerError::RestrictedDomain { input: index });
            }
            self.charge(allowance as usize)?;
            let converted = MultivariatePolynomial::try_from_domain_expression(
                input,
                self.zero.variables(),
                limits,
            )?;
            if !converted.conditions().is_empty() {
                return Err(GroebnerError::RestrictedDomain { input: index });
            }
            result.push(converted.polynomial().clone());
        }
        Ok(result)
    }
}

fn divides(a: &[u32], b: &[u32]) -> bool {
    a.iter().zip(b).all(|(x, y)| x <= y)
}

fn difference(a: &[u32], b: &[u32]) -> Vec<u32> {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}

fn leading(p: &MultivariatePolynomial) -> Result<(&[u32], &Rational), GroebnerError> {
    p.terms()
        .last_key_value()
        .map(|(powers, coefficient)| (powers.as_slice(), coefficient))
        .ok_or(GroebnerError::InvalidCertificate)
}

fn ordered_reduce(
    work: &mut Work,
    f: &MultivariatePolynomial,
    basis: &[MultivariatePolynomial],
) -> Result<GroebnerReduction, GroebnerError> {
    work.admit(f)?;
    work.charge(basis.len() + 1)?;
    let mut quotients = vec![work.zero.clone(); basis.len()];
    let mut remainder = work.zero.clone();
    let mut pending = f.clone();
    let mut steps = 0;
    while let Some((powers, coefficient)) = pending.terms().last_key_value() {
        work.charge(1)?;
        steps += 1;
        let old_lead = powers.clone();
        let mut reduced = false;
        for (i, divisor) in basis.iter().enumerate() {
            work.charge(1)?;
            let (divisor_powers, divisor_coefficient) = leading(divisor)?;
            if !divides(divisor_powers, powers) {
                continue;
            }
            let scalar = work.scalar_quotient(coefficient, divisor_coefficient)?;
            let factor = work.term(&difference(powers, divisor_powers), scalar)?;
            let product = work.multiply(&factor, divisor)?;
            pending = work.subtract(&pending, &product)?;
            quotients[i] = work.add(&quotients[i], &factor)?;
            reduced = true;
            break;
        }
        if !reduced {
            let (powers, coefficient) = leading(&pending)?;
            let term = work.term(powers, coefficient.clone())?;
            pending = work.subtract(&pending, &term)?;
            remainder = work.add(&remainder, &term)?;
        }
        if pending
            .terms()
            .last_key_value()
            .is_some_and(|(p, _)| p >= &old_lead)
        {
            return Err(GroebnerError::InvalidCertificate);
        }
    }
    Ok(GroebnerReduction {
        quotients,
        remainder,
        steps,
    })
}

struct PairPolynomial {
    lcm: Vec<u32>,
    left_factor: MultivariatePolynomial,
    right_factor: MultivariatePolynomial,
    polynomial: MultivariatePolynomial,
}

fn s_polynomial(
    work: &mut Work,
    left: &MultivariatePolynomial,
    right: &MultivariatePolynomial,
) -> Result<PairPolynomial, GroebnerError> {
    work.charge(1)?;
    let (a, c) = leading(left)?;
    let (b, d) = leading(right)?;
    let lcm: Vec<_> = a.iter().zip(b).map(|(&x, &y)| x.max(y)).collect();
    work.degree(&lcm)?;
    let one = Rational::from(Integer::one());
    let c = work.scalar_quotient(&one, c)?;
    let d = work.scalar_quotient(&one, d)?;
    let left_factor = work.term(&difference(&lcm, a), c)?;
    let right_factor = work.term(&difference(&lcm, b), d)?;
    let l = work.multiply(&left_factor, left)?;
    let r = work.multiply(&right_factor, right)?;
    let polynomial = work.subtract(&l, &r)?;
    Ok(PairPolynomial {
        lcm,
        left_factor,
        right_factor,
        polynomial,
    })
}

fn remainder_representation(
    work: &mut Work,
    pair: &PairPolynomial,
    left: usize,
    right: usize,
    rows: &[Vec<MultivariatePolynomial>],
    division: &GroebnerReduction,
) -> Result<Vec<MultivariatePolynomial>, GroebnerError> {
    let mut row = Vec::with_capacity(rows[left].len());
    for (a, b) in rows[left].iter().zip(&rows[right]) {
        let l = work.multiply(&pair.left_factor, a)?;
        let r = work.multiply(&pair.right_factor, b)?;
        row.push(work.subtract(&l, &r)?);
    }
    for (q, basis_row) in division.quotients.iter().zip(rows) {
        if q.is_zero() {
            continue;
        }
        for (entry, multiplier) in row.iter_mut().zip(basis_row) {
            let product = work.multiply(q, multiplier)?;
            *entry = work.subtract(entry, &product)?;
        }
    }
    Ok(row)
}

fn build(
    work: &mut Work,
    generators: &[MultivariatePolynomial],
) -> Result<GroebnerCertificate, GroebnerError> {
    work.generators(generators)?;
    let mut basis = Vec::new();
    let mut representations = Vec::new();
    for (i, generator) in generators.iter().enumerate() {
        if generator.is_zero() {
            continue;
        }
        let mut row = vec![work.zero.clone(); generators.len()];
        work.charge(generators.len())?;
        row[i] = work.one()?;
        basis.push(generator.clone());
        representations.push(row);
    }

    // Process every pair once, including pairs introduced by later remainders.
    let mut right = 1;
    while right < basis.len() {
        for left in 0..right {
            let pair = s_polynomial(work, &basis[left], &basis[right])?;
            let division = ordered_reduce(work, &pair.polynomial, &basis)?;
            if division.remainder.is_zero() {
                continue;
            }
            if basis.len() >= MAX_BASIS {
                return Err(GroebnerError::BasisLimit);
            }
            let row =
                remainder_representation(work, &pair, left, right, &representations, &division)?;
            basis.push(division.remainder);
            representations.push(row);
        }
        right += 1;
    }

    let generator_reductions = generators
        .iter()
        .map(|g| ordered_reduce(work, g, &basis))
        .collect::<Result<Vec<_>, _>>()?;
    let mut pairs = Vec::new();
    for right in 1..basis.len() {
        for left in 0..right {
            let pair = s_polynomial(work, &basis[left], &basis[right])?;
            pairs.push(GroebnerPair {
                left,
                right,
                lcm: pair.lcm,
                reduction: ordered_reduce(work, &pair.polynomial, &basis)?,
            });
        }
    }
    let certificate = GroebnerCertificate {
        variables: work.zero.variables().to_vec(),
        basis,
        representations,
        generator_reductions,
        pairs,
    };
    if !verify(work, generators, &certificate)? {
        return Err(GroebnerError::InvalidCertificate);
    }
    Ok(certificate)
}

fn check_reduction(
    work: &mut Work,
    f: &MultivariatePolynomial,
    basis: &[MultivariatePolynomial],
    claimed: &GroebnerReduction,
) -> Result<bool, GroebnerError> {
    work.admit(f)?;
    if claimed.quotients.len() != basis.len() || claimed.steps as u64 > work.limits.max_work_units {
        return Ok(false);
    }
    for q in claimed
        .quotients
        .iter()
        .chain(std::iter::once(&claimed.remainder))
    {
        work.admit(q)?;
    }
    let mut sum = claimed.remainder.clone();
    for (q, g) in claimed.quotients.iter().zip(basis) {
        let product = work.multiply(q, g)?;
        sum = work.add(&sum, &product)?;
    }
    if sum != *f {
        return Ok(false);
    }
    for powers in claimed.remainder.terms().keys() {
        for g in basis {
            work.charge(1)?;
            if divides(leading(g)?.0, powers) {
                return Ok(false);
            }
        }
    }
    // Replay additionally checks divisor priority and strict descent.
    Ok(ordered_reduce(work, f, basis)? == *claimed)
}

fn verify(
    work: &mut Work,
    generators: &[MultivariatePolynomial],
    cert: &GroebnerCertificate,
) -> Result<bool, GroebnerError> {
    work.generators(generators)?;
    let n = cert.basis.len();
    if n > MAX_BASIS {
        return Err(GroebnerError::BasisLimit);
    }
    if cert.variables.as_slice() != work.zero.variables()
        || cert.representations.len() != n
        || cert.generator_reductions.len() != generators.len()
        || cert.pairs.len() != n * n.saturating_sub(1) / 2
    {
        return Ok(false);
    }
    for (g, row) in cert.basis.iter().zip(&cert.representations) {
        work.admit(g)?;
        if g.is_zero() || row.len() != generators.len() {
            return Ok(false);
        }
        let mut sum = work.zero.clone();
        for (coefficient, original) in row.iter().zip(generators) {
            work.admit(coefficient)?;
            let product = work.multiply(coefficient, original)?;
            sum = work.add(&sum, &product)?;
        }
        if sum != *g {
            return Ok(false);
        }
    }
    for (g, reduction) in generators.iter().zip(&cert.generator_reductions) {
        if !reduction.remainder.is_zero() || !check_reduction(work, g, &cert.basis, reduction)? {
            return Ok(false);
        }
    }
    let mut index = 0;
    for right in 1..n {
        for left in 0..right {
            let evidence = &cert.pairs[index];
            index += 1;
            if (evidence.left, evidence.right) != (left, right)
                || !evidence.reduction.remainder.is_zero()
            {
                return Ok(false);
            }
            let pair = s_polynomial(work, &cert.basis[left], &cert.basis[right])?;
            if evidence.lcm != pair.lcm
                || !check_reduction(work, &pair.polynomial, &cert.basis, &evidence.reduction)?
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Computes a certified basis for the formal ideal in the specified Q ring.
/// Empty input and all-zero generators describe the zero ideal. Caller limits
/// are clamped to 3 variables, 8 generators, 12 basis elements, 128 terms,
/// 1024 intermediate input terms, degree 24, 512 scalar bits and 250000
/// aggregate work units. No partial basis is returned.
pub fn groebner_basis(
    variables: &[Symbol],
    generators: &[MultivariatePolynomial],
    limits: MultivariateLimits,
) -> Result<GroebnerCertificate, GroebnerError> {
    build(&mut Work::new(variables, limits)?, generators)
}

/// Independently checks both ideal inclusions and every final S-pair without
/// running Buchberger's basis-generation loop.
pub fn verify_groebner_basis(
    variables: &[Symbol],
    generators: &[MultivariatePolynomial],
    certificate: &GroebnerCertificate,
    limits: MultivariateLimits,
) -> Result<bool, GroebnerError> {
    verify(&mut Work::new(variables, limits)?, generators, certificate)
}

/// Restricted original expressions are refused, not converted into an
/// unconditional ideal. Bare Expr normalization cannot recover lost history.
pub fn groebner_basis_from_domain_expressions(
    variables: &[Symbol],
    generators: &[DomainExpression],
    limits: MultivariateLimits,
) -> Result<GroebnerCertificate, GroebnerError> {
    if generators.len() > MAX_GENERATORS {
        return Err(GroebnerError::GeneratorLimit);
    }
    let mut work = Work::new(variables, limits)?;
    let sources: Vec<_> = generators.iter().collect();
    let generators = work.convert(&sources)?;
    build(&mut work, &generators)
}

pub fn verify_groebner_basis_from_domain_expressions(
    variables: &[Symbol],
    generators: &[DomainExpression],
    certificate: &GroebnerCertificate,
    limits: MultivariateLimits,
) -> Result<bool, GroebnerError> {
    if generators.len() > MAX_GENERATORS {
        return Err(GroebnerError::GeneratorLimit);
    }
    let mut work = Work::new(variables, limits)?;
    let sources: Vec<_> = generators.iter().collect();
    let generators = work.convert(&sources)?;
    verify(&mut work, &generators, certificate)
}

fn membership(
    work: &mut Work,
    generators: &[MultivariatePolynomial],
    certificate: &GroebnerCertificate,
    target: &MultivariatePolynomial,
) -> Result<IdealMembership, GroebnerError> {
    if !verify(work, generators, certificate)? {
        return Err(GroebnerError::InvalidCertificate);
    }
    let reduction = ordered_reduce(work, target, &certificate.basis)?;
    if !check_reduction(work, target, &certificate.basis, &reduction)? {
        return Err(GroebnerError::InvalidCertificate);
    }
    if reduction.remainder.is_zero() {
        Ok(IdealMembership::Member(reduction))
    } else {
        Ok(IdealMembership::NonMember(reduction))
    }
}

/// Only a verified Groebner basis permits a nonmembership conclusion from a
/// nonzero remainder. A zero remainder proves membership in the input ideal.
pub fn ideal_membership(
    variables: &[Symbol],
    generators: &[MultivariatePolynomial],
    certificate: &GroebnerCertificate,
    target: &MultivariatePolynomial,
    limits: MultivariateLimits,
) -> Result<IdealMembership, GroebnerError> {
    membership(
        &mut Work::new(variables, limits)?,
        generators,
        certificate,
        target,
    )
}

/// The domain-aware query refuses historical restrictions on either the ideal
/// generators or the target. The caller retains the original expressions.
pub fn ideal_membership_from_domain_expressions(
    variables: &[Symbol],
    generators: &[DomainExpression],
    certificate: &GroebnerCertificate,
    target: &DomainExpression,
    limits: MultivariateLimits,
) -> Result<IdealMembership, GroebnerError> {
    if generators.len() > MAX_GENERATORS {
        return Err(GroebnerError::GeneratorLimit);
    }
    let mut work = Work::new(variables, limits)?;
    let mut sources: Vec<_> = generators.iter().collect();
    sources.push(target);
    let mut polynomials = work.convert(&sources)?;
    let target = polynomials.pop().ok_or(GroebnerError::InvalidCertificate)?;
    membership(&mut work, &polynomials, certificate, &target)
}

pub fn verify_ideal_membership(
    variables: &[Symbol],
    generators: &[MultivariatePolynomial],
    certificate: &GroebnerCertificate,
    target: &MultivariatePolynomial,
    claimed: &IdealMembership,
    limits: MultivariateLimits,
) -> Result<bool, GroebnerError> {
    let mut work = Work::new(variables, limits)?;
    if !verify(&mut work, generators, certificate)? {
        return Ok(false);
    }
    check_membership(&mut work, certificate, target, claimed)
}

fn check_membership(
    work: &mut Work,
    certificate: &GroebnerCertificate,
    target: &MultivariatePolynomial,
    claimed: &IdealMembership,
) -> Result<bool, GroebnerError> {
    let (reduction, member) = match claimed {
        IdealMembership::Member(reduction) => (reduction, true),
        IdealMembership::NonMember(reduction) => (reduction, false),
    };
    Ok(reduction.remainder.is_zero() == member
        && check_reduction(work, target, &certificate.basis, reduction)?)
}

/// Verify a query against its original domain-aware inputs rather than an
/// unchecked normalized expression.
pub fn verify_ideal_membership_from_domain_expressions(
    variables: &[Symbol],
    generators: &[DomainExpression],
    certificate: &GroebnerCertificate,
    target: &DomainExpression,
    claimed: &IdealMembership,
    limits: MultivariateLimits,
) -> Result<bool, GroebnerError> {
    if generators.len() > MAX_GENERATORS {
        return Err(GroebnerError::GeneratorLimit);
    }
    let mut work = Work::new(variables, limits)?;
    let mut sources: Vec<_> = generators.iter().collect();
    sources.push(target);
    let mut polynomials = work.convert(&sources)?;
    let target = polynomials.pop().ok_or(GroebnerError::InvalidCertificate)?;
    if !verify(&mut work, &polynomials, certificate)? {
        return Ok(false);
    }
    check_membership(&mut work, certificate, &target, claimed)
}
