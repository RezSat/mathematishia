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

/// A typed total affine argument, with no inherited expression restrictions.
/// Adapters must independently establish that lowering preserves the source domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrigEquation {
    pub function: TrigFunction,
    pub slope: Rational,
    pub shift: PiMultiple,
    pub target: ExactQuadraticRoot,
    pub domain: ScalarDomain,
}

/// The parameter in a periodic family ranges over every integer, not real numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicParameter {
    IntegerK,
}

/// x = offset + period*k, with a positive period and k in Z.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicFamily {
    pub offset: PiMultiple,
    pub period: PiMultiple,
    pub parameter: PeriodicParameter,
}

/// Source-bound evidence for a complete real solution set. An absent principal
/// angle and empty families certify only the checked rational outside-range case.
/// The argument period is a period of the solution set, not necessarily of f.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodicTrigCertificate {
    pub equation: PeriodicTrigEquation,
    pub principal: Option<SpecialAngleCertificate>,
    pub argument_period: Option<PiMultiple>,
    pub families: Vec<PeriodicFamily>,
    pub excluded_poles: Option<PeriodicFamily>,
}

/// A polynomial identity in sin(t)^2, cos(t)^2, cos(2t), and 1, in that order.
/// The two multipliers witness an exact combination of the unit-circle and
/// cosine-addition identities. All terms are defined for every real t.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrigIdentityCertificate {
    pub coefficients: [Rational; 4],
    pub circle_multiplier: Rational,
    pub addition_multiplier: Rational,
    pub domain: ScalarDomain,
}

impl TrigFunction {
    /// Construct a typed equation f(a*x+b*pi)=c. Zero slopes are not admitted
    /// by the solver; no restriction-bearing expression is lowered here.
    pub fn periodic_equation(
        self,
        slope: Rational,
        shift: PiMultiple,
        target: ExactQuadraticRoot,
        domain: ScalarDomain,
    ) -> PeriodicTrigEquation {
        PeriodicTrigEquation {
            function: self,
            slope,
            shift,
            target,
            domain,
        }
    }

    /// Prove a bounded identity by exact linear combination of
    /// sin(t)^2+cos(t)^2-1=0 and sin(t)^2-cos(t)^2+cos(2t)=0.
    pub fn prove_squared_identity(
        coefficients: &[Rational; 4],
        domain: ScalarDomain,
        limits: TrigLimits,
    ) -> Result<TrigIdentityCertificate, TrigError> {
        let certificate = TrigIdentityCertificate {
            coefficients: coefficients.clone(),
            circle_multiplier: -coefficients[3].clone(),
            addition_multiplier: coefficients[2].clone(),
            domain,
        };
        if !certificate.verify(coefficients, domain, limits)? {
            return Err(TrigError::UnsupportedEquation);
        }
        Ok(certificate)
    }
}

impl TrigIdentityCertificate {
    pub fn verify(
        &self,
        expected: &[Rational; 4],
        domain: ScalarDomain,
        limits: TrigLimits,
    ) -> Result<bool, TrigError> {
        admit_angle(&PiMultiple::new(ratio(0, 1)), domain, limits)?;
        for value in expected.iter().chain(self.coefficients.iter()).chain([
            &self.circle_multiplier,
            &self.addition_multiplier,
        ]) {
            admit(value, limits.max_input_bits)?;
        }
        let circle = &self.circle_multiplier;
        let addition = &self.addition_multiplier;
        Ok(self.domain == domain
            && &self.coefficients == expected
            && *expected
                == [
                    circle + addition,
                    circle - addition,
                    addition.clone(),
                    -circle.clone(),
                ])
    }
}

fn admit_periodic_equation(
    equation: &PeriodicTrigEquation,
    limits: TrigLimits,
) -> Result<(), TrigError> {
    admit_angle(&equation.shift, equation.domain, limits)?;
    admit(&equation.slope, limits.max_input_bits)?;
    if equation.slope.numerator().is_zero() {
        return Err(TrigError::UnsupportedEquation);
    }
    match &equation.target {
        ExactQuadraticRoot::Rational(value) => admit(value, limits.max_input_bits),
        ExactQuadraticRoot::Algebraic {
            minimal_polynomial, ..
        } => {
            let coefficients = minimal_polynomial.coefficients();
            if coefficients.len() != 3 {
                return Err(TrigError::UnsupportedEquation);
            }
            for value in coefficients {
                admit(value, limits.max_input_bits)?;
            }
            Ok(())
        }
    }
}

