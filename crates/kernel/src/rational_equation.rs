//! Exact rational-linear equations over Q with domain-aware solution sets.
//!
//! A canceled denominator never creates a solution at its original pole.
//! A zero residual describes all admissible rationals, NOT necessarily all Q.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::poly_division::rational_quotient;
use crate::{RationalFunction, RationalPolynomial};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalEquationError {
    NonlinearNumerator,
}

impl fmt::Display for RationalEquationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonlinearNumerator => {
                f.write_str("rational equation has numerator degree greater than one")
            }
        }
    }
}

impl Error for RationalEquationError {}

/// AllAdmissibleRationals means every rational satisfying domain conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RationalEquationSolution {
    Unique(Rational),
    NoSolution,
    AllAdmissibleRationals,
}

/// A result with its original partial functions and exact residual LHS-RHS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalEquationResult {
    left: RationalFunction,
    right: RationalFunction,
    residual: RationalFunction,
    solution: RationalEquationSolution,
}

impl RationalEquationResult {
    pub fn left(&self) -> &RationalFunction {
        &self.left
    }

    pub fn right(&self) -> &RationalFunction {
        &self.right
    }

    pub fn residual(&self) -> &RationalFunction {
        &self.residual
    }

    pub fn solution(&self) -> &RationalEquationSolution {
        &self.solution
    }

    /// The exact conditions defining the shared domain of the equation.
    pub fn domain_conditions(&self) -> &[RationalPolynomial] {
        self.residual.excluded_zero_sets()
    }

    pub(crate) fn from_parts(
        left: RationalFunction,
        right: RationalFunction,
        residual: RationalFunction,
        solution: RationalEquationSolution,
    ) -> Self {
        Self {
            left,
            right,
            residual,
            solution,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalEquationVerificationError {
    IncorrectResidual,
    IncorrectSolution,
    UnsupportedDegree,
}

impl fmt::Display for RationalEquationVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncorrectResidual => f.write_str("rational equation residual or domain is wrong"),
            Self::IncorrectSolution => f.write_str("rational equation solution is incorrect"),
            Self::UnsupportedDegree => {
                f.write_str("rational equation exceeds linear numerator scope")
            }
        }
    }
}

impl Error for RationalEquationVerificationError {}

fn classify(
    residual: &RationalFunction,
) -> Result<RationalEquationSolution, RationalEquationVerificationError> {
    let numerator = residual.numerator();
    match numerator.degree() {
        None => Ok(RationalEquationSolution::AllAdmissibleRationals),
        Some(0) => Ok(RationalEquationSolution::NoSolution),
        Some(1) => {
            let zero = Rational::from(Integer::zero());
            let constant = numerator.coefficients().first().unwrap_or(&zero);
            let coefficient = &numerator.coefficients()[1];
            let root = rational_quotient(&(-constant), coefficient);
            if residual
                .excluded_zero_sets()
                .iter()
                .any(|condition| condition.evaluate(&root).numerator().is_zero())
                || residual.denominator().evaluate(&root).numerator().is_zero()
            {
                Ok(RationalEquationSolution::NoSolution)
            } else {
                Ok(RationalEquationSolution::Unique(root))
            }
        }
        Some(_) => Err(RationalEquationVerificationError::UnsupportedDegree),
    }
}

/// Rebuilds LHS-RHS independently of the checked solver and checks the exact
/// solution classification, including every inherited denominator condition.
///
/// This verifier uses trusted low-level exact arithmetic, not random numeric
/// sampling. Checked callers must admit inputs before invoking it.
pub fn verify_rational_equation(
    result: &RationalEquationResult,
) -> Result<(), RationalEquationVerificationError> {
    let expected = result
        .left
        .subtract(&result.right)
        .map_err(|_| RationalEquationVerificationError::IncorrectResidual)?;
    if expected != result.residual {
        return Err(RationalEquationVerificationError::IncorrectResidual);
    }
    let classification = classify(&expected)?;
    if classification != result.solution {
        return Err(RationalEquationVerificationError::IncorrectSolution);
    }
    Ok(())
}

#[cfg(test)]
mod forged_certificate_tests {
    use super::*;
    use crate::EvaluationContext;

    fn poly(coefficients: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            coefficients
                .iter()
                .map(|&n| Rational::from(Integer::from(n)))
                .collect(),
        )
    }

    #[test]
    fn verifier_rejects_forged_root_and_domain() {
        let lhs = RationalFunction::new(poly(&[1, 2]), poly(&[0, 1])).unwrap();
        let rhs = RationalFunction::from_polynomial(poly(&[3]));
        let result = EvaluationContext::default()
            .solve_rational_equation(&lhs, &rhs)
            .unwrap();

        let mut forged = result.clone();
        forged.solution = RationalEquationSolution::Unique(Rational::from(Integer::from(2)));
        assert_eq!(
            verify_rational_equation(&forged),
            Err(RationalEquationVerificationError::IncorrectSolution)
        );

        let mut forged = result;
        forged.residual = RationalFunction::from_polynomial(poly(&[-1, 1]));
        assert_eq!(
            verify_rational_equation(&forged),
            Err(RationalEquationVerificationError::IncorrectResidual)
        );
    }
}
