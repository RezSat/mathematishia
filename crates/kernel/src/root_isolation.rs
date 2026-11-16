//! Exact rational bisection of a supplied interval.
//!
//! A strict endpoint sign change certifies at least one real zero by continuity,
//! not uniqueness or exhaustion of all real roots. Equal nonzero endpoint signs
//! are inconclusive, including for even-multiplicity roots.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::budget::WorkBudget;
use crate::{EvaluationContext, EvaluationError, RationalPolynomial};

/// The certified observation from one bounded interval search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootIntervalOutcome {
    /// A rational zero checked by exact substitution.
    ExactRoot(Rational),
    /// Open interval containing at least one real zero; endpoint values
    /// are both nonzero and have opposite signs.
    SignChange { lower: Rational, upper: Rational },
    /// Neither endpoint is zero and their signs agree. No claim is made
    /// about existence or absence of roots inside.
    Unresolved { lower: Rational, upper: Rational },
}

/// A deterministic bisection result. A false precision flag on a sign-change
/// bracket means the caller's bisection count was exhausted before its target
/// width was attained. Exact roots always meet any positive width target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootRefinement {
    outcome: RootIntervalOutcome,
    bisections: usize,
    precision_met: bool,
}

impl RootRefinement {
    pub fn outcome(&self) -> &RootIntervalOutcome {
        &self.outcome
    }

    pub const fn bisections(&self) -> usize {
        self.bisections
    }

