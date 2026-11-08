//! Canonical exact roots of Q-polynomials through degree two.
//!
//! Irreducible quadratics are represented by the monic minimal polynomial
//! plus a branch. No floating-point approximations or radical factorization.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::budget::WorkBudget;
use crate::poly_division::rational_quotient;
use crate::{EvaluationError, QuadraticError, RationalPolynomial};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuadraticBranch {
    LowerReal,
    UpperReal,
    NegativeImaginary,
    PositiveImaginary,
}

impl QuadraticBranch {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::LowerReal => "lower_real",
            Self::UpperReal => "upper_real",
            Self::NegativeImaginary => "negative_imaginary",
            Self::PositiveImaginary => "positive_imaginary",
        }
    }
}

/// Canonical equality for degree-at-most-two algebraic values. No general
/// arithmetic or order of unrelated irrational values is claimed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExactQuadraticRoot {
    Rational(Rational),
    Algebraic {
        minimal_polynomial: RationalPolynomial,
        branch: QuadraticBranch,
    },
}

impl ExactQuadraticRoot {
    pub fn as_rational(&self) -> Option<&Rational> {
        match self {
            Self::Rational(value) => Some(value),
            Self::Algebraic { .. } => None,
        }
    }

    pub fn minimal_polynomial(&self) -> Option<&RationalPolynomial> {
        match self {
            Self::Rational(_) => None,
            Self::Algebraic { minimal_polynomial, .. } => Some(minimal_polynomial),
        }
    }

