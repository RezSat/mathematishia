//! Canonical sparse multivariate polynomials over Q.
//!
//! Variables are an explicitly selected ambient ring, ordered by Symbol name.
//! Exponent vectors are nonnegative and follow that order. Operations never
//! implicitly extend a ring or convert semantic expressions.
//!
//! Bounds are conservative: an intermediate can be rejected even when later
//! cancellation would make the final polynomial small.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use mathema::{Integer, Rational, Symbol};

/// Per-operation admission limits, additionally capped by HARD_LIMITS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultivariateLimits {
    pub max_variables: usize,
    pub max_terms: usize,
    pub max_input_terms: usize,
    pub max_total_degree: u32,
    pub max_scalar_bits: u64,
    pub max_work_units: u64,
}

impl MultivariateLimits {
    /// Hard upper bounds, regardless of caller-provided limits.
    pub const HARD_LIMITS: Self = Self {
        max_variables: 16,
        max_terms: 2048,
        max_input_terms: 8192,
        max_total_degree: 128,
        max_scalar_bits: 4096,
        max_work_units: 250_000,
    };

    fn bounded(self) -> Self {
        let hard = Self::HARD_LIMITS;
        Self {
            max_variables: self.max_variables.min(hard.max_variables),
            max_terms: self.max_terms.min(hard.max_terms),
            max_input_terms: self.max_input_terms.min(hard.max_input_terms),
            max_total_degree: self.max_total_degree.min(hard.max_total_degree),
            max_scalar_bits: self.max_scalar_bits.min(hard.max_scalar_bits),
            max_work_units: self.max_work_units.min(hard.max_work_units),
        }
    }
}

impl Default for MultivariateLimits {
    fn default() -> Self {
        Self::HARD_LIMITS
    }
}

/// Explicit invalid inputs and conservative resource refusals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultivariateError {
    DuplicateVariable,
    VariableMismatch,
    UnknownVariable,
    ExponentArity,
    NegativeExponent,
    ValueArity,
    VariableLimit,
    TotalDegreeLimit,
    TermLimit,
    InputTermLimit,
    ScalarBitLimit,
    WorkLimit,
}

impl fmt::Display for MultivariateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            Self::DuplicateVariable => "ambient variables must be distinct",
            Self::VariableMismatch => "polynomials belong to different ambient rings",
            Self::UnknownVariable => "variable is not in the ambient ring",
            Self::ExponentArity => "exponent vector does not match the variable count",
            Self::NegativeExponent => "polynomial exponents must be nonnegative",
            Self::ValueArity => "evaluation point does not match the variable count",
            Self::VariableLimit => "multivariate variable limit exceeded",
            Self::TotalDegreeLimit => "multivariate total degree limit exceeded",
            Self::TermLimit => "multivariate distinct term limit exceeded",
            Self::InputTermLimit => "multivariate input or intermediate term limit exceeded",
            Self::ScalarBitLimit => "exact scalar bit limit exceeded",
            Self::WorkLimit => "multivariate arithmetic work limit exceeded",
        };
        f.write_str(description)
    }
}

impl Error for MultivariateError {}

fn zero_scalar() -> Rational {
    Rational::from(Integer::zero())
}

fn is_zero(value: &Rational) -> bool {
    value.numerator().is_zero()
}

struct Meter {
    limits: MultivariateLimits,
    remaining: u64,
}

impl Meter {
    fn new(limits: MultivariateLimits) -> Self {
        let limits = limits.bounded();
        Self {
            remaining: limits.max_work_units,
            limits,
        }
    }

    fn charge(&mut self, amount: u64) -> Result<(), MultivariateError> {
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or(MultivariateError::WorkLimit)?;
        Ok(())
    }

    fn scalar(&self, value: &Rational) -> Result<(), MultivariateError> {
        if value.numerator().bit_len() > self.limits.max_scalar_bits
            || value.denominator().bit_len() > self.limits.max_scalar_bits
        {
            Err(MultivariateError::ScalarBitLimit)
        } else {
            Ok(())
        }
    }

    fn bits(&self, numerator: u64, denominator: u64) -> Result<(), MultivariateError> {
        if numerator > self.limits.max_scalar_bits || denominator > self.limits.max_scalar_bits {
            Err(MultivariateError::ScalarBitLimit)
        } else {
            Ok(())
        }
    }