    pub const fn precision_met(&self) -> bool {
        self.precision_met
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootRefinementError {
    ZeroPolynomial,
    InvalidInterval,
    NonpositiveTargetWidth,
    Evaluation(EvaluationError),
}

impl fmt::Display for RootRefinementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for RootRefinementError {}

impl From<EvaluationError> for RootRefinementError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootRefinementVerificationError {
    InvalidInput,
    InvalidEvidence,
    Evaluation(EvaluationError),
}

impl fmt::Display for RootRefinementVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for RootRefinementVerificationError {}

impl From<EvaluationError> for RootRefinementVerificationError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

fn sign(value: &Rational) -> Ordering {
    value.numerator().cmp(&Integer::zero())
}

fn half() -> Rational {
    Rational::new(Integer::one(), Integer::from(2)).expect("2 is nonzero")
}

fn checked_input(
    ctx: &EvaluationContext,
    polynomial: &RationalPolynomial,
    lower: &Rational,
    upper: &Rational,
    target_width: &Rational,
) -> Result<(), RootRefinementError> {
    ctx.check_polynomial(polynomial)?;
    ctx.check_rational(lower)?;
    ctx.check_rational(upper)?;
    ctx.check_rational(target_width)?;
    if polynomial.is_zero() {
        return Err(RootRefinementError::ZeroPolynomial);
    }
    if lower >= upper {
        return Err(RootRefinementError::InvalidInterval);
    }
    if sign(target_width) != Ordering::Greater {
        return Err(RootRefinementError::NonpositiveTargetWidth);
    }
    Ok(())
}

impl EvaluationContext {
    /// Refines a *supplied* exact rational interval by at most max_bisections.
    ///
    /// The input must be a nonzero polynomial, lower < upper, and target_width
    /// > 0. Exact endpoint roots take precedence. With matching nonzero endpoint
    /// signs the original interval is returned unresolved without sampling.
    /// With a strict sign change, each exact midpoint either supplies a rational
    /// zero or replaces the endpoint having the same sign. The final bracket
    /// certifies existence, never uniqueness. It may still contain other roots.
    ///
    /// A zero bisection budget still classifies the input endpoints. Numeric
    /// resource limits and per-call work units apply to exact evaluations,
    /// midpoint formation and interval widths. An arithmetic limit returns an
    /// explicit error, not a partly verified certificate.
    pub fn refine_real_root_bracket(
        &self,
        polynomial: &RationalPolynomial,
        lower: &Rational,
        upper: &Rational,
        target_width: &Rational,
        max_bisections: usize,
    ) -> Result<RootRefinement, RootRefinementError> {
        checked_input(self, polynomial, lower, upper, target_width)?;
        let mut work = WorkBudget::new(self.limits());
        let lower_value = work.evaluate(polynomial, lower)?;
        if sign(&lower_value) == Ordering::Equal {
            return Ok(RootRefinement {
                outcome: RootIntervalOutcome::ExactRoot(lower.clone()),
                bisections: 0,
                precision_met: true,
            });
        }
        let upper_value = work.evaluate(polynomial, upper)?;
        if sign(&upper_value) == Ordering::Equal {
            return Ok(RootRefinement {
                outcome: RootIntervalOutcome::ExactRoot(upper.clone()),
                bisections: 0,
                precision_met: true,
            });
        }
        let mut lower_sign = sign(&lower_value);
        if lower_sign == sign(&upper_value) {
            return Ok(RootRefinement {
                outcome: RootIntervalOutcome::Unresolved {
                    lower: lower.clone(),
                    upper: upper.clone(),
                },
                bisections: 0,
                precision_met: false,
            });
        }

        let mut left = lower.clone();
        let mut right = upper.clone();
        let mut bisections = 0;
        loop {
            let width = work.subtract_scalar(&right, &left)?;
            if width <= *target_width || bisections == max_bisections {
                return Ok(RootRefinement {
                    outcome: RootIntervalOutcome::SignChange {
                        lower: left,
                        upper: right,
                    },
                    bisections,
                    precision_met: width <= *target_width,
                });
            }
            let sum = work.add_scalar(&left, &right)?;
            let middle = work.multiply_scalar(&sum, &half())?;
            let middle_value = work.evaluate(polynomial, &middle)?;
            bisections += 1;
            let middle_sign = sign(&middle_value);
            if middle_sign == Ordering::Equal {
                return Ok(RootRefinement {
                    outcome: RootIntervalOutcome::ExactRoot(middle),
                    bisections,
                    precision_met: true,
                });
            }
            if middle_sign == lower_sign {
                left = middle;
                lower_sign = middle_sign;
            } else {
                right = middle;
            }
        }
    }
}

/// Independently checks a claimed result by replaying exact rational midpoint
/// decisions without invoking the generator. It also re-evaluates the original
/// polynomial at the claimed final endpoints (or exact root). A supplied
/// bisection budget, width claim, interval, or root cannot be substituted.
/// As with generation, a resource-limit error is not a verification success.
pub fn verify_root_refinement(
    ctx: &EvaluationContext,
    polynomial: &RationalPolynomial,
    lower: &Rational,
    upper: &Rational,
    target_width: &Rational,
    max_bisections: usize,
    claimed: &RootRefinement,
) -> Result<(), RootRefinementVerificationError> {
    checked_input(ctx, polynomial, lower, upper, target_width).map_err(|error| match error {
        RootRefinementError::Evaluation(error) => {
            RootRefinementVerificationError::Evaluation(error)
        }
        _ => RootRefinementVerificationError::InvalidInput,
    })?;
    if claimed.bisections > max_bisections {
        return Err(RootRefinementVerificationError::InvalidEvidence);
    }
    let mut work = WorkBudget::new(ctx.limits());
    let initial_lower_sign = sign(&work.evaluate(polynomial, lower)?);
    let initial_upper_sign = if initial_lower_sign == Ordering::Equal {
        Ordering::Equal
    } else {
        sign(&work.evaluate(polynomial, upper)?)
    };

    let expected = if initial_lower_sign == Ordering::Equal {
        RootRefinement {
            outcome: RootIntervalOutcome::ExactRoot(lower.clone()),
            bisections: 0,
            precision_met: true,
        }
    } else if initial_upper_sign == Ordering::Equal {
        RootRefinement {
            outcome: RootIntervalOutcome::ExactRoot(upper.clone()),
            bisections: 0,
            precision_met: true,
        }
    } else if initial_lower_sign == initial_upper_sign {
        RootRefinement {
            outcome: RootIntervalOutcome::Unresolved {
                lower: lower.clone(),
                upper: upper.clone(),
            },
            bisections: 0,
            precision_met: false,
        }
    } else {
        let mut left = lower.clone();
        let mut right = upper.clone();
        let mut left_sign = initial_lower_sign;
        let mut steps = 0;
        loop {
            let width = work.subtract_scalar(&right, &left)?;
            if width <= *target_width || steps == max_bisections {
                break RootRefinement {
                    outcome: RootIntervalOutcome::SignChange {
                        lower: left,
                        upper: right,
                    },
                    bisections: steps,
                    precision_met: width <= *target_width,
                };
            }
            let sum = work.add_scalar(&left, &right)?;
            let middle = work.multiply_scalar(&sum, &half())?;
            let mid_sign = sign(&work.evaluate(polynomial, &middle)?);
            steps += 1;
            if mid_sign == Ordering::Equal {
                break RootRefinement {
                    outcome: RootIntervalOutcome::ExactRoot(middle),
                    bisections: steps,
                    precision_met: true,
                };
            }
            if mid_sign == left_sign {
                left = middle;
                left_sign = mid_sign;
            } else {
                right = middle;
            }
        }
    };
    if *claimed != expected {
        return Err(RootRefinementVerificationError::InvalidEvidence);
    }
    match &claimed.outcome {
        RootIntervalOutcome::ExactRoot(root) => {
            ctx.check_rational(root)?;
            if sign(&work.evaluate(polynomial, root)?) != Ordering::Equal {
                return Err(RootRefinementVerificationError::InvalidEvidence);
            }
        }
        RootIntervalOutcome::SignChange { lower, upper } => {
            ctx.check_rational(lower)?;
            ctx.check_rational(upper)?;
            if lower >= upper || sign(&work.evaluate(polynomial, lower)?) == Ordering::Equal {
                return Err(RootRefinementVerificationError::InvalidEvidence);
            }
            let left_sign = sign(&work.evaluate(polynomial, lower)?);
            let right_sign = sign(&work.evaluate(polynomial, upper)?);
            if right_sign == Ordering::Equal || left_sign == right_sign {
                return Err(RootRefinementVerificationError::InvalidEvidence);
            }
        }
        RootIntervalOutcome::Unresolved { lower, upper } => {
            ctx.check_rational(lower)?;
            ctx.check_rational(upper)?;
            if lower >= upper {
                return Err(RootRefinementVerificationError::InvalidEvidence);
            }
            let left_sign = sign(&work.evaluate(polynomial, lower)?);
            let right_sign = sign(&work.evaluate(polynomial, upper)?);
            if left_sign == Ordering::Equal
                || right_sign == Ordering::Equal
                || left_sign != right_sign
            {
                return Err(RootRefinementVerificationError::InvalidEvidence);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    fn rational(value: i64) -> Rational {
        Rational::from(Integer::from(value))
    }

    #[test]
    fn forged_precision_steps_endpoints_and_exact_root_are_rejected() {
        let ctx = EvaluationContext::default();
        let p = RationalPolynomial::new(vec![rational(-2), rational(0), rational(1)]);
        let a = rational(1);
        let b = rational(2);
        let width = Rational::new(Integer::one(), Integer::from(16)).unwrap();
        let original = ctx.refine_real_root_bracket(&p, &a, &b, &width, 4).unwrap();
        assert!(verify_root_refinement(&ctx, &p, &a, &b, &width, 4, &original).is_ok());

        let mut forged = original.clone();
        forged.precision_met = !forged.precision_met;
        assert_eq!(
            verify_root_refinement(&ctx, &p, &a, &b, &width, 4, &forged),
            Err(RootRefinementVerificationError::InvalidEvidence)
        );
        let mut forged = original.clone();
        forged.bisections += 1;
        assert_eq!(
            verify_root_refinement(&ctx, &p, &a, &b, &width, 4, &forged),
            Err(RootRefinementVerificationError::InvalidEvidence)
        );
        let mut forged = original.clone();
        forged.outcome = RootIntervalOutcome::ExactRoot(rational(1));
        assert_eq!(
            verify_root_refinement(&ctx, &p, &a, &b, &width, 4, &forged),
            Err(RootRefinementVerificationError::InvalidEvidence)
        );
        let mut forged = original.clone();
        forged.outcome = RootIntervalOutcome::SignChange {
            lower: rational(0),
            upper: rational(3),
        };
        assert_eq!(
            verify_root_refinement(&ctx, &p, &a, &b, &width, 4, &forged),
            Err(RootRefinementVerificationError::InvalidEvidence)
        );
    }
}