    pub fn branch(&self) -> Option<QuadraticBranch> {
        match self {
            Self::Rational(_) => None,
            Self::Algebraic { branch, .. } => Some(*branch),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlgebraicRootSet {
    Finite(Vec<ExactQuadraticRoot>),
    AllComplexNumbers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlgebraicRootRule {
    NormalizeMonic,
    CalculateDiscriminant,
    ZeroPolynomial,
    NonzeroConstant,
    Linear,
    RepeatedRationalRoot,
    DistinctRationalRoots,
    IrrationalRealPair,
    NonrealConjugatePair,
}

impl AlgebraicRootRule {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::NormalizeMonic => "roots.normalize_monic",
            Self::CalculateDiscriminant => "roots.quadratic.discriminant",
            Self::ZeroPolynomial => "roots.zero_polynomial",
            Self::NonzeroConstant => "roots.nonzero_constant",
            Self::Linear => "roots.linear",
            Self::RepeatedRationalRoot => "roots.quadratic.repeated",
            Self::DistinctRationalRoots => "roots.quadratic.rational_pair",
            Self::IrrationalRealPair => "roots.quadratic.irrational_real_pair",
            Self::NonrealConjugatePair => "roots.quadratic.nonreal_conjugates",
        }
    }
}

/// Ordered typed evidence, not a false equality claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlgebraicRootEvidence {
    MonicNormalization(RationalPolynomial),
    Discriminant(Rational),
    Classification(AlgebraicRootRule),
}

impl AlgebraicRootEvidence {
    pub const fn rule(&self) -> AlgebraicRootRule {
        match self {
            Self::MonicNormalization(_) => AlgebraicRootRule::NormalizeMonic,
            Self::Discriminant(_) => AlgebraicRootRule::CalculateDiscriminant,
            Self::Classification(rule) => *rule,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlgebraicRootResult {
    original: RationalPolynomial,
    roots: AlgebraicRootSet,
    evidence: Vec<AlgebraicRootEvidence>,
}

impl AlgebraicRootResult {
    pub fn original(&self) -> &RationalPolynomial {
        &self.original
    }
    pub fn roots(&self) -> &AlgebraicRootSet {
        &self.roots
    }
    pub fn evidence(&self) -> &[AlgebraicRootEvidence] {
        &self.evidence
    }
}

fn zero() -> Rational {
    Rational::from(Integer::zero())
}
fn coefficient(p: &RationalPolynomial, index: usize) -> Rational {
    p.coefficients().get(index).cloned().unwrap_or_else(zero)
}
fn algebraic_pair(p: &RationalPolynomial, complex: bool) -> AlgebraicRootSet {
    let branches = if complex {
        [QuadraticBranch::NegativeImaginary, QuadraticBranch::PositiveImaginary]
    } else {
        [QuadraticBranch::LowerReal, QuadraticBranch::UpperReal]
    };
    AlgebraicRootSet::Finite(branches.into_iter().map(|branch| {
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial: p.clone(),
            branch,
        }
    }).collect())
}

pub(crate) fn checked_algebraic_roots(
    original: &RationalPolynomial,
    work: &mut WorkBudget,
) -> Result<AlgebraicRootResult, EvaluationError> {
    if original.degree().is_some_and(|degree| degree > 2) {
        return Err(QuadraticError::DegreeExceeded.into());
    }
    work.charge(1)?;
    let mut evidence = Vec::new();
    let roots = if original.is_zero() {
        evidence.push(AlgebraicRootEvidence::Classification(AlgebraicRootRule::ZeroPolynomial));
        AlgebraicRootSet::AllComplexNumbers
    } else {
        let leading = original.coefficients().last().expect("nonzero polynomial");
        let mut coefficients = Vec::with_capacity(original.coefficients().len());
        for value in original.coefficients() {
            coefficients.push(work.quotient(value, leading)?);
        }
        let monic = RationalPolynomial::new(coefficients);
        evidence.push(AlgebraicRootEvidence::MonicNormalization(monic.clone()));
        match monic.degree() {
            Some(0) => {
                evidence.push(AlgebraicRootEvidence::Classification(AlgebraicRootRule::NonzeroConstant));
                AlgebraicRootSet::Finite(Vec::new())
            }
            Some(1) => {
                evidence.push(AlgebraicRootEvidence::Classification(AlgebraicRootRule::Linear));
                AlgebraicRootSet::Finite(vec![ExactQuadraticRoot::Rational(-coefficient(&monic, 0))])
            }
            Some(2) => {
                let b = coefficient(&monic, 1);
                let c = coefficient(&monic, 0);
                let b2 = work.multiply_scalar(&b, &b)?;
                let four_c = work.multiply_scalar(&Rational::from(Integer::from(4)), &c)?;
                let delta = work.subtract_scalar(&b2, &four_c)?;
                evidence.push(AlgebraicRootEvidence::Discriminant(delta.clone()));
                let bits = delta.numerator().bit_len().saturating_add(delta.denominator().bit_len());
                work.charge(bits.saturating_mul(bits).saturating_div(64).saturating_add(1))?;
                if let Some(sqrt) = delta.exact_sqrt() {
                    let negative_b = -&b;
                    let less = work.subtract_scalar(&negative_b, &sqrt)?;
                    let greater = work.add_scalar(&negative_b, &sqrt)?;
                    let two = Rational::from(Integer::from(2));
                    let mut roots = vec![work.quotient(&less, &two)?, work.quotient(&greater, &two)?];
                    roots.sort();
                    roots.dedup();
                    let rule = if roots.len() == 1 {
                        AlgebraicRootRule::RepeatedRationalRoot
                    } else {
                        AlgebraicRootRule::DistinctRationalRoots
                    };
                    evidence.push(AlgebraicRootEvidence::Classification(rule));
                    AlgebraicRootSet::Finite(roots.into_iter().map(ExactQuadraticRoot::Rational).collect())
                } else {
                    let complex = delta < zero();
                    evidence.push(AlgebraicRootEvidence::Classification(
                        if complex { AlgebraicRootRule::NonrealConjugatePair }
                        else { AlgebraicRootRule::IrrationalRealPair }
                    ));
                    algebraic_pair(&monic, complex)
                }
            }
            _ => return Err(QuadraticError::DegreeExceeded.into()),
        }
    };
    Ok(AlgebraicRootResult {
        original: original.clone(),
        roots,
        evidence,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlgebraicVerificationError {
    UnsupportedDegree,
    IncorrectEvidence,
    IncorrectRoots,
}
impl fmt::Display for AlgebraicVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDegree => f.write_str("algebraic root verification degree exceeded"),
            Self::IncorrectEvidence => f.write_str("incorrect algebraic root evidence"),
            Self::IncorrectRoots => f.write_str("incorrect exact algebraic root set"),
        }
    }
}
impl Error for AlgebraicVerificationError {}

/// Independent reference reconstruction using trusted exact rational math.
/// Verifies proof order, monic identity, discriminant, complete root set, and
/// direct substitution of all rational solutions.
pub fn verify_algebraic_roots(claim: &AlgebraicRootResult) -> Result<(), AlgebraicVerificationError> {
    use AlgebraicVerificationError as E;
    let original = &claim.original;
    if original.degree().is_some_and(|degree| degree > 2) {
        return Err(E::UnsupportedDegree);
    }
    let mut evidence = Vec::new();
    let expected = if original.is_zero() {
        evidence.push(AlgebraicRootEvidence::Classification(AlgebraicRootRule::ZeroPolynomial));
        AlgebraicRootSet::AllComplexNumbers
    } else {
        let leading = original.coefficients().last().ok_or(E::IncorrectEvidence)?;
        let normalized = RationalPolynomial::new(original.coefficients().iter()
            .map(|value| rational_quotient(value, leading)).collect());
        evidence.push(AlgebraicRootEvidence::MonicNormalization(normalized.clone()));
        match normalized.degree() {
            Some(0) => {
                evidence.push(AlgebraicRootEvidence::Classification(AlgebraicRootRule::NonzeroConstant));
                AlgebraicRootSet::Finite(Vec::new())
            }
            Some(1) => {
                evidence.push(AlgebraicRootEvidence::Classification(AlgebraicRootRule::Linear));
                AlgebraicRootSet::Finite(vec![
                    ExactQuadraticRoot::Rational(-&normalized.coefficients()[0])
                ])
            }
            Some(2) => {
                let b = &normalized.coefficients()[1];
                let c = &normalized.coefficients()[0];
                let four = Rational::from(Integer::from(4));
                let delta = &(b * b) - &(&four * c);
                evidence.push(AlgebraicRootEvidence::Discriminant(delta.clone()));
                match delta.exact_sqrt() {
                    Some(sqrt) => {
                        let negative_b = -b;
                        let two = Rational::from(Integer::from(2));
                        let mut roots = vec![
                            rational_quotient(&(&negative_b - &sqrt), &two),
                            rational_quotient(&(&negative_b + &sqrt), &two),
                        ];
                        roots.sort();
                        roots.dedup();
                        evidence.push(AlgebraicRootEvidence::Classification(
                            if roots.len() == 1 { AlgebraicRootRule::RepeatedRationalRoot }
                            else { AlgebraicRootRule::DistinctRationalRoots }
                        ));
                        AlgebraicRootSet::Finite(roots.into_iter().map(ExactQuadraticRoot::Rational).collect())
                    }
                    None => {
                        let complex = delta < zero();
                        evidence.push(AlgebraicRootEvidence::Classification(
                            if complex { AlgebraicRootRule::NonrealConjugatePair }
                            else { AlgebraicRootRule::IrrationalRealPair }
                        ));
                        algebraic_pair(&normalized, complex)
                    }
                }
            }
            _ => return Err(E::UnsupportedDegree),
        }
    };
    if evidence != claim.evidence {
        return Err(E::IncorrectEvidence);
    }
    if expected != claim.roots {
        return Err(E::IncorrectRoots);
    }
    if let AlgebraicRootSet::Finite(roots) = &claim.roots {
        for root in roots {
            if let ExactQuadraticRoot::Rational(value) = root
                && !original.evaluate(value).numerator().is_zero()
            {
                return Err(E::IncorrectRoots);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod adversarial {
    use super::*;
    use crate::EvaluationContext;

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(values.iter().map(|&v| Rational::from(Integer::from(v))).collect())
    }

    #[test]
    fn independently_rejects_forged_evidence_roots_and_order() {
        let genuine = EvaluationContext::default().algebraic_roots(&p(&[-2, 0, 1])).unwrap();
        let mut forged = genuine.clone();
        forged.evidence.pop();
        assert_eq!(verify_algebraic_roots(&forged), Err(AlgebraicVerificationError::IncorrectEvidence));
        let mut forged = genuine.clone();
        if let AlgebraicRootSet::Finite(ref mut roots) = forged.roots {
            roots[0] = ExactQuadraticRoot::Algebraic {
                minimal_polynomial: p(&[-3, 0, 1]),
                branch: QuadraticBranch::LowerReal,
            };
        }
        assert_eq!(verify_algebraic_roots(&forged), Err(AlgebraicVerificationError::IncorrectRoots));
        let mut forged = genuine;
        if let AlgebraicRootSet::Finite(ref mut roots) = forged.roots {
            roots.swap(0, 1);
        }
        assert_eq!(verify_algebraic_roots(&forged), Err(AlgebraicVerificationError::IncorrectRoots));
    }
}