    fn add(&mut self, left: &Rational, right: &Rational) -> Result<Rational, MultivariateError> {
        self.charge(1)?;
        self.scalar(left)?;
        self.scalar(right)?;
        if is_zero(left) {
            return Ok(right.clone());
        }
        if is_zero(right) {
            return Ok(left.clone());
        }
        self.bits(
            left.numerator()
                .bit_len()
                .saturating_add(right.denominator().bit_len())
                .max(
                    right
                        .numerator()
                        .bit_len()
                        .saturating_add(left.denominator().bit_len()),
                )
                .saturating_add(1),
            left.denominator()
                .bit_len()
                .saturating_add(right.denominator().bit_len()),
        )?;
        let result = left + right;
        self.scalar(&result)?;
        Ok(result)
    }

    fn mul(&mut self, left: &Rational, right: &Rational) -> Result<Rational, MultivariateError> {
        self.charge(1)?;
        self.scalar(left)?;
        self.scalar(right)?;
        if is_zero(left) || is_zero(right) {
            return Ok(zero_scalar());
        }
        if left == &Rational::from(Integer::one()) {
            return Ok(right.clone());
        }
        if right == &Rational::from(Integer::one()) {
            return Ok(left.clone());
        }
        self.bits(
            left.numerator()
                .bit_len()
                .saturating_add(right.numerator().bit_len()),
            left.denominator()
                .bit_len()
                .saturating_add(right.denominator().bit_len()),
        )?;
        let result = left * right;
        self.scalar(&result)?;
        Ok(result)
    }

    fn power(
        &mut self,
        value: &Rational,
        mut exponent: u32,
    ) -> Result<Rational, MultivariateError> {
        self.scalar(value)?;
        let mut result = Rational::from(Integer::one());
        let mut base = value.clone();
        while exponent != 0 {
            if exponent & 1 != 0 {
                result = self.mul(&result, &base)?;
            }
            exponent >>= 1;
            if exponent != 0 {
                base = self.mul(&base, &base)?;
            }
        }
        Ok(result)
    }
}

/// A polynomial in Q[variables] with exact, nonzero sparse coefficients.
///
/// The zero polynomial has no terms, but retains its ambient variable list.
/// Distinct ambient rings are not implicitly identified, even for constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultivariatePolynomial {
    variables: Vec<Symbol>,
    terms: BTreeMap<Vec<u32>, Rational>,
}