fn outside_unit_range(equation: &PeriodicTrigEquation) -> bool {
    match &equation.target {
        ExactQuadraticRoot::Rational(value) if equation.function != TrigFunction::Tan => {
            value.numerator() > value.denominator()
                || value.numerator() < &(-value.denominator().clone())
        }
        _ => false,
    }
}

fn absolute_slope(slope: &Rational) -> Rational {
    if rational_sign(slope) < 0 {
        -slope.clone()
    } else {
        slope.clone()
    }
}

fn affine_family(
    equation: &PeriodicTrigEquation,
    base: &Rational,
    argument_period: &Rational,
) -> PeriodicFamily {
    let shifted = base - equation.shift.coefficient();
    PeriodicFamily {
        offset: PiMultiple::new(rational_quotient(&shifted, &equation.slope)),
        period: PiMultiple::new(rational_quotient(
            argument_period,
            &absolute_slope(&equation.slope),
        )),
        parameter: PeriodicParameter::IntegerK,
    }
}

fn principal_ticks(function: TrigFunction) -> &'static [i64] {
    match function {
        TrigFunction::Sin => &[-6, -4, -3, -2, 0, 2, 3, 4, 6],
        TrigFunction::Cos => &[0, 2, 3, 4, 6, 8, 9, 10, 12],
        TrigFunction::Tan => &[-4, -3, -2, 0, 2, 3, 4],
    }
}

impl PeriodicTrigEquation {
    /// Search only the supported native special angles in the injective
    /// principal interval. Extend by exact symmetries, never by sampling x.
    /// Unsupported right-hand sides (including non-table values within range)
    /// and constant slopes are refused rather than called empty.
    pub fn solve(&self, limits: TrigLimits) -> Result<PeriodicTrigCertificate, TrigError> {
        admit_periodic_equation(self, limits)?;
        let mut certificate = PeriodicTrigCertificate {
            equation: self.clone(),
            principal: None,
            argument_period: None,
            families: Vec::new(),
            excluded_poles: None,
        };
        if !outside_unit_range(self) {
            let mut principal = None;
            for tick in principal_ticks(self.function) {
                let angle = PiMultiple::new(ratio(*tick, 12));
                let value = evaluate_special_angle(
                    &angle,
                    self.function,
                    self.domain,
                    TrigLimits::default(),
                )?;
                if value.outcome == SpecialAngleOutcome::Exact(self.target.clone()) {
                    principal = Some(value);
                    break;
                }
            }
            let principal = principal.ok_or(TrigError::UnsupportedEquation)?;
            let base = principal.angle.coefficient();
            let zero = self.target == ExactQuadraticRoot::Rational(ratio(0, 1));
            let period = if self.function == TrigFunction::Tan || zero {
                ratio(1, 1)
            } else {
                ratio(2, 1)
            };
            certificate.families.push(affine_family(self, base, &period));
            if !zero {
                let reflected = match self.function {
                    TrigFunction::Sin if *base != ratio(1, 2) && *base != ratio(-1, 2) => {
                        Some(&ratio(1, 1) - base)
                    }
                    TrigFunction::Cos if *base != ratio(0, 1) && *base != ratio(1, 1) => {
                        Some(-base.clone())
                    }
                    _ => None,
                };
                if let Some(other) = reflected {
                    certificate
                        .families
                        .push(affine_family(self, &other, &period));
                }
            }
            if self.function == TrigFunction::Tan {
                certificate.excluded_poles = Some(affine_family(self, &ratio(1, 2), &ratio(1, 1)));
            }
            certificate.argument_period = Some(PiMultiple::new(period));
            certificate.principal = Some(principal);
        }
        if !self.verify(&certificate, limits)? {
            return Err(TrigError::InvalidEvidence);
        }
        Ok(certificate)
    }

