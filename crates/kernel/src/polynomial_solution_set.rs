//! Certified exact roots of Q[x], with an explicit unresolved cofactor.
//!
//! The zero set is over the complex numbers. Certified degree-three or higher
//! factors still have unresolved roots: factor irreducibility is not a root formula.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::budget::WorkBudget;
use crate::{
    AlgebraicRootResult, AlgebraicRootSet, EvaluationContext, EvaluationError,
    ExactQuadraticRoot, FactorizationVerificationError, RationalFactor,
    RationalFactorization, RationalFactorizationError, RationalPolynomial,
    verify_algebraic_roots, verify_rational_factorization,
};

fn one() -> RationalPolynomial {
    RationalPolynomial::new(vec![Rational::from(Integer::one())])
}

/// An exactly represented distinct root, with its positive algebraic
/// multiplicity in the source polynomial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialRootMultiplicity {
    root: ExactQuadraticRoot,
    multiplicity: usize,
}

impl PolynomialRootMultiplicity {
    pub fn new(root: ExactQuadraticRoot, multiplicity: usize) -> Self {
        Self { root, multiplicity }
    }

    pub fn root(&self) -> &ExactQuadraticRoot {
        &self.root
    }

    pub fn multiplicity(&self) -> usize {
        self.multiplicity
    }
}

/// Zero sets over C. For Finite, roots contains only certified roots.
/// When incomplete, the monic unresolved cofactor may contain additional
/// roots; a root-free conclusion about that cofactor is NOT established.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolynomialZeroSet {
    AllComplexNumbers,
    Finite {
        roots: Vec<PolynomialRootMultiplicity>,
        unresolved_cofactor: Option<RationalPolynomial>,
        complete: bool,
    },
}

impl PolynomialZeroSet {
    pub fn is_complete(&self) -> bool {
        match self {
            Self::AllComplexNumbers => true,
            Self::Finite { complete, .. } => *complete,
        }
    }
}

/// Independent degree-one/two root evidence for a certified monic factor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolynomialFactorRootEvidence {
    factor: RationalPolynomial,
    multiplicity: usize,
    roots: AlgebraicRootResult,
}

impl PolynomialFactorRootEvidence {
    pub fn from_parts(
        factor: RationalPolynomial,
        multiplicity: usize,
        roots: AlgebraicRootResult,
    ) -> Self {
        Self {
            factor,
            multiplicity,
            roots,
        }
    }

    pub fn factor(&self) -> &RationalPolynomial {
        &self.factor
    }

    pub fn multiplicity(&self) -> usize {
        self.multiplicity
    }

    pub fn roots(&self) -> &AlgebraicRootResult {
        &self.roots
    }
}

/// A proposed solution set plus its exact factorization and root certificates.
/// Construction via from_unverified_parts does not establish validity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialPolynomialRootResult {
    original: RationalPolynomial,
    zero_set: PolynomialZeroSet,
    factorization: Option<RationalFactorization>,
    root_evidence: Vec<PolynomialFactorRootEvidence>,
}

impl PartialPolynomialRootResult {
    pub fn from_unverified_parts(
        original: RationalPolynomial,
        zero_set: PolynomialZeroSet,
        factorization: Option<RationalFactorization>,
        root_evidence: Vec<PolynomialFactorRootEvidence>,
    ) -> Self {
        Self {
            original,
            zero_set,
            factorization,
            root_evidence,
        }
    }

    pub fn original(&self) -> &RationalPolynomial {
        &self.original
    }

    pub fn zero_set(&self) -> &PolynomialZeroSet {
        &self.zero_set
    }

    pub fn factorization(&self) -> Option<&RationalFactorization> {
        self.factorization.as_ref()
    }