impl MultivariatePolynomial {
    fn canonical_variables(
        mut variables: Vec<Symbol>,
        meter: &mut Meter,
    ) -> Result<Vec<Symbol>, MultivariateError> {
        if variables.len() > meter.limits.max_variables {
            return Err(MultivariateError::VariableLimit);
        }
        variables.sort();
        if variables.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(MultivariateError::DuplicateVariable);
        }
        Ok(variables)
    }

    fn total_exponent_degree(exponents: &[u32], meter: &Meter) -> Result<u32, MultivariateError> {
        let degree = exponents.iter().try_fold(0_u32, |sum, exponent| {
            sum.checked_add(*exponent)
                .ok_or(MultivariateError::TotalDegreeLimit)
        })?;
        if degree > meter.limits.max_total_degree {
            return Err(MultivariateError::TotalDegreeLimit);
        }
        Ok(degree)
    }

    fn from_terms(
        variables: Vec<Symbol>,
        input: impl IntoIterator<Item = (Vec<u32>, Rational)>,
        meter: &mut Meter,
    ) -> Result<Self, MultivariateError> {
        let mut grouped: BTreeMap<Vec<u32>, Vec<Rational>> = BTreeMap::new();
        let mut seen = 0_usize;
        for (exponents, coefficient) in input {
            meter.charge(1)?;
            seen = seen
                .checked_add(1)
                .ok_or(MultivariateError::InputTermLimit)?;
            if seen > meter.limits.max_input_terms {
                return Err(MultivariateError::InputTermLimit);
            }
            if exponents.len() != variables.len() {
                return Err(MultivariateError::ExponentArity);
            }
            Self::total_exponent_degree(&exponents, meter)?;
            meter.scalar(&coefficient)?;
            if !is_zero(&coefficient) {
                grouped.entry(exponents).or_default().push(coefficient);
            }
        }
        if grouped.len() > meter.limits.max_terms {
            return Err(MultivariateError::TermLimit);
        }
        let mut terms = BTreeMap::new();
        for (exponents, mut coefficients) in grouped {
            // Sorting before exact accumulation makes limit behavior independent
            // of the caller's term insertion order.
            coefficients.sort();
            let mut sum = zero_scalar();
            for coefficient in coefficients {
                sum = meter.add(&sum, &coefficient)?;
            }
            if !is_zero(&sum) {
                terms.insert(exponents, sum);
            }
        }
        Ok(Self { variables, terms })
    }

    /// Constructs a polynomial from exponent vectors indexed by the supplied
    /// variable order. Names are then sorted lexicographically and powers remapped.
    ///
    /// Duplicate names, negative powers and excessive input or intermediate
    /// complexity are rejected. Repeated monomials are collected exactly.
    pub fn try_new(
        variables: Vec<Symbol>,
        terms: impl IntoIterator<Item = (Vec<i32>, Rational)>,
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateError> {
        let mut meter = Meter::new(limits);
        let sorted = Self::canonical_variables(variables.clone(), &mut meter)?;
        let positions: Vec<usize> = sorted
            .iter()
            .map(|name| {
                variables
                    .iter()
                    .position(|candidate| candidate == name)
                    .expect("each sorted variable came from the input")
            })
            .collect();
        let mut converted = Vec::new();
        for (powers, coefficient) in terms {
            if converted.len() >= meter.limits.max_input_terms {
                return Err(MultivariateError::InputTermLimit);
            }
            if powers.len() != variables.len() {
                return Err(MultivariateError::ExponentArity);
            }
            let ordered = positions
                .iter()
                .map(|position| {
                    u32::try_from(powers[*position])
                        .map_err(|_| MultivariateError::NegativeExponent)
                })
                .collect::<Result<Vec<_>, _>>()?;
            converted.push((ordered, coefficient));
        }
        Self::from_terms(sorted, converted, &mut meter)
    }

    /// Constructs zero in the explicitly selected ambient ring.
    pub fn zero(
        variables: Vec<Symbol>,
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateError> {
        let mut meter = Meter::new(limits);
        let variables = Self::canonical_variables(variables, &mut meter)?;
        Ok(Self {
            variables,
            terms: BTreeMap::new(),
        })
    }

    /// Constructs a scalar polynomial in an explicitly selected ambient ring.
    pub fn constant(
        variables: Vec<Symbol>,
        value: Rational,
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateError> {
        let mut meter = Meter::new(limits);
        let variables = Self::canonical_variables(variables, &mut meter)?;
        let exponents = vec![0; variables.len()];
        Self::from_terms(variables, [(exponents, value)], &mut meter)
    }

    pub fn variables(&self) -> &[Symbol] {
        &self.variables
    }

    /// Canonical term keys are exponent vectors in variable-name order.
    pub fn terms(&self) -> &BTreeMap<Vec<u32>, Rational> {
        &self.terms
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Total degree; the zero polynomial has no degree.
    pub fn total_degree(&self) -> Option<u32> {
        self.terms
            .keys()
            .map(|powers| powers.iter().sum::<u32>())
            .max()
    }

    /// Degree in the named variable; zero has no degree.
    pub fn degree_in(&self, variable: &Symbol) -> Result<Option<u32>, MultivariateError> {
        let position = self.index_of(variable)?;
        Ok(self.terms.keys().map(|powers| powers[position]).max())
    }

    pub fn coefficient(&self, powers: &[u32]) -> Result<Rational, MultivariateError> {
        if powers.len() != self.variables.len() {
            return Err(MultivariateError::ExponentArity);
        }
        Ok(self.terms.get(powers).cloned().unwrap_or_else(zero_scalar))
    }

    fn index_of(&self, variable: &Symbol) -> Result<usize, MultivariateError> {
        self.variables
            .binary_search(variable)
            .map_err(|_| MultivariateError::UnknownVariable)
    }

    fn admit(&self, meter: &Meter) -> Result<(), MultivariateError> {
        if self.variables.len() > meter.limits.max_variables {
            return Err(MultivariateError::VariableLimit);
        }
        if self.terms.len() > meter.limits.max_terms {
            return Err(MultivariateError::TermLimit);
        }
        for (exponents, coefficient) in &self.terms {
            Self::total_exponent_degree(exponents, meter)?;
            meter.scalar(coefficient)?;
        }
        Ok(())
    }

    fn same_ring(&self, other: &Self) -> Result<(), MultivariateError> {
        if self.variables == other.variables {
            Ok(())
        } else {
            Err(MultivariateError::VariableMismatch)
        }
    }

    pub fn add(&self, other: &Self, limits: MultivariateLimits) -> Result<Self, MultivariateError> {
        self.same_ring(other)?;
        let mut meter = Meter::new(limits);
        self.admit(&meter)?;
        other.admit(&meter)?;
        Self::from_terms(
            self.variables.clone(),
            self.terms
                .iter()
                .chain(other.terms.iter())
                .map(|(powers, value)| (powers.clone(), value.clone())),
            &mut meter,
        )
    }

    pub fn subtract(
        &self,
        other: &Self,
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateError> {
        self.same_ring(other)?;
        let mut meter = Meter::new(limits);
        self.admit(&meter)?;
        other.admit(&meter)?;
        Self::from_terms(
            self.variables.clone(),
            self.terms
                .iter()
                .map(|(powers, value)| (powers.clone(), value.clone()))
                .chain(
                    other
                        .terms
                        .iter()
                        .map(|(powers, value)| (powers.clone(), -value)),
                ),
            &mut meter,
        )
    }

    pub fn multiply(
        &self,
        other: &Self,
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateError> {
        self.same_ring(other)?;
        let mut meter = Meter::new(limits);
        self.admit(&meter)?;
        other.admit(&meter)?;
        if self.is_zero() || other.is_zero() {
            return Self::zero(self.variables.clone(), limits);
        }
        if self
            .terms
            .len()
            .checked_mul(other.terms.len())
            .is_none_or(|count| count > meter.limits.max_input_terms)
        {
            return Err(MultivariateError::InputTermLimit);
        }
        let mut products = Vec::new();
        for (left_powers, left_value) in &self.terms {
            for (right_powers, right_value) in &other.terms {
                meter.charge(1)?;
                let powers = left_powers
                    .iter()
                    .zip(right_powers)
                    .map(|(a, b)| a.checked_add(*b).ok_or(MultivariateError::TotalDegreeLimit))
                    .collect::<Result<Vec<_>, _>>()?;
                Self::total_exponent_degree(&powers, &meter)?;
                products.push((powers, meter.mul(left_value, right_value)?));
            }
        }
        Self::from_terms(self.variables.clone(), products, &mut meter)
    }

    /// Exact partial derivative; other variables and the ambient ring persist.
    pub fn differentiate(
        &self,
        variable: &Symbol,
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateError> {
        let position = self.index_of(variable)?;
        let mut meter = Meter::new(limits);
        self.admit(&meter)?;
        let mut derivatives = Vec::new();
        for (powers, value) in &self.terms {
            meter.charge(1)?;
            if powers[position] == 0 {
                continue;
            }
            let mut result_powers = powers.clone();
            result_powers[position] -= 1;
            let factor = Rational::from(Integer::from(powers[position]));
            derivatives.push((result_powers, meter.mul(value, &factor)?));
        }
        Self::from_terms(self.variables.clone(), derivatives, &mut meter)
    }

    /// Exact simultaneous evaluation in canonical variable-name order.
    pub fn evaluate(
        &self,
        values: &[Rational],
        limits: MultivariateLimits,
    ) -> Result<Rational, MultivariateError> {
        if values.len() != self.variables.len() {
            return Err(MultivariateError::ValueArity);
        }
        let mut meter = Meter::new(limits);
        self.admit(&meter)?;
        for value in values {
            meter.scalar(value)?;
        }
        let mut total = zero_scalar();
        for (powers, coefficient) in &self.terms {
            meter.charge(1)?;
            let mut term = coefficient.clone();
            for (power, value) in powers.iter().zip(values) {
                if *power != 0 {
                    let value_power = meter.power(value, *power)?;
                    term = meter.mul(&term, &value_power)?;
                }
            }
            total = meter.add(&total, &term)?;
        }
        Ok(total)
    }

    /// Substitutes one rational value, removing that variable from the ring.
    /// Does not use the simultaneous-evaluation implementation.
    pub fn substitute(
        &self,
        variable: &Symbol,
        at: &Rational,
        limits: MultivariateLimits,
    ) -> Result<Self, MultivariateError> {
        let position = self.index_of(variable)?;
        let mut meter = Meter::new(limits);
        self.admit(&meter)?;
        meter.scalar(at)?;
        let mut variables = self.variables.clone();
        variables.remove(position);
        let mut result = Vec::new();
        for (powers, coefficient) in &self.terms {
            meter.charge(1)?;
            let value_power = meter.power(at, powers[position])?;
            let new_coefficient = meter.mul(coefficient, &value_power)?;
            let mut new_powers = powers.clone();
            new_powers.remove(position);
            result.push((new_powers, new_coefficient));
        }
        Self::from_terms(variables, result, &mut meter)
    }
}

impl fmt::Display for MultivariatePolynomial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zero() {
            return f.write_str("0");
        }
        for (index, (powers, coefficient)) in self.terms.iter().enumerate() {
            if index != 0 {
                f.write_str(" + ")?;
            }
            write!(f, "{coefficient}")?;
            for (variable, power) in self.variables.iter().zip(powers) {
                if *power == 0 {
                    continue;
                }
                write!(f, "*(\"{}\")", variable.name().escape_default())?;
                if *power != 1 {
                    write!(f, "^{power}")?;
                }
            }
        }
        Ok(())
    }
}