    /// Check completeness independently of the angle search and affine solver.
    /// Sine is injective on [-pi/2,pi/2], cosine on [0,pi], and tangent on
    /// (-pi/2,pi/2). Reflection and integer translations exhaust the real
    /// preimages of these intervals. Invert the affine map by multiplication.
    pub fn verify(
        &self,
        certificate: &PeriodicTrigCertificate,
        limits: TrigLimits,
    ) -> Result<bool, TrigError> {
        admit_periodic_equation(self, limits)?;
        admit_periodic_equation(&certificate.equation, limits)?;
        if certificate.equation != *self || certificate.families.len() > 2 {
            return Ok(false);
        }
        if outside_unit_range(self) {
            return Ok(certificate.principal.is_none()
                && certificate.argument_period.is_none()
                && certificate.families.is_empty()
                && certificate.excluded_poles.is_none());
        }
        let (Some(principal), Some(period)) = (&certificate.principal, &certificate.argument_period)
        else {
            return Ok(false);
        };
        admit(principal.angle.coefficient(), MAX_EVIDENCE_BITS)?;
        admit(period.coefficient(), MAX_EVIDENCE_BITS)?;
        let p = principal.angle.coefficient();
        if !principal_ticks(self.function)
            .iter()
            .any(|tick| *p == ratio(*tick, 12))
            || !verify_special_angle(
                &principal.angle,
                self.function,
                self.domain,
                principal,
                TrigLimits::default(),
            )?
            || principal.outcome != SpecialAngleOutcome::Exact(self.target.clone())
        {
            return Ok(false);
        }

        // Zero sine/cosine roots have one residue modulo pi; all other
        // sine/cosine roots use a full turn, with the extrema unduplicated.
        let (expected_period, other) = match self.function {
            TrigFunction::Tan => (ratio(1, 1), None),
            TrigFunction::Sin if *p == ratio(0, 1) => (ratio(1, 1), None),
            TrigFunction::Cos if *p == ratio(1, 2) => (ratio(1, 1), None),
            TrigFunction::Sin if *p == ratio(1, 2) || *p == ratio(-1, 2) => {
                (ratio(2, 1), None)
            }
            TrigFunction::Cos if *p == ratio(0, 1) || *p == ratio(1, 1) => {
                (ratio(2, 1), None)
            }
            TrigFunction::Sin => (ratio(2, 1), Some(&ratio(1, 1) - p)),
            TrigFunction::Cos => (ratio(2, 1), Some(-p.clone())),
        };
        if *period.coefficient() != expected_period
            || certificate.families.len() != 1 + usize::from(other.is_some())
        {
            return Ok(false);
        }
        if !self.verify_family(&certificate.families[0], p, &expected_period, limits)? {
            return Ok(false);
        }
        if let Some(other) = other
            && !self.verify_family(&certificate.families[1], &other, &expected_period, limits)?
        {
            return Ok(false);
        }
        match (&certificate.excluded_poles, self.function) {
            (Some(poles), TrigFunction::Tan) => {
                self.verify_family(poles, &ratio(1, 2), &ratio(1, 1), limits)
            }
            (None, TrigFunction::Sin | TrigFunction::Cos) => Ok(true),
            _ => Ok(false),
        }
    }

    fn verify_family(
        &self,
        family: &PeriodicFamily,
        base: &Rational,
        period: &Rational,
        limits: TrigLimits,
    ) -> Result<bool, TrigError> {
        // Two input-sized rational factors plus a fixed special-angle constant
        // suffice for the affine inverse. Bound forged claims before arithmetic.
        let output_bits = 2 * limits.max_input_bits + 16;
        let offset = family.offset.coefficient();
        let step = family.period.coefficient();
        admit(offset, output_bits)?;
        admit(step, output_bits)?;
        Ok(family.parameter == PeriodicParameter::IntegerK
            && rational_sign(step) > 0
            && &(&self.slope * offset) + self.shift.coefficient() == *base
            && &absolute_slope(&self.slope) * step == *period)
    }
}

#[cfg(test)]
mod periodic_tests {
    use super::*;

    fn equation(
        function: TrigFunction,
        a: Rational,
        b: Rational,
        c: Rational,
    ) -> PeriodicTrigEquation {
        function.periodic_equation(
            a,
            PiMultiple::new(b),
            ExactQuadraticRoot::Rational(c),
            ScalarDomain::Real,
        )
    }

