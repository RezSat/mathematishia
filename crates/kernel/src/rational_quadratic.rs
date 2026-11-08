//! Domain-aware exact rational roots for rational equations with numerator degree <= 2.
//!
//! The domain is the intersection of the original rational functions' domains.
//! Zeros of canceled denominators remain excluded. No rational roots is not
//! equivalent to no solutions over R or C.

use mathema::Rational;

use crate::quadratic::independent_quadratic_roots;
use crate::{
    QuadraticRoots, QuadraticVerificationError, RationalFunction, RationalPolynomial,
};

/// Exact rational solutions within the common (possibly punctured) domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RationalQuadraticRoots {
    Finite(Vec<Rational>),
    AllAdmissibleRationals,
}

/// A quadratic-or-lower rational equation and its complete rational solutions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RationalQuadraticResult {
    left: RationalFunction,
    right: RationalFunction,
    residual: RationalFunction,
    roots: RationalQuadraticRoots,
}

impl RationalQuadraticResult {
    pub fn left(&self) -> &RationalFunction {
        &self.left
    }

    pub fn right(&self) -> &RationalFunction {
        &self.right
    }

    pub fn residual(&self) -> &RationalFunction {
        &self.residual
    }

    pub fn roots(&self) -> &RationalQuadraticRoots {
        &self.roots
    }

    pub fn domain_conditions(&self) -> &[RationalPolynomial] {
        self.residual.excluded_zero_sets()
    }

    pub(crate) fn from_parts(
        left: RationalFunction,
        right: RationalFunction,
        residual: RationalFunction,
        roots: RationalQuadraticRoots,
    ) -> Self {
        Self {
            left,
            right,
            residual,
            roots,
        }
    }
}

fn admissible(function: &RationalFunction, value: &Rational) -> bool {
    function
        .excluded_zero_sets()
        .iter()
        .all(|condition| !condition.evaluate(value).numerator().is_zero())
        && !function.denominator().evaluate(value).numerator().is_zero()
}

/// Reconstructs the equation residual independently and checks *all* rational
/// roots using the exact polynomial-discriminant criterion, followed by domain
/// exclusion. This does not call the checked equation solver.
pub fn verify_rational_quadratic_equation(
    result: &RationalQuadraticResult,
) -> Result<(), QuadraticVerificationError> {
    let reconstructed = result
        .left
        .subtract(&result.right)
        .map_err(|_| QuadraticVerificationError::IncorrectResidual)?;
    if reconstructed != result.residual {
        return Err(QuadraticVerificationError::IncorrectResidual);
    }
    let expected = match independent_quadratic_roots(reconstructed.numerator())? {
        QuadraticRoots::AllRationals => RationalQuadraticRoots::AllAdmissibleRationals,
        QuadraticRoots::Finite(values) => RationalQuadraticRoots::Finite(
            values
                .into_iter()
                .filter(|candidate| admissible(&reconstructed, candidate))
                .collect(),
        ),
    };
    if expected == result.roots {
        Ok(())
    } else {
        Err(QuadraticVerificationError::IncorrectRoots)
    }
}

#[cfg(test)]
mod forged {
    use super::*;
    use mathema::{Integer, Rational};
    use crate::EvaluationContext;

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(
            values
                .iter()
                .map(|&n| Rational::from(Integer::from(n)))
                .collect(),
        )
    }

    #[test]
    fn verifier_rejects_spurious_canceled_pole_and_wrong_residual() {
        let lhs = RationalFunction::new(p(&[6, -5, 1]), p(&[-2, 1])).unwrap();
        let rhs = RationalFunction::from_polynomial(p(&[]));
        let mut result = EvaluationContext::default()
            .solve_rational_quadratic_equation(&lhs, &rhs)
            .unwrap();
        result.roots = RationalQuadraticRoots::Finite(vec![
            Rational::from(Integer::from(2)),
            Rational::from(Integer::from(3)),
        ]);
        assert_eq!(
            verify_rational_quadratic_equation(&result),
            Err(QuadraticVerificationError::IncorrectRoots)
        );
        result.residual = RationalFunction::from_polynomial(p(&[1]));
        assert_eq!(
            verify_rational_quadratic_equation(&result),
            Err(QuadraticVerificationError::IncorrectResidual)
        );
    }
}
