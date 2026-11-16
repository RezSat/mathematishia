//! Checked final-result explanations for ordered multivariate division.
//!
//! The division API does not expose intermediate reductions. These records
//! certify the final exact recomposition and reduced-remainder property only;
//! they do not claim a unique normal form for arbitrary divisors.

use std::error::Error;
use std::fmt::{self, Write};

use crate::{
    MultivariateDivision, MultivariateDivisionError, MultivariateLimits, MultivariatePolynomial,
    Relation, divide_multivariate, verify_multivariate_division,
};

const MAX_RENDER_LINES: usize = 72;
const MAX_RENDER_BYTES: usize = 65_536;
const MAX_LINE_BYTES: usize = 16_384;
const MAX_VARIABLE_NAME_BYTES: usize = 256;

/// The order implemented by the existing division algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivisionMonomialOrder {
    /// Lexicographic order, using the ambient ring's canonical symbol order.
    Lexicographic,
}

/// An ordered, exact quotient-divisor product in a recomposition identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivisionProduct {
    pub divisor_index: usize,
    pub quotient: MultivariatePolynomial,
    pub divisor: MultivariatePolynomial,
}

/// A polynomial equality, not equation equivalence or normal-form uniqueness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivisionRecompositionClaim {
    pub relation: Relation,
    pub dividend: MultivariatePolynomial,
    pub ordered_products: Vec<DivisionProduct>,
    pub remainder: MultivariatePolynomial,
}

/// No term of the remainder is divisible by any listed leading monomial.
///
/// The exponent vectors are in the ambient variable order and each entry is
/// associated with the divisor at the same position. This does not assert a
/// unique remainder for arbitrary generators.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivisionReducedRemainderClaim {
    pub remainder: MultivariatePolynomial,
    pub leading_exponents: Vec<Vec<u32>>,
    pub monomial_order: DivisionMonomialOrder,
}

/// Each line refers to checked structured data, rather than embedding proof
/// claims in explanatory strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DivisionExplanationKind {
    Dividend,
    AmbientRingAndOrder,
    Divisor { index: usize },
    Quotient { index: usize },
    Remainder,
    Recomposition,
    ReducedRemainder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivisionExplanationLine {
    pub kind: DivisionExplanationKind,
    pub explanation: &'static str,
    pub display: String,
}

/// A checked final-result presentation, valid only for its supplied evidence.
///
/// Public members are not intrinsically certified after modification. Call
/// verify_multivariate_derivation to independently recheck the mathematical
/// evidence and reconstruct all presentation fields before trusting a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultivariateDivisionDerivation {
    pub dividend: MultivariatePolynomial,
    pub ordered_divisors: Vec<MultivariatePolynomial>,
    pub quotients: Vec<MultivariatePolynomial>,
    pub remainder: MultivariatePolynomial,
    pub monomial_order: DivisionMonomialOrder,
    pub recomposition: DivisionRecompositionClaim,
    pub reduced_remainder: DivisionReducedRemainderClaim,
    pub lines: Vec<DivisionExplanationLine>,
    /// Reported by the deterministic algorithm, not a per-step trace.
    pub reported_iterations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultivariateDerivationError {
    Division(MultivariateDivisionError),
    InvalidDivisionEvidence,
    InvalidExplanation,
    ResourceLimit,
}

impl fmt::Display for MultivariateDerivationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Division(error) => write!(f, "multivariate division: {error}"),
            Self::InvalidDivisionEvidence => {
                f.write_str("division evidence does not certify this ordered result")
            }
            Self::InvalidExplanation => {
                f.write_str("derivation does not match independently checked division evidence")
            }
            Self::ResourceLimit => f.write_str("division explanation resource limit exceeded"),
        }
    }
}

impl Error for MultivariateDerivationError {}

impl From<MultivariateDivisionError> for MultivariateDerivationError {
    fn from(value: MultivariateDivisionError) -> Self {
        Self::Division(value)
    }
}

/// Writes one bounded line without creating an unbounded formatted String.
struct BoundedLine<'a> {
    content: String,
    total_bytes: &'a mut usize,
}