    fn assert_families(source: &PeriodicTrigEquation, expected: &[(Rational, Rational)]) {
        let limits = TrigLimits::default();
        let result = source.solve(limits).unwrap();
        assert!(source.verify(&result, limits).unwrap());
        assert_eq!(result, source.solve(limits).unwrap());
        assert_eq!(result.families.len(), expected.len());
        for (family, (offset, period)) in result.families.iter().zip(expected) {
            assert_eq!(family.offset.coefficient(), offset);
            assert_eq!(family.period.coefficient(), period);
            assert_eq!(family.parameter, PeriodicParameter::IntegerK);
        }
    }

    #[test]
    fn complete_periodic_families_and_extrema() {
        use TrigFunction::{Cos, Sin, Tan};

        assert_families(
            &equation(Sin, ratio(1, 1), ratio(0, 1), ratio(0, 1)),
            &[(ratio(0, 1), ratio(1, 1))],
        );
        assert_families(
            &equation(Cos, ratio(1, 1), ratio(0, 1), ratio(1, 2)),
            &[(ratio(1, 3), ratio(2, 1)), (ratio(-1, 3), ratio(2, 1))],
        );
        assert_families(
            &equation(Sin, ratio(2, 1), ratio(0, 1), ratio(1, 2)),
            &[(ratio(1, 12), ratio(1, 1)), (ratio(5, 12), ratio(1, 1))],
        );
        assert_families(
            &equation(Tan, ratio(1, 1), ratio(0, 1), ratio(1, 1)),
            &[(ratio(1, 4), ratio(1, 1))],
        );
        for (function, target, base) in [
            (Cos, 1, ratio(0, 1)),
            (Cos, -1, ratio(1, 1)),
            (Sin, 1, ratio(1, 2)),
            (Sin, -1, ratio(-1, 2)),
        ] {
            assert_families(
                &equation(function, ratio(1, 1), ratio(0, 1), ratio(target, 1)),
                &[(base, ratio(2, 1))],
            );
        }
        assert_families(
            &equation(Cos, ratio(1, 1), ratio(0, 1), ratio(0, 1)),
            &[(ratio(1, 2), ratio(1, 1))],
        );
    }

    #[test]
    fn signed_rational_affine_inverse_and_poles() {
        let source = equation(TrigFunction::Tan, ratio(-2, 3), ratio(1, 6), ratio(-1, 1));
        assert_families(&source, &[(ratio(5, 8), ratio(3, 2))]);
        let result = source.solve(TrigLimits::default()).unwrap();
        let poles = result.excluded_poles.unwrap();
        assert_eq!(poles.offset.coefficient(), &ratio(-1, 2));
        assert_eq!(poles.period.coefficient(), &ratio(3, 2));
    }

    #[test]
    fn every_supported_signed_native_value_has_a_complete_family() {
        for function in [TrigFunction::Sin, TrigFunction::Cos, TrigFunction::Tan] {
            for tick in [0, 2, 3, 4, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20, 21, 22] {
                let angle = PiMultiple::new(ratio(tick, 12));
                let outcome = evaluate_special_angle(
                    &angle,
                    function,
                    ScalarDomain::Real,
                    TrigLimits::default(),
                )
                .unwrap()
                .outcome;
                let SpecialAngleOutcome::Exact(target) = outcome else {
                    assert_eq!(function, TrigFunction::Tan);
                    continue;
                };
                let source = function.periodic_equation(
                    ratio(3, 2),
                    PiMultiple::new(ratio(-5, 7)),
                    target,
                    ScalarDomain::Real,
                );
                let result = source.solve(TrigLimits::default()).unwrap();
                assert!(source.verify(&result, TrigLimits::default()).unwrap());
            }
        }
    }

    #[test]
    fn only_proved_outside_range_is_empty() {
        let limits = TrigLimits::default();
        for function in [TrigFunction::Sin, TrigFunction::Cos] {
            for target in [ratio(2, 1), ratio(-3, 2)] {
                let source = equation(function, ratio(1, 1), ratio(0, 1), target);
                assert_families(&source, &[]);
            }
            let source = equation(function, ratio(1, 1), ratio(0, 1), ratio(1, 3));
            assert_eq!(source.solve(limits), Err(TrigError::UnsupportedEquation));
        }
        let source = equation(TrigFunction::Tan, ratio(1, 1), ratio(0, 1), ratio(2, 1));
        assert_eq!(source.solve(limits), Err(TrigError::UnsupportedEquation));
        let source = equation(TrigFunction::Sin, ratio(0, 1), ratio(0, 1), ratio(0, 1));
        assert_eq!(source.solve(limits), Err(TrigError::UnsupportedEquation));
    }

