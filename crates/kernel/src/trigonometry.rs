//! Exact special-angle values for explicitly typed real multiples of pi.
//!
//! This is not a parser, a numerical evaluator, or a general trigonometric solver.

use std::cmp::Ordering;
use std::fmt;

use mathema::{Integer, Rational, ScalarDomain};

use crate::poly_division::rational_quotient;
use crate::{ExactQuadraticRoot, QuadraticBranch, RationalPolynomial};

const HARD_MAX_INPUT_BITS: u64 = 4_096;
const MAX_EVIDENCE_BITS: u64 = 16;

/// The mathematical angle q*pi radians, not an expression containing a symbol
/// named "pi". The canonical rational coefficient retains the original angle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiMultiple(Rational);

impl PiMultiple {
    pub fn new(coefficient: Rational) -> Self {
        Self(coefficient)
    }

    pub fn coefficient(&self) -> &Rational {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrigFunction {
    Sin,
    Cos,
    Tan,
}

/// Bounds input integers before exact modular reduction. The algorithm uses
/// a fixed number of arithmetic operations and no input-dependent search.
/// Intermediate integers can be at most five bits wider than the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrigLimits {
    pub max_input_bits: u64,
}

impl Default for TrigLimits {
    fn default() -> Self {
        Self {
            max_input_bits: HARD_MAX_INPUT_BITS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrigError {
    InvalidLimits,
    ScalarLimit,
    UnsupportedDomain(ScalarDomain),
    UnsupportedAngle,
    UnsupportedEquation,
    InvalidEvidence,
}

impl fmt::Display for TrigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => f.write_str("invalid trigonometric resource limits"),
            Self::ScalarLimit => f.write_str("trigonometric scalar bit limit exceeded"),
            Self::UnsupportedDomain(domain) => write!(f, "unsupported angle domain: {domain:?}"),
            Self::UnsupportedAngle => f.write_str("angle outside the exact unit-circle table"),
            Self::UnsupportedEquation => f.write_str("unsupported trigonometric equation"),
            Self::InvalidEvidence => f.write_str("trigonometric evidence failed verification"),
        }
    }
}

impl std::error::Error for TrigError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecialAngleOutcome {
    Exact(ExactQuadraticRoot),
    UndefinedTangentPole,
}

/// Public evidence is untrusted until checked against the caller's source.
/// The tick is the angle reduced modulo 2*pi in units of pi/12.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpecialAngleCertificate {
    pub angle: PiMultiple,
    pub function: TrigFunction,
    pub domain: ScalarDomain,
    pub tick: u8,
    pub outcome: SpecialAngleOutcome,
}

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator))
        .expect("nonzero constant denominator")
}

fn admit(value: &Rational, max_bits: u64) -> Result<(), TrigError> {
    if value.numerator().bit_len() > max_bits || value.denominator().bit_len() > max_bits {
        return Err(TrigError::ScalarLimit);
    }
    Ok(())
}

fn admit_angle(
    angle: &PiMultiple,
    domain: ScalarDomain,
    limits: TrigLimits,
) -> Result<(), TrigError> {
    if limits.max_input_bits == 0 || limits.max_input_bits > HARD_MAX_INPUT_BITS {
        return Err(TrigError::InvalidLimits);
    }
    if domain != ScalarDomain::Real {
        return Err(TrigError::UnsupportedDomain(domain));
    }
    admit(angle.coefficient(), limits.max_input_bits)
}

fn reduced_tick(angle: &PiMultiple) -> Result<u8, TrigError> {
    let coefficient = angle.coefficient();
    let denominator = coefficient.denominator();
    let modulus = denominator + denominator;
    let residue = coefficient
        .numerator()
        .modulo_positive(&modulus)
        .ok_or(TrigError::InvalidEvidence)?;
    let scaled = &residue * &Integer::from(12);
    let tick = scaled
        .divide_exact(denominator)
        .and_then(|value| value.try_i64())
        .ok_or(TrigError::UnsupportedAngle)?;
    u8::try_from(tick).map_err(|_| TrigError::InvalidEvidence)
}