    pub fn root_evidence(&self) -> &[PolynomialFactorRootEvidence] {
        &self.root_evidence
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartialPolynomialRootError {
    Evaluation(EvaluationError),
    Factorization(RationalFactorizationError),
    InvalidFactorization(FactorizationVerificationError),
    IncorrectCertificate,
}

impl fmt::Display for PartialPolynomialRootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::Factorization(error) => write!(f, "{error}"),
            Self::InvalidFactorization(error) => write!(f, "{error}"),
            Self::IncorrectCertificate => f.write_str("partial polynomial root certificate is invalid"),
        }
    }
}

impl Error for PartialPolynomialRootError {}

impl From<EvaluationError> for PartialPolynomialRootError {
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}
impl From<RationalFactorizationError> for PartialPolynomialRootError {
    fn from(error: RationalFactorizationError) -> Self {
        Self::Factorization(error)
    }
}
impl From<FactorizationVerificationError> for PartialPolynomialRootError {
    fn from(error: FactorizationVerificationError) -> Self {
        Self::InvalidFactorization(error)
    }
}

// Exclude every factor without a supported exact degree-one/two root
// constructor, including certified irreducible higher-degree factors.
// The returned product is monic; the original scalar unit remains in the
// factorization certificate. Checked multiplication bounds intermediates.
fn unresolved_product(
    context: &EvaluationContext,
    factorization: &RationalFactorization,
) -> Result<Option<RationalPolynomial>, PartialPolynomialRootError> {
    let mut product = one();
    let mut has_unresolved = false;
    let mut work = WorkBudget::new(context.limits());
    for factor in factorization.factors().iter().filter(|factor| {
        factor.polynomial().degree().is_some_and(|degree| degree > 2)
    }).chain(factorization.unresolved()) {
        has_unresolved = true;
        for _ in 0..factor.multiplicity() {
            product = work.multiply(&product, factor.polynomial())?;
        }
    }
    if has_unresolved {
        context.check_polynomial(&product)?;
        Ok(Some(product))
    } else {
        Ok(None)
    }
}

fn add_distinct_roots(
    result: &mut Vec<PolynomialRootMultiplicity>,
    roots: &[ExactQuadraticRoot],
    multiplicity: usize,
) -> bool {
    if multiplicity == 0 {
        return false;
    }
    for root in roots {
        if result.iter().any(|entry| entry.root() == root) {
            return false;
        }
        result.push(PolynomialRootMultiplicity::new(root.clone(), multiplicity));
    }
    true
}

/// Finds every currently constructible exact root and retains all other
/// factor powers as an unresolved cofactor. The classification is complete
/// only if no unresolved nonconstant factor remains.
pub fn solve_partial_polynomial_roots(
    context: &EvaluationContext,
    original: &RationalPolynomial,
) -> Result<PartialPolynomialRootResult, PartialPolynomialRootError> {
    context.check_polynomial(original)?;
    let mut work = WorkBudget::new(context.limits());
    work.charge(1)?;

    if original.is_zero() {
        return Ok(PartialPolynomialRootResult::from_unverified_parts(
            original.clone(),
            PolynomialZeroSet::AllComplexNumbers,
            None,
            Vec::new(),
        ));
    }

    if original.degree() == Some(0) {
        return Ok(PartialPolynomialRootResult::from_unverified_parts(
            original.clone(),
            PolynomialZeroSet::Finite {
                roots: Vec::new(),
                unresolved_cofactor: None,
                complete: true,
            },
            None,
            Vec::new(),
        ));
    }

    let factorization = context.factor_rational_polynomial(original)?;
    let mut roots = Vec::new();
    let mut root_evidence = Vec::new();
    for factor in factorization.factors() {
        if factor.polynomial().degree().is_none_or(|degree| degree > 2) {
            continue;
        }
        work.charge(1)?;
        let proof = context.algebraic_roots(factor.polynomial())?;
        let AlgebraicRootSet::Finite(values) = proof.roots() else {
            return Err(PartialPolynomialRootError::IncorrectCertificate);
        };
        if !add_distinct_roots(&mut roots, values, factor.multiplicity()) {
            return Err(PartialPolynomialRootError::IncorrectCertificate);
        }
        root_evidence.push(PolynomialFactorRootEvidence::from_parts(
            factor.polynomial().clone(),
            factor.multiplicity(),
            proof,
        ));
    }

    let unresolved_cofactor = unresolved_product(context, &factorization)?;
    let result = PartialPolynomialRootResult::from_unverified_parts(
        original.clone(),
        PolynomialZeroSet::Finite {
            roots,
            complete: unresolved_cofactor.is_none(),
            unresolved_cofactor,
        },
        Some(factorization),
        root_evidence,
    );
    if !verify_partial_polynomial_roots(context, original, &result)? {
        return Err(PartialPolynomialRootError::IncorrectCertificate);
    }
    Ok(result)
}

