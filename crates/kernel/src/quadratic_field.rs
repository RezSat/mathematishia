//! Quadratic fields with a distinguished real or complex embedding.

use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational};

use crate::poly_division::rational_quotient;
use crate::{
    EvaluationContext, EvaluationError, ExactQuadraticRoot, QuadraticBranch, RationalPolynomial,
};

fn zero() -> Rational {
    Rational::from(Integer::zero())
}

fn one() -> Rational {
    Rational::from(Integer::one())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuadraticFieldError {
    Evaluation(EvaluationError),
    NotQuadratic,
    ReduciblePolynomial,
    InvalidBranch,
    MismatchedFields,
    RationalRoot,
    DivisionByZero,
    InvalidMinimalPolynomial,
    Residue(ResidueError),
}

impl fmt::Display for QuadraticFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "{error}"),
            Self::NotQuadratic => f.write_str("the minimal polynomial must have degree two"),
            Self::ReduciblePolynomial => {
                f.write_str("the quadratic polynomial is reducible over Q")
            }
            Self::InvalidBranch => {
                f.write_str("the root branch is incompatible with the discriminant")
            }
            Self::MismatchedFields => f.write_str("quadratic elements use different embeddings"),
            Self::RationalRoot => f.write_str("a rational root does not define a quadratic field"),
            Self::DivisionByZero => f.write_str("cannot invert zero in a quadratic field"),
            Self::InvalidMinimalPolynomial => f.write_str("the computed minimal polynomial failed exact verification"),
            Self::Residue(error) => write!(f, "{error}"),
        }
    }
}

impl Error for QuadraticFieldError {}

impl From<ResidueError> for QuadraticFieldError {
    fn from(error: ResidueError) -> Self {
        Self::Residue(error)
    }
}

impl From<EvaluationError> for QuadraticFieldError {
    fn from(value: EvaluationError) -> Self {
        Self::Evaluation(value)
    }
}

/// Q(alpha) with an irreducible monic polynomial alpha^2 + b*alpha + c
/// and a selected embedding alpha into C.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QuadraticField {
    polynomial: RationalPolynomial,
    branch: QuadraticBranch,
}

impl QuadraticField {
    pub fn new(
        ctx: &EvaluationContext,
        polynomial: RationalPolynomial,
        branch: QuadraticBranch,
    ) -> Result<Self, QuadraticFieldError> {
        ctx.check_polynomial(&polynomial)?;
        if polynomial.degree() != Some(2) {
            return Err(QuadraticFieldError::NotQuadratic);
        }
        let leading = &polynomial.coefficients()[2];
        let inverse = rational_quotient(&one(), leading);
        let monic = RationalPolynomial::new(
            polynomial
                .coefficients()
                .iter()
                .map(|coefficient| coefficient * &inverse)
                .collect(),
        );
        ctx.check_polynomial(&monic)?;

        let b = &monic.coefficients()[1];
        let c = &monic.coefficients()[0];
        let discriminant = &(b * b) - &(&Rational::from(Integer::from(4)) * c);
        ctx.check_rational(&discriminant)?;
        if discriminant.exact_sqrt().is_some() {
            return Err(QuadraticFieldError::ReduciblePolynomial);
        }
        let valid = if discriminant < zero() {
            matches!(
                branch,
                QuadraticBranch::NegativeImaginary | QuadraticBranch::PositiveImaginary
            )
        } else {
            matches!(
                branch,
                QuadraticBranch::LowerReal | QuadraticBranch::UpperReal
            )
        };
        if !valid {
            return Err(QuadraticFieldError::InvalidBranch);
        }
        Ok(Self {
            polynomial: monic,
            branch,
        })
    }

    pub fn polynomial(&self) -> &RationalPolynomial {
        &self.polynomial
    }

    pub fn branch(&self) -> QuadraticBranch {
        self.branch
    }