// Fold into the first quadrant and store exact squared magnitudes.
fn sine_table(tick: u8) -> Result<(i8, Rational), TrigError> {
    let half_turn = tick % 12;
    let acute = half_turn.min(12 - half_turn);
    let square = match acute {
        0 => ratio(0, 1),
        2 => ratio(1, 4),
        3 => ratio(1, 2),
        4 => ratio(3, 4),
        6 => ratio(1, 1),
        _ => return Err(TrigError::UnsupportedAngle),
    };
    let sign = if acute == 0 {
        0
    } else if tick < 12 {
        1
    } else {
        -1
    };
    Ok((sign, square))
}

fn exact_root(sign: i8, square: Rational) -> ExactQuadraticRoot {
    if let Some(root) = square.exact_sqrt() {
        return ExactQuadraticRoot::Rational(if sign < 0 { -root } else { root });
    }
    ExactQuadraticRoot::Algebraic {
        minimal_polynomial: RationalPolynomial::new(vec![-square, ratio(0, 1), ratio(1, 1)]),
        branch: if sign < 0 {
            QuadraticBranch::LowerReal
        } else {
            QuadraticBranch::UpperReal
        },
    }
}

fn table_value(function: TrigFunction, tick: u8) -> Result<SpecialAngleOutcome, TrigError> {
    let (sine_sign, sine_square) = sine_table(tick)?;
    let (cosine_sign, cosine_square) = sine_table((tick + 6) % 24)?;
    let value = match function {
        TrigFunction::Sin => exact_root(sine_sign, sine_square),
        TrigFunction::Cos => exact_root(cosine_sign, cosine_square),
        TrigFunction::Tan if cosine_sign == 0 => {
            return Ok(SpecialAngleOutcome::UndefinedTangentPole);
        }
        TrigFunction::Tan => exact_root(
            sine_sign * cosine_sign,
            rational_quotient(&sine_square, &cosine_square),
        ),
    };
    Ok(SpecialAngleOutcome::Exact(value))
}

/// Supports 0, pi/6, pi/4, pi/3, pi/2 and their quadrant reflections
/// and full-turn translations. A tangent pole is not a real value.
pub fn evaluate_special_angle(
    angle: &PiMultiple,
    function: TrigFunction,
    domain: ScalarDomain,
    limits: TrigLimits,
) -> Result<SpecialAngleCertificate, TrigError> {
    admit_angle(angle, domain, limits)?;
    let tick = reduced_tick(angle)?;
    let certificate = SpecialAngleCertificate {
        angle: angle.clone(),
        function,
        domain,
        tick,
        outcome: table_value(function, tick)?,
    };
    if !verify_special_angle(angle, function, domain, &certificate, limits)? {
        return Err(TrigError::InvalidEvidence);
    }
    Ok(certificate)
}

fn rational_sign(value: &Rational) -> i8 {
    match value.numerator().cmp(&Integer::zero()) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

fn verify_root(value: &ExactQuadraticRoot, sign: i8, square: &Rational) -> Result<bool, TrigError> {
    match value {
        ExactQuadraticRoot::Rational(value) => {
            admit(value, MAX_EVIDENCE_BITS)?;
            Ok(rational_sign(value) == sign && &(value * value) == square)
        }
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch,
        } => {
            let coefficients = minimal_polynomial.coefficients();
            if coefficients.len() != 3 {
                return Ok(false);
            }
            for coefficient in coefficients {
                admit(coefficient, MAX_EVIDENCE_BITS)?;
            }
            let valid_branch = matches!(
                (sign, *branch),
                (-1, QuadraticBranch::LowerReal) | (1, QuadraticBranch::UpperReal)
            );
            Ok(valid_branch
                && square.exact_sqrt().is_none()
                && coefficients == [-square, ratio(0, 1), ratio(1, 1)])
        }
    }
}

