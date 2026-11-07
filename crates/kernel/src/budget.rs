//! Deterministic, per-call work metering for checked exact algorithms.
//!
//! Arithmetic is guarded before large integer products are allocated. The
//! guards are conservative bit-size upper bounds: finite budgets may refuse
//! mathematically harmless cancellation. This does not provide operating
//! system memory/time isolation or bound construction performed by callers
//! before they invoke checked APIs.

use mathema::{Expr, ExprView, Integer, Rational, Symbol};

use crate::{
    EvaluationError, PolynomialConversionError, RationalPolynomial, ResourceLimitKind,
    ResourceLimits, SquareFreeDecomposition, SquareFreeError, SquareFreeFactor,
    SquareFreeVerificationError,
};

pub(crate) struct WorkBudget {
    limits: ResourceLimits,
    remaining: u64,
}

fn zero() -> Rational {
    Rational::from(Integer::zero())
}

fn one() -> Rational {
    Rational::from(Integer::one())
}

fn unit() -> RationalPolynomial {
    RationalPolynomial::new(vec![one()])
}

impl WorkBudget {
    pub(crate) fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            remaining: limits.max_work_units,
        }
    }

    /// A work unit is a visited expression node or a rational scalar
    /// arithmetic operation; expensive polynomial verification is metered
    /// through the same operations. It is not a calibrated CPU-time unit.
    pub(crate) fn charge(&mut self, units: u64) -> Result<(), EvaluationError> {
        self.remaining = self
            .remaining
            .checked_sub(units)
            .ok_or(EvaluationError::Limit(ResourceLimitKind::WorkUnits))?;
        Ok(())
    }

    fn size(&self, numerator_bits: u64, denominator_bits: u64) -> Result<(), EvaluationError> {
        if numerator_bits > self.limits.max_intermediate_scalar_bits
            || denominator_bits > self.limits.max_intermediate_scalar_bits
        {
            Err(EvaluationError::Limit(
                ResourceLimitKind::IntermediateScalarBits,
            ))
        } else {
            Ok(())
        }
    }

    fn scalar(&self, value: &Rational) -> Result<(), EvaluationError> {
        self.size(value.numerator().bit_len(), value.denominator().bit_len())
    }

    fn polynomial_shape(&self, polynomial: &RationalPolynomial) -> Result<(), EvaluationError> {
        if polynomial
            .degree()
            .is_some_and(|degree| degree > self.limits.max_polynomial_degree)
        {
            return Err(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree));
        }
        for coefficient in polynomial.coefficients() {
            self.scalar(coefficient)?;
        }
        Ok(())
    }

    fn add_scalar(&mut self, a: &Rational, b: &Rational) -> Result<Rational, EvaluationError> {
        self.charge(1)?;
        self.scalar(a)?;
        self.scalar(b)?;
        if a.numerator().is_zero() {
            return Ok(b.clone());
        }
        if b.numerator().is_zero() {
            return Ok(a.clone());
        }
        let first = a
            .numerator()
            .bit_len()
            .saturating_add(b.denominator().bit_len());
        let second = b
            .numerator()
            .bit_len()
            .saturating_add(a.denominator().bit_len());
        let numerator = first.max(second).saturating_add(1);
        let denominator = a
            .denominator()
            .bit_len()
            .saturating_add(b.denominator().bit_len());
        self.size(numerator, denominator)?;
        let output = a + b;
        self.scalar(&output)?;
        Ok(output)
    }

    fn subtract_scalar(&mut self, a: &Rational, b: &Rational) -> Result<Rational, EvaluationError> {
        self.add_scalar(a, &(-b))
    }

    fn multiply_scalar(&mut self, a: &Rational, b: &Rational) -> Result<Rational, EvaluationError> {
        self.charge(1)?;
        self.scalar(a)?;
        self.scalar(b)?;
        if a.numerator().is_zero() || b.numerator().is_zero() {
            return Ok(zero());
        }
        self.size(
            a.numerator()
                .bit_len()
                .saturating_add(b.numerator().bit_len()),
            a.denominator()
                .bit_len()
                .saturating_add(b.denominator().bit_len()),
        )?;
        let output = a * b;
        self.scalar(&output)?;
        Ok(output)
    }

    pub(crate) fn quotient(
        &mut self,
        a: &Rational,
        b: &Rational,
    ) -> Result<Rational, EvaluationError> {
        self.divide_scalar(a, b)
    }

    fn divide_scalar(&mut self, a: &Rational, b: &Rational) -> Result<Rational, EvaluationError> {
        self.charge(1)?;
        self.scalar(a)?;
        self.scalar(b)?;
        if b.numerator().is_zero() {
            return Err(EvaluationError::DivisionByZero);
        }
        if a.numerator().is_zero() {
            return Ok(zero());
        }
        self.size(
            a.numerator()
                .bit_len()
                .saturating_add(b.denominator().bit_len()),
            a.denominator()
                .bit_len()
                .saturating_add(b.numerator().bit_len()),
        )?;
        let result = Rational::new(
            a.numerator() * b.denominator(),
            a.denominator() * b.numerator(),
        )
        .expect("a nonzero divisor has a nonzero numerator");
        self.scalar(&result)?;
        Ok(result)
    }

    pub(crate) fn add(
        &mut self,
        a: &RationalPolynomial,
        b: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.polynomial_shape(a)?;
        self.polynomial_shape(b)?;
        let zero = zero();
        let capacity = a.coefficients().len().max(b.coefficients().len());
        let mut coefficients = Vec::with_capacity(capacity);
        for index in 0..capacity {
            let left = a.coefficients().get(index).unwrap_or(&zero);
            let right = b.coefficients().get(index).unwrap_or(&zero);
            coefficients.push(self.add_scalar(left, right)?);
        }
        let result = RationalPolynomial::new(coefficients);
        self.polynomial_shape(&result)?;
        Ok(result)
    }

    pub(crate) fn subtract(
        &mut self,
        a: &RationalPolynomial,
        b: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.polynomial_shape(a)?;
        self.polynomial_shape(b)?;
        let zero = zero();
        let capacity = a.coefficients().len().max(b.coefficients().len());
        let mut coefficients = Vec::with_capacity(capacity);
        for index in 0..capacity {
            let left = a.coefficients().get(index).unwrap_or(&zero);
            let right = b.coefficients().get(index).unwrap_or(&zero);
            coefficients.push(self.subtract_scalar(left, right)?);
        }
        let result = RationalPolynomial::new(coefficients);
        self.polynomial_shape(&result)?;
        Ok(result)
    }

    pub(crate) fn multiply(
        &mut self,
        a: &RationalPolynomial,
        b: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.polynomial_shape(a)?;
        self.polynomial_shape(b)?;
        if a.is_zero() || b.is_zero() {
            self.charge(1)?;
            return Ok(RationalPolynomial::zero());
        }
        let degree = a
            .degree()
            .unwrap_or(0)
            .checked_add(b.degree().unwrap_or(0))
            .ok_or(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree))?;
        if degree > self.limits.max_polynomial_degree {
            return Err(EvaluationError::Limit(ResourceLimitKind::PolynomialDegree));
        }
        let mut coefficients = vec![zero(); degree + 1];
        for (i, left) in a.coefficients().iter().enumerate() {
            for (j, right) in b.coefficients().iter().enumerate() {
                let product = self.multiply_scalar(left, right)?;
                coefficients[i + j] = self.add_scalar(&coefficients[i + j], &product)?;
            }
        }
        let result = RationalPolynomial::new(coefficients);
        self.polynomial_shape(&result)?;
        Ok(result)
    }

    pub(crate) fn differentiate(
        &mut self,
        polynomial: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.polynomial_shape(polynomial)?;
        let mut terms = Vec::with_capacity(polynomial.coefficients().len().saturating_sub(1));
        for (power, coefficient) in polynomial.coefficients().iter().enumerate().skip(1) {
            terms.push(self.multiply_scalar(coefficient, &Rational::from(Integer::from(power)))?);
        }
        let result = RationalPolynomial::new(terms);
        self.polynomial_shape(&result)?;
        Ok(result)
    }

    pub(crate) fn evaluate(
        &mut self,
        polynomial: &RationalPolynomial,
        at: &Rational,
    ) -> Result<Rational, EvaluationError> {
        self.polynomial_shape(polynomial)?;
        self.scalar(at)?;
        let mut value = zero();
        for coefficient in polynomial.coefficients().iter().rev() {
            value = self.multiply_scalar(&value, at)?;
            value = self.add_scalar(&value, coefficient)?;
        }
        Ok(value)
    }

    pub(crate) fn divide(
        &mut self,
        dividend: &RationalPolynomial,
        divisor: &RationalPolynomial,
    ) -> Result<(RationalPolynomial, RationalPolynomial), EvaluationError> {
        self.polynomial_shape(dividend)?;
        self.polynomial_shape(divisor)?;
        let divisor_degree = divisor.degree().ok_or(EvaluationError::DivisionByZero)?;
        let Some(dividend_degree) = dividend.degree() else {
            return Ok((RationalPolynomial::zero(), RationalPolynomial::zero()));
        };
        if dividend_degree < divisor_degree {
            return Ok((RationalPolynomial::zero(), dividend.clone()));
        }
        let mut quotient = vec![zero(); dividend_degree - divisor_degree + 1];
        let mut remainder = dividend.clone();
        while let Some(remainder_degree) = remainder.degree() {
            if remainder_degree < divisor_degree {
                break;
            }
            self.charge(1)?;
            let offset = remainder_degree - divisor_degree;
            let factor = self.divide_scalar(
                &remainder.coefficients()[remainder_degree],
                &divisor.coefficients()[divisor_degree],
            )?;
            quotient[offset] = self.add_scalar(&quotient[offset], &factor)?;
            let mut subtraction = vec![zero(); offset + divisor_degree + 1];
            for (index, coefficient) in divisor.coefficients().iter().enumerate() {
                subtraction[offset + index] = self.multiply_scalar(coefficient, &factor)?;
            }
            remainder = self.subtract(&remainder, &RationalPolynomial::new(subtraction))?;
        }
        let quotient = RationalPolynomial::new(quotient);
        self.polynomial_shape(&quotient)?;
        self.polynomial_shape(&remainder)?;
        Ok((quotient, remainder))
    }

    pub(crate) fn divide_exact(
        &mut self,
        a: &RationalPolynomial,
        b: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        let (quotient, remainder) = self.divide(a, b)?;
        if !remainder.is_zero() {
            return Err(EvaluationError::SquareFree(
                SquareFreeError::NonExactDivision,
            ));
        }
        Ok(quotient)
    }

    pub(crate) fn gcd(
        &mut self,
        left: &RationalPolynomial,
        right: &RationalPolynomial,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.polynomial_shape(left)?;
        self.polynomial_shape(right)?;
        let mut a = left.clone();
        let mut b = right.clone();
        while !b.is_zero() {
            self.charge(1)?;
            let (_, remainder) = self.divide(&a, &b)?;
            a = b;
            b = remainder;
        }
        let Some(leading) = a.coefficients().last().cloned() else {
            return Ok(RationalPolynomial::zero());
        };
        let mut normalized = Vec::with_capacity(a.coefficients().len());
        for coefficient in a.coefficients() {
            normalized.push(self.divide_scalar(coefficient, &leading)?);
        }
        let result = RationalPolynomial::new(normalized);
        self.polynomial_shape(&result)?;
        Ok(result)
    }

    pub(crate) fn square_free(
        &mut self,
        input: &RationalPolynomial,
    ) -> Result<SquareFreeDecomposition, EvaluationError> {
        self.polynomial_shape(input)?;
        let Some(leading) = input.coefficients().last().cloned() else {
            return Err(EvaluationError::SquareFree(SquareFreeError::ZeroPolynomial));
        };
        let mut normalized = Vec::with_capacity(input.coefficients().len());
        for coefficient in input.coefficients() {
            normalized.push(self.divide_scalar(coefficient, &leading)?);
        }
        let polynomial = RationalPolynomial::new(normalized);
        let derivative = self.differentiate(&polynomial)?;
        let mut gcd = self.gcd(&polynomial, &derivative)?;
        let mut remaining = self.divide_exact(&polynomial, &gcd)?;
        let mut multiplicity = 1usize;
        let mut factors = Vec::new();
        while remaining.degree().is_some_and(|degree| degree > 0) {
            self.charge(1)?;
            let common = self.gcd(&remaining, &gcd)?;
            let factor = self.divide_exact(&remaining, &common)?;
            if factor.degree().is_some_and(|degree| degree > 0) {
                factors.push(SquareFreeFactor::from_parts(factor, multiplicity));
            }
            remaining = common.clone();
            gcd = self.divide_exact(&gcd, &common)?;
            multiplicity += 1;
        }
        Ok(SquareFreeDecomposition::from_parts(leading, factors))
    }

    /// Checks a reported result via exact algebraic invariants, not by
    /// calling the generator. Work is metered through the same scalar math.
    pub(crate) fn verify_square_free(
        &mut self,
        original: &RationalPolynomial,
        result: &SquareFreeDecomposition,
    ) -> Result<(), EvaluationError> {
        use SquareFreeVerificationError as V;

        self.polynomial_shape(original)?;
        let Some(leading) = original.coefficients().last() else {
            return Err(V::ZeroPolynomial.into());
        };
        if result.unit() != leading || result.unit().numerator().is_zero() {
            return Err(V::InvalidUnit.into());
        }

        let mut degree = 0usize;
        let mut last = 0usize;
        let one = one();
        let unity = unit();
        for entry in result.factors() {
            self.charge(1)?;
            let factor = entry.polynomial();
            self.polynomial_shape(factor)?;
            let Some(n) = factor.degree() else {
                return Err(V::InvalidFactor.into());
            };
            if n == 0 || factor.coefficients().last() != Some(&one) {
                return Err(V::InvalidFactor.into());
            }
            if entry.multiplicity() <= last {
                return Err(V::InvalidMultiplicity.into());
            }
            last = entry.multiplicity();
            degree = n
                .checked_mul(entry.multiplicity())
                .and_then(|value| degree.checked_add(value))
                .ok_or(V::WrongDegree)?;
            if degree > original.degree().unwrap_or(0) {
                return Err(V::WrongDegree.into());
            }
            let derivative = self.differentiate(factor)?;
            if self.gcd(factor, &derivative)? != unity {
                return Err(V::NotSquareFree.into());
            }
        }
        if degree != original.degree().unwrap_or(0) {
            return Err(V::WrongDegree.into());
        }
        for (index, a) in result.factors().iter().enumerate() {
            for b in result.factors().iter().skip(index + 1) {
                if self.gcd(a.polynomial(), b.polynomial())? != unity {
                    return Err(V::NotCoprime.into());
                }
            }
        }
        let mut product = unit();
        for entry in result.factors() {
            for _ in 0..entry.multiplicity() {
                product = self.multiply(&product, entry.polynomial())?;
            }
        }
        let scale = RationalPolynomial::new(vec![result.unit().clone()]);
        if self.multiply(&product, &scale)? != *original {
            return Err(V::ProductMismatch.into());
        }
        Ok(())
    }

    /// Bounded exact conversion over Q[x]; rejects nonpolynomial structure and
    /// excessive polynomial growth before doing dense multiplication.
    pub(crate) fn convert(
        &mut self,
        expr: &Expr,
        variable: &Symbol,
    ) -> Result<RationalPolynomial, EvaluationError> {
        self.charge(1)?;
        let output = match expr.view() {
            ExprView::Integer(value) => {
                let value = Rational::from(value.clone());
                self.scalar(&value)?;
                RationalPolynomial::new(vec![value])
            }
            ExprView::Rational(value) => {
                self.scalar(value)?;
                RationalPolynomial::new(vec![value.clone()])
            }
            ExprView::Symbol(found) if found == variable => {
                if self.limits.max_polynomial_degree == 0 {
                    return Err(PolynomialConversionError::DegreeLimitExceeded.into());
                }
                RationalPolynomial::new(vec![zero(), one()])
            }
            ExprView::Symbol(_) => return Err(PolynomialConversionError::OtherSymbol.into()),
            ExprView::Add(terms) => {
                let mut sum = RationalPolynomial::zero();
                for term in terms {
                    let converted = self.convert(term, variable)?;
                    sum = self.add(&sum, &converted)?;
                }
                sum
            }
            ExprView::Mul(factors) => {
                let mut product = unit();
                for factor in factors {
                    let converted = self.convert(factor, variable)?;
                    product = self.multiply(&product, &converted)?;
                }
                product
            }
            ExprView::Pow { base, exponent } => {
                let ExprView::Integer(value) = exponent.view() else {
                    return Err(PolynomialConversionError::UnsupportedPower.into());
                };
                if value <= &Integer::zero() {
                    return Err(PolynomialConversionError::UnsupportedPower.into());
                }
                let power: usize = value
                    .to_string()
                    .parse()
                    .map_err(|_| PolynomialConversionError::DegreeLimitExceeded)?;
                if power > self.limits.max_polynomial_degree {
                    return Err(PolynomialConversionError::DegreeLimitExceeded.into());
                }
                let mut factor = self.convert(base, variable)?;
                if factor
                    .degree()
                    .and_then(|degree| degree.checked_mul(power))
                    .is_some_and(|degree| degree > self.limits.max_polynomial_degree)
                {
                    return Err(PolynomialConversionError::DegreeLimitExceeded.into());
                }
                let mut result = unit();
                let mut remaining = power;
                while remaining > 0 {
                    if remaining & 1 != 0 {
                        result = self.multiply(&result, &factor)?;
                    }
                    remaining >>= 1;
                    if remaining > 0 {
                        factor = self.multiply(&factor, &factor)?;
                    }
                }
                result
            }
        };
        self.polynomial_shape(&output)?;
        Ok(output)
    }
}