    fn b(&self) -> &Rational {
        &self.polynomial.coefficients()[1]
    }

    fn c(&self) -> &Rational {
        &self.polynomial.coefficients()[0]
    }
}

/// An exact value a + b*alpha in one selected quadratic embedding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuadraticElement {
    field: QuadraticField,
    constant: Rational,
    coefficient: Rational,
}

impl QuadraticElement {
    pub fn new(
        ctx: &EvaluationContext,
        field: QuadraticField,
        constant: Rational,
        coefficient: Rational,
    ) -> Result<Self, QuadraticFieldError> {
        ctx.check_rational(&constant)?;
        ctx.check_rational(&coefficient)?;
        Ok(Self {
            field,
            constant,
            coefficient,
        })
    }

    pub fn from_root(
        ctx: &EvaluationContext,
        root: &ExactQuadraticRoot,
    ) -> Result<Self, QuadraticFieldError> {
        let ExactQuadraticRoot::Algebraic {
            minimal_polynomial,
            branch,
        } = root
        else {
            return Err(QuadraticFieldError::RationalRoot);
        };
        let field = QuadraticField::new(ctx, minimal_polynomial.clone(), *branch)?;
        Self::new(ctx, field, zero(), one())
    }

    pub fn field(&self) -> &QuadraticField {
        &self.field
    }

    pub fn constant(&self) -> &Rational {
        &self.constant
    }

    pub fn coefficient(&self) -> &Rational {
        &self.coefficient
    }

    fn compatible(&self, other: &Self) -> Result<(), QuadraticFieldError> {
        if self.field == other.field {
            Ok(())
        } else {
            Err(QuadraticFieldError::MismatchedFields)
        }
    }

    pub fn add(&self, other: &Self, ctx: &EvaluationContext) -> Result<Self, QuadraticFieldError> {
        self.compatible(other)?;
        Self::new(
            ctx,
            self.field.clone(),
            &self.constant + &other.constant,
            &self.coefficient + &other.coefficient,
        )
    }