/// Independently checks the signed root using the double-angle identities
/// sin²(t)=(1-cos(2t))/2 and cos²(t)=(1+cos(2t))/2, exact quadrant signs,
/// and tan²(t)=sin²(t)/cos²(t). It never calls the value table.
pub fn verify_special_angle(
    expected_angle: &PiMultiple,
    expected_function: TrigFunction,
    expected_domain: ScalarDomain,
    certificate: &SpecialAngleCertificate,
    limits: TrigLimits,
) -> Result<bool, TrigError> {
    admit_angle(expected_angle, expected_domain, limits)?;
    admit(certificate.angle.coefficient(), limits.max_input_bits)?;
    if certificate.angle != *expected_angle
        || certificate.function != expected_function
        || certificate.domain != expected_domain
    {
        return Ok(false);
    }
    let tick = reduced_tick(expected_angle)?;
    if certificate.tick != tick {
        return Ok(false);
    }
    let double_cosine = match tick % 12 {
        0 => ratio(1, 1),
        2 | 10 => ratio(1, 2),
        3 | 9 => ratio(0, 1),
        4 | 8 => ratio(-1, 2),
        6 => ratio(-1, 1),
        _ => return Err(TrigError::UnsupportedAngle),
    };
    let one = ratio(1, 1);
    let half = ratio(1, 2);
    let sine_square = &(&one - &double_cosine) * &half;
    let cosine_square = &(&one + &double_cosine) * &half;
    let sine_sign = match tick {
        0 | 12 => 0,
        1..=11 => 1,
        _ => -1,
    };
    let cosine_sign = match tick {
        6 | 18 => 0,
        7..=17 => -1,
        _ => 1,
    };

    let (sign, square) = match expected_function {
        TrigFunction::Sin => (sine_sign, sine_square),
        TrigFunction::Cos => (cosine_sign, cosine_square),
        TrigFunction::Tan if cosine_square.numerator().is_zero() => {
            return Ok(certificate.outcome == SpecialAngleOutcome::UndefinedTangentPole);
        }
        TrigFunction::Tan => (
            sine_sign * cosine_sign,
            rational_quotient(&sine_square, &cosine_square),
        ),
    };
    match &certificate.outcome {
        SpecialAngleOutcome::Exact(value) => verify_root(value, sign, &square),
        SpecialAngleOutcome::UndefinedTangentPole => Ok(false),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrigEquationUnsupported {
    IntegerParameterCompletenessUnavailable,
}

/// The equation f(x)=0 over unrestricted real x is not solved completely.
/// These are verified examples of roots, not a finite solution set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialTrigZeros {
    function: TrigFunction,
    known_zeros: [SpecialAngleCertificate; 2],
}

impl PartialTrigZeros {
    pub fn function(&self) -> TrigFunction {
        self.function
    }

    pub const fn unsupported_reason(&self) -> TrigEquationUnsupported {
        TrigEquationUnsupported::IntegerParameterCompletenessUnavailable
    }

    pub fn known_zeros(&self) -> &[SpecialAngleCertificate; 2] {
        &self.known_zeros
    }
}

/// Returns two checked zeros without asserting a parameterized solution set.
/// Only sine and cosine are supported; integer-parameter completeness is
/// deliberately deferred.
pub fn partial_trig_zeros(
    function: TrigFunction,
    domain: ScalarDomain,
    limits: TrigLimits,
) -> Result<PartialTrigZeros, TrigError> {
    let angles = match function {
        TrigFunction::Sin => [ratio(0, 1), ratio(1, 1)],
        TrigFunction::Cos => [ratio(1, 2), ratio(3, 2)],
        TrigFunction::Tan => return Err(TrigError::UnsupportedEquation),
    };
    let [first, second] = angles.map(PiMultiple::new);
    let known_zeros = [
        evaluate_special_angle(&first, function, domain, limits)?,
        evaluate_special_angle(&second, function, domain, limits)?,
    ];
    let zero = SpecialAngleOutcome::Exact(ExactQuadraticRoot::Rational(ratio(0, 1)));
    if known_zeros
        .iter()
        .any(|certificate| certificate.outcome != zero)
    {
        return Err(TrigError::InvalidEvidence);
    }
    Ok(PartialTrigZeros {
        function,
        known_zeros,
    })
}