/// Independently recomputes and certifies the complete factorization product,
/// verifies the individual exact root constructors, and checks every claimed
/// root, multiplicity, unresolved factor and completeness flag.
///
/// Invalid proposed evidence returns Ok(false). Resource exhaustion and
/// factorization algorithm failures retain their own typed errors.
pub fn verify_partial_polynomial_roots(
    context: &EvaluationContext,
    original: &RationalPolynomial,
    proposed: &PartialPolynomialRootResult,
) -> Result<bool, PartialPolynomialRootError> {
    context.check_polynomial(original)?;
    let mut work = WorkBudget::new(context.limits());
    work.charge(1)?;
    if proposed.original() != original {
        return Ok(false);
    }
    if original.is_zero() {
        return Ok(proposed.factorization.is_none()
            && proposed.root_evidence.is_empty()
            && proposed.zero_set == PolynomialZeroSet::AllComplexNumbers);
    }
    if original.degree() == Some(0) {
        return Ok(proposed.factorization.is_none()
            && proposed.root_evidence.is_empty()
            && proposed.zero_set == PolynomialZeroSet::Finite {
                roots: Vec::new(),
                unresolved_cofactor: None,
                complete: true,
            });
    }

    let factorization = context.factor_rational_polynomial(original)?;
    if proposed.factorization.as_ref() != Some(&factorization) {
        return Ok(false);
    }
    // This checker reconstructs the exact polynomial independently of the
    // factorization generator, including each multiplicity and scalar unit.
    verify_rational_factorization(original, &factorization)?;

    let expected_evidence_count = factorization.factors().iter().filter(|factor| {
        factor.polynomial().degree().is_some_and(|degree| degree <= 2)
    }).count();
    if proposed.root_evidence.len() != expected_evidence_count {
        return Ok(false);
    }
    let mut roots = Vec::new();
    let mut evidence = proposed.root_evidence.iter();
    for factor in factorization.factors() {
        if factor.polynomial().degree().is_none_or(|degree| degree > 2) {
            continue;
        }
        work.charge(1)?;
        let Some(claim) = evidence.next() else {
            return Ok(false);
        };
        if claim.factor() != factor.polynomial()
            || claim.multiplicity() != factor.multiplicity()
            || claim.roots().original() != factor.polynomial()
            || claim.roots().evidence().len() > 3
        {
            return Ok(false);
        }
        let AlgebraicRootSet::Finite(values) = claim.roots().roots() else {
            return Ok(false);
        };
        if values.len() > 2 || verify_algebraic_roots(claim.roots()).is_err() {
            return Ok(false);
        }
        if !add_distinct_roots(&mut roots, values, factor.multiplicity()) {
            return Ok(false);
        }
    }
    let unresolved_cofactor = unresolved_product(context, &factorization)?;
    Ok(proposed.zero_set == PolynomialZeroSet::Finite {
        roots,
        complete: unresolved_cofactor.is_none(),
        unresolved_cofactor,
    })
}

/// All roots are over C. No real-only completeness claim is made.
impl EvaluationContext {
    pub fn partial_polynomial_roots(
        &self,
        original: &RationalPolynomial,
    ) -> Result<PartialPolynomialRootResult, PartialPolynomialRootError> {
        solve_partial_polynomial_roots(self, original)
    }
}