    #[test]
    fn forged_coverage_period_pole_and_source_are_rejected() {
        let limits = TrigLimits::default();
        let source = equation(TrigFunction::Sin, ratio(2, 1), ratio(0, 1), ratio(1, 2));
        let valid = source.solve(limits).unwrap();
        let mut forged = valid.clone();
        forged.families.pop();
        assert!(!source.verify(&forged, limits).unwrap());
        forged = valid.clone();
        forged.families[0].period = PiMultiple::new(ratio(2, 1));
        assert!(!source.verify(&forged, limits).unwrap());
        forged = valid.clone();
        forged.families[1] = forged.families[0].clone();
        assert!(!source.verify(&forged, limits).unwrap());
        forged = valid.clone();
        forged.argument_period = Some(PiMultiple::new(ratio(1, 1)));
        assert!(!source.verify(&forged, limits).unwrap());
        forged = valid.clone();
        forged.principal = None;
        forged.families.clear();
        forged.argument_period = None;
        assert!(!source.verify(&forged, limits).unwrap());
        forged = valid.clone();
        forged.equation.shift = PiMultiple::new(ratio(1, 3));
        assert!(!source.verify(&forged, limits).unwrap());
        let tangent = equation(TrigFunction::Tan, ratio(1, 1), ratio(0, 1), ratio(1, 1));
        let mut forged = tangent.solve(limits).unwrap();
        forged.excluded_poles = None;
        assert!(!tangent.verify(&forged, limits).unwrap());
        forged = tangent.solve(limits).unwrap();
        forged.excluded_poles = Some(forged.families[0].clone());
        assert!(!tangent.verify(&forged, limits).unwrap());
    }

    #[test]
    fn periodic_bounds_and_domains_are_enforced() {
        let limits = TrigLimits { max_input_bits: 8 };
        let mut source = equation(TrigFunction::Sin, ratio(257, 1), ratio(0, 1), ratio(0, 1));
        assert_eq!(source.solve(limits), Err(TrigError::ScalarLimit));
        source.slope = ratio(1, 1);
        source.domain = ScalarDomain::Complex;
        assert_eq!(
            source.solve(limits),
            Err(TrigError::UnsupportedDomain(ScalarDomain::Complex))
        );
        source.domain = ScalarDomain::Real;
        let mut forged = source.solve(limits).unwrap();
        let large = "9".repeat(100).parse::<Integer>().unwrap();
        forged.families[0].offset = PiMultiple::new(Rational::from(large));
        assert_eq!(source.verify(&forged, limits), Err(TrigError::ScalarLimit));
        assert_eq!(
            source.solve(TrigLimits { max_input_bits: 0 }),
            Err(TrigError::InvalidLimits)
        );
    }

    #[test]
    fn squared_identities_have_exact_checked_combination_evidence() {
        let limits = TrigLimits::default();
        for terms in [[1, 1, 0, -1], [1, -1, 1, 0], [2, 0, 1, -1], [0, -2, 1, 1]] {
            let coefficients = terms.map(|value| ratio(value, 1));
            let proof = TrigFunction::prove_squared_identity(
                &coefficients,
                ScalarDomain::Real,
                limits,
            )
            .unwrap();
            assert!(
                proof
                    .verify(&coefficients, ScalarDomain::Real, limits)
                    .unwrap()
            );
            let mut forged = proof.clone();
            forged.circle_multiplier = &forged.circle_multiplier + &ratio(1, 1);
            assert!(
                !forged
                    .verify(&coefficients, ScalarDomain::Real, limits)
                    .unwrap()
            );
            assert!(
                !proof
                    .verify(
                        &[ratio(0, 1), ratio(0, 1), ratio(0, 1), ratio(0, 1)],
                        ScalarDomain::Real,
                        limits,
                    )
                    .unwrap()
            );
        }
        assert_eq!(
            TrigFunction::prove_squared_identity(
                &[ratio(1, 1), ratio(1, 1), ratio(0, 1), ratio(0, 1)],
                ScalarDomain::Real,
                limits,
            ),
            Err(TrigError::UnsupportedEquation)
        );
    }
}
