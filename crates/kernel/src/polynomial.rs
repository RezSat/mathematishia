use mathema::{Integer, Rational};

/// An exact univariate polynomial over the rational numbers.
///
/// Coefficients are stored in ascending degree order: index i is the
/// coefficient of x^i. Trailing zeros are removed, and the zero polynomial
/// is represented by an empty vector. The indeterminate is implicit; this
/// type does not own a symbolic variable or parse expressions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RationalPolynomial {
    coefficients: Vec<Rational>,
}

impl RationalPolynomial {
    /// Constructs a polynomial, removing all trailing zero coefficients.
    pub fn new(mut coefficients: Vec<Rational>) -> Self {
        while coefficients
            .last()
            .is_some_and(|coefficient| coefficient.numerator().is_zero())
        {
            coefficients.pop();
        }
        Self { coefficients }
    }

    /// Returns the canonical zero polynomial.
    pub fn zero() -> Self {
        Self {
            coefficients: Vec::new(),
        }
    }

    /// Returns whether every coefficient is zero.
    pub fn is_zero(&self) -> bool {
        self.coefficients.is_empty()
    }

    /// Returns the polynomial's degree, or None for the zero polynomial.
    pub fn degree(&self) -> Option<usize> {
        self.coefficients.len().checked_sub(1)
    }

    /// Returns canonical coefficients in ascending degree order.
    pub fn coefficients(&self) -> &[Rational] {
        &self.coefficients
    }

    /// Adds two polynomials exactly.
    pub fn add(&self, other: &Self) -> Self {
        let zero = Rational::from(Integer::zero());
        let coefficients = (0..self.coefficients.len().max(other.coefficients.len()))
            .map(|index| {
                let left = self.coefficients.get(index).unwrap_or(&zero);
                let right = other.coefficients.get(index).unwrap_or(&zero);
                left + right
            })
            .collect();
        Self::new(coefficients)
    }

    /// Subtracts two polynomials exactly.
    pub fn subtract(&self, other: &Self) -> Self {
        let zero = Rational::from(Integer::zero());
        let coefficients = (0..self.coefficients.len().max(other.coefficients.len()))
            .map(|index| {
                let left = self.coefficients.get(index).unwrap_or(&zero);
                let right = other.coefficients.get(index).unwrap_or(&zero);
                left - right
            })
            .collect();
        Self::new(coefficients)
    }

    /// Multiplies polynomials by exact coefficient convolution.
    ///
    /// This straightforward quadratic algorithm is the baseline; it is not
    /// a claim of asymptotically optimal multiplication.
    pub fn multiply(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }

        let mut coefficients = vec![
            Rational::from(Integer::zero());
            self.coefficients.len() + other.coefficients.len() - 1
        ];

        for (i, left) in self.coefficients.iter().enumerate() {
            for (j, right) in other.coefficients.iter().enumerate() {
                let product = left * right;
                coefficients[i + j] = &coefficients[i + j] + &product;
            }
        }
        Self::new(coefficients)
    }

    /// Evaluates exactly with Horner's method.
    pub fn evaluate(&self, at: &Rational) -> Rational {
        self.coefficients
            .iter()
            .rev()
            .fold(Rational::from(Integer::zero()), |value, coefficient| {
                &(&value * at) + coefficient
            })
    }
}