    pub fn subtract(
        &self,
        other: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, QuadraticFieldError> {
        self.compatible(other)?;
        Self::new(
            ctx,
            self.field.clone(),
            &self.constant - &other.constant,
            &self.coefficient - &other.coefficient,
        )
    }

    /// Uses alpha^2 = -b*alpha-c, with b,c from the monic minimal polynomial.
    pub fn multiply(
        &self,
        other: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, QuadraticFieldError> {
        self.compatible(other)?;
        let product = &self.coefficient * &other.coefficient;
        let constant = &(&self.constant * &other.constant) - &(self.field.c() * &product);
        let linear = &(&(&self.constant * &other.coefficient)
            + &(&self.coefficient * &other.constant))
            - &(self.field.b() * &product);
        Self::new(ctx, self.field.clone(), constant, linear)
    }

    pub fn conjugate(&self, ctx: &EvaluationContext) -> Result<Self, QuadraticFieldError> {
        Self::new(
            ctx,
            self.field.clone(),
            &self.constant - &(self.field.b() * &self.coefficient),
            -&self.coefficient,
        )
    }

    pub fn trace(&self, ctx: &EvaluationContext) -> Result<Rational, QuadraticFieldError> {
        let result = &(&Rational::from(Integer::from(2)) * &self.constant)
            - &(self.field.b() * &self.coefficient);
        ctx.check_rational(&result)?;
        Ok(result)
    }

    pub fn norm(&self, ctx: &EvaluationContext) -> Result<Rational, QuadraticFieldError> {
        let a2 = &self.constant * &self.constant;
        let ab = &self.constant * &self.coefficient;
        let b2 = &self.coefficient * &self.coefficient;
        let result = &(&a2 - &(self.field.b() * &ab)) + &(self.field.c() * &b2);
        ctx.check_rational(&result)?;
        Ok(result)
    }

    pub fn inverse(&self, ctx: &EvaluationContext) -> Result<Self, QuadraticFieldError> {
        let norm = self.norm(ctx)?;
        if norm.numerator().is_zero() {
            return Err(QuadraticFieldError::DivisionByZero);
        }
        let conjugate = self.conjugate(ctx)?;
        Self::new(
            ctx,
            self.field.clone(),
            rational_quotient(&conjugate.constant, &norm),
            rational_quotient(&conjugate.coefficient, &norm),
        )
    }

    pub fn divide(
        &self,
        other: &Self,
        ctx: &EvaluationContext,
    ) -> Result<Self, QuadraticFieldError> {
        self.compatible(other)?;
        self.multiply(&other.inverse(ctx)?, ctx)
    }

    /// Minimal polynomial of the value over Q (degree one for rational values).
    pub fn minimal_polynomial(
        &self,
        ctx: &EvaluationContext,
    ) -> Result<RationalPolynomial, QuadraticFieldError> {
        let result = if self.coefficient.numerator().is_zero() {
            RationalPolynomial::new(vec![-&self.constant, one()])
        } else {
            RationalPolynomial::new(vec![self.norm(ctx)?, -self.trace(ctx)?, one()])
        };
        ctx.check_polynomial(&result)?;
        if !verify_quadratic_minimal_polynomial(ctx, self, &result)? {
            return Err(QuadraticFieldError::InvalidMinimalPolynomial);
        }
        Ok(result)
    }

    /// Converts into an exact root identity with a normalized minimal polynomial.
    /// Multiplication by a negative rational reverses the selected root branch.
    pub fn as_exact_root(
        &self,
        ctx: &EvaluationContext,
    ) -> Result<ExactQuadraticRoot, QuadraticFieldError> {
        if self.coefficient.numerator().is_zero() {
            return Ok(ExactQuadraticRoot::Rational(self.constant.clone()));
        }
        let branch = if self.coefficient < zero() {
            match self.field.branch {
                QuadraticBranch::LowerReal => QuadraticBranch::UpperReal,
                QuadraticBranch::UpperReal => QuadraticBranch::LowerReal,
                QuadraticBranch::NegativeImaginary => QuadraticBranch::PositiveImaginary,
                QuadraticBranch::PositiveImaginary => QuadraticBranch::NegativeImaginary,
            }
        } else {
            self.field.branch
        };
        Ok(ExactQuadraticRoot::Algebraic {
            minimal_polynomial: self.minimal_polynomial(ctx)?,
            branch,
        })
    }
}

/// Independently checks a proposed monic minimal polynomial in Q(alpha).
/// Evaluation takes place in the quotient ring, rather than using the
/// trace/norm formula that constructs the candidate.
pub fn verify_quadratic_minimal_polynomial(
    ctx: &EvaluationContext,
    value: &QuadraticElement,
    candidate: &RationalPolynomial,
) -> Result<bool, QuadraticFieldError> {
    ctx.check_polynomial(candidate)?;
    let degree = if value.coefficient.numerator().is_zero() {
        1
    } else {
        2
    };
    if candidate.degree() != Some(degree)
        || candidate.coefficients().last() != Some(&one())
    {
        return Ok(false);
    }
    let modulus = value.field.polynomial().clone();
    let element = PolynomialResidue::new(
        ctx,
        RationalPolynomial::new(vec![value.constant.clone(), value.coefficient.clone()]),
        modulus.clone(),
    )?;
    let mut accumulator = PolynomialResidue::new(
        ctx,
        RationalPolynomial::zero(),
        modulus.clone(),
    )?;
    for coefficient in candidate.coefficients().iter().rev() {
        let scalar = PolynomialResidue::new(
            ctx,
            RationalPolynomial::new(vec![coefficient.clone()]),
            modulus.clone(),
        )?;
        accumulator = accumulator.multiply(&element, ctx)?.add(&scalar, ctx)?;
    }
    Ok(accumulator.is_zero())
}