impl fmt::Write for BoundedLine<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let line_bytes = self
            .content
            .len()
            .checked_add(text.len())
            .ok_or(fmt::Error)?;
        let total_bytes = self.total_bytes.checked_add(text.len()).ok_or(fmt::Error)?;
        if line_bytes > MAX_LINE_BYTES || total_bytes > MAX_RENDER_BYTES {
            return Err(fmt::Error);
        }
        self.content.push_str(text);
        *self.total_bytes = total_bytes;
        Ok(())
    }
}

fn push_line(
    lines: &mut Vec<DivisionExplanationLine>,
    bytes: &mut usize,
    kind: DivisionExplanationKind,
    explanation: &'static str,
    write_display: impl FnOnce(&mut BoundedLine<'_>) -> fmt::Result,
) -> Result<(), MultivariateDerivationError> {
    if lines.len() >= MAX_RENDER_LINES {
        return Err(MultivariateDerivationError::ResourceLimit);
    }
    let mut writer = BoundedLine {
        content: String::new(),
        total_bytes: bytes,
    };
    write_display(&mut writer).map_err(|_| MultivariateDerivationError::ResourceLimit)?;
    lines.push(DivisionExplanationLine {
        kind,
        explanation,
        display: writer.content,
    });
    Ok(())
}

/// The existing verifier proves recomposition and leading-term
/// nondivisibility. Re-running the bounded algorithm additionally ensures
/// quotient order and reported iteration count match its fixed priority.
fn validate(
    dividend: &MultivariatePolynomial,
    divisors: &[MultivariatePolynomial],
    evidence: &MultivariateDivision,
    limits: MultivariateLimits,
) -> Result<(), MultivariateDerivationError> {
    if dividend
        .variables()
        .iter()
        .any(|symbol| symbol.name().len() > MAX_VARIABLE_NAME_BYTES)
        || evidence.steps() > usize::try_from(limits.max_work_units).unwrap_or(usize::MAX)
    {
        return Err(MultivariateDerivationError::ResourceLimit);
    }
    let verified = verify_multivariate_division(dividend, divisors, evidence, limits)?;
    if !verified {
        return Err(MultivariateDerivationError::InvalidDivisionEvidence);
    }
    // The independent certificate alone does not establish priority:
    // multiple quotient vectors can satisfy f = sum(q_i*g_i) + r.
    let canonical = divide_multivariate(dividend, divisors, limits)?;
    if &canonical != evidence {
        return Err(MultivariateDerivationError::InvalidDivisionEvidence);
    }
    Ok(())
}

/// Builds a typed explanation only after verifying the entire supplied
/// division certificate and its deterministic ordered-algorithm result.
///
/// The displayed identity is f = sum_i q_i*g_i + r in the exact ring Q[vars].
/// No intermediate reductions or Groebner-basis claims are manufactured.
pub fn render_multivariate_division(
    dividend: &MultivariatePolynomial,
    divisors: &[MultivariatePolynomial],
    evidence: &MultivariateDivision,
    limits: MultivariateLimits,
) -> Result<MultivariateDivisionDerivation, MultivariateDerivationError> {
    validate(dividend, divisors, evidence, limits)?;

    let quotients = evidence.quotients().to_vec();
    let remainder = evidence.remainder().clone();
    let products: Vec<_> = quotients
        .iter()
        .cloned()
        .zip(divisors.iter().cloned())
        .enumerate()
        .map(|(divisor_index, (quotient, divisor))| DivisionProduct {
            divisor_index,
            quotient,
            divisor,
        })
        .collect();
    let leading_exponents = divisors
        .iter()
        .map(|g| {
            g.terms()
                .last_key_value()
                .expect("independent verifier rejects zero divisors")
                .0
                .clone()
        })
        .collect();
    let mut lines = Vec::with_capacity(5 + divisors.len() * 2);
    let mut bytes = 0;
    push_line(
        &mut lines,
        &mut bytes,
        DivisionExplanationKind::Dividend,
        "Exact dividend in the selected ambient polynomial ring.",
        |w| write!(w, "f = {dividend}"),
    )?;
    push_line(
        &mut lines,
        &mut bytes,
        DivisionExplanationKind::AmbientRingAndOrder,
        "Lexicographic monomial order in canonical variable order; divisors are tried by index.",
        |w| {
            w.write_str("Q[")?;
            for (i, symbol) in dividend.variables().iter().enumerate() {
                if i > 0 {
                    w.write_str(", ")?;
                }
                write!(w, "{:?}", symbol.name())?;
            }
            w.write_str("]; lexicographic, ordered divisors g[0], g[1], ...")
        },
    )?;
    for (i, divisor) in divisors.iter().enumerate() {
        push_line(
            &mut lines,
            &mut bytes,
            DivisionExplanationKind::Divisor { index: i },
            "Divisor at the stated priority position.",
            |w| write!(w, "g[{i}] = {divisor}"),
        )?;
    }
    for (i, quotient) in quotients.iter().enumerate() {
        push_line(
            &mut lines,
            &mut bytes,
            DivisionExplanationKind::Quotient { index: i },
            "Exact quotient paired with the divisor at this index.",
            |w| write!(w, "q[{i}] = {quotient}"),
        )?;
    }
    push_line(
        &mut lines,
        &mut bytes,
        DivisionExplanationKind::Remainder,
        "Exact remainder; zero is retained in the ambient ring.",
        |w| write!(w, "r = {remainder}"),
    )?;
    push_line(
        &mut lines,
        &mut bytes,
        DivisionExplanationKind::Recomposition,
        "Independently checked exact polynomial equality, not a uniqueness claim.",
        |w| {
            write!(w, "({dividend}) = ")?;
            for (i, product) in products.iter().enumerate() {
                if i != 0 {
                    w.write_str(" + ")?;
                }
                write!(w, "({}) * ({})", product.quotient, product.divisor)?;
            }
            write!(w, " + ({remainder})")
        },
    )?;
    push_line(
        &mut lines,
        &mut bytes,
        DivisionExplanationKind::ReducedRemainder,
        "No remainder monomial is divisible by a listed leading monomial; no unique normal form is asserted.",
        |w| {
            if remainder.is_zero() {
                w.write_str("r = 0; reduced-remainder condition holds vacuously; ")?;
            } else {
                w.write_str("r is reduced; ")?;
            }
            for (i, divisor) in divisors.iter().enumerate() {
                if i != 0 {
                    w.write_str("; ")?;
                }
                let leading = divisor
                    .terms()
                    .last_key_value()
                    .expect("verified nonzero divisor")
                    .0;
                write!(w, "LM(g[{i}]) has exponents {leading:?}")?;
            }
            w.write_str("; no monomial of r is divisible by these leading monomials")
        },
    )?;
    Ok(MultivariateDivisionDerivation {
        dividend: dividend.clone(),
        ordered_divisors: divisors.to_vec(),
        quotients,
        remainder: remainder.clone(),
        monomial_order: DivisionMonomialOrder::Lexicographic,
        recomposition: DivisionRecompositionClaim {
            relation: Relation::Equal,
            dividend: dividend.clone(),
            ordered_products: products,
            remainder: remainder.clone(),
        },
        reduced_remainder: DivisionReducedRemainderClaim {
            remainder,
            leading_exponents,
            monomial_order: DivisionMonomialOrder::Lexicographic,
        },
        lines,
        reported_iterations: evidence.steps(),
    })
}

/// Independently validates the mathematical certificate and reconstructs the
/// exact deterministic explanation. This detects forged structured claims and
/// modified prose; equality of display strings alone never proves division.
pub fn verify_multivariate_derivation(
    dividend: &MultivariatePolynomial,
    divisors: &[MultivariatePolynomial],
    evidence: &MultivariateDivision,
    claimed: &MultivariateDivisionDerivation,
    limits: MultivariateLimits,
) -> Result<(), MultivariateDerivationError> {
    let expected = render_multivariate_division(dividend, divisors, evidence, limits)?;
    if &expected == claimed {
        Ok(())
    } else {
        Err(MultivariateDerivationError::InvalidExplanation)
    }
}
