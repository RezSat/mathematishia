use std::error::Error;
use std::fmt;

use mathema::{Expr, ExprView, Integer, Rational, Symbol};

use crate::RationalPolynomial;

/// A semantic expression is outside the supported univariate rational-polynomial domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolynomialConversionError {
    /// A symbol other than the selected indeterminate occurred.
    OtherSymbol,
    /// Powers require positive, explicitly integer exponents fitting in usize.
    /// Zero powers are not interpreted because the expression IR leaves 0^0 undefined.
    UnsupportedPower,
    /// The requested degree/power bound would be exceeded.
    DegreeLimitExceeded,
}

impl fmt::Display for PolynomialConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OtherSymbol => f.write_str("expression contains another symbol"),
            Self::UnsupportedPower => f.write_str("unsupported polynomial power"),
            Self::DegreeLimitExceeded => f.write_str("polynomial degree or power limit exceeded"),
        }
    }
}

impl Error for PolynomialConversionError {}

impl RationalPolynomial {
    /// Interprets an exact expression as a polynomial in one chosen symbol.
    ///
    /// Addition and multiplication use polynomial arithmetic rather than
    /// general expression rewriting. Only positive integer powers are accepted:
    /// zero, negative and symbolic powers are deliberately rejected. Both the
    /// resulting degree and individual powers must not exceed max_degree;
    /// this explicit bound also limits constant-power evaluation.
    ///
    /// No assumptions about other symbols are made; they are rejected.
    pub fn try_from_expr(
        expr: &Expr,
        variable: &Symbol,
        max_degree: usize,
    ) -> Result<Self, PolynomialConversionError> {
        match expr.view() {
            ExprView::Integer(value) => {
                Ok(Self::new(vec![Rational::from(value.clone())]))
            }
            ExprView::Rational(value) => Ok(Self::new(vec![value.clone()])),
            ExprView::Symbol(value) if value == variable => {
                if max_degree < 1 {
                    return Err(PolynomialConversionError::DegreeLimitExceeded);
                }
                Ok(Self::new(vec![
                    Rational::from(Integer::zero()),
                    Rational::from(Integer::one()),
                ]))
            }
            ExprView::Symbol(_) => Err(PolynomialConversionError::OtherSymbol),
            ExprView::Add(children) => {
                let mut sum = Self::zero();
                for child in children {
                    sum = sum.add(&Self::try_from_expr(child, variable, max_degree)?);
                }
                Ok(sum)
            }
            ExprView::Mul(children) => {
                let mut product = Self::new(vec![Rational::from(Integer::one())]);
                for child in children {
                    let factor = Self::try_from_expr(child, variable, max_degree)?;
                    if let (Some(a), Some(b)) = (product.degree(), factor.degree()) {
                        if a.checked_add(b).is_none_or(|degree| degree > max_degree) {
                            return Err(PolynomialConversionError::DegreeLimitExceeded);
                        }
                    }
                    product = product.multiply(&factor);
                }
                Ok(product)
            }
            ExprView::Pow { base, exponent } => {
                let ExprView::Integer(integer) = exponent.view() else {
                    return Err(PolynomialConversionError::UnsupportedPower);
                };
                if integer <= &Integer::zero() {
                    return Err(PolynomialConversionError::UnsupportedPower);
                }
                let power = integer
                    .to_string()
                    .parse::<usize>()
                    .map_err(|_| PolynomialConversionError::DegreeLimitExceeded)?;
                if power > max_degree {
                    return Err(PolynomialConversionError::DegreeLimitExceeded);
                }
                let base_poly = Self::try_from_expr(base, variable, max_degree)?;
                if base_poly
                    .degree()
                    .and_then(|degree| degree.checked_mul(power))
                    .is_some_and(|degree| degree > max_degree)
                {
                    return Err(PolynomialConversionError::DegreeLimitExceeded);
                }
                let mut result = Self::new(vec![Rational::from(Integer::one())]);
                let mut factor = base_poly;
                let mut remaining = power;
                while remaining != 0 {
                    if remaining & 1 != 0 {
                        result = result.multiply(&factor);
                    }
                    remaining >>= 1;
                    if remaining != 0 {
                        factor = factor.multiply(&factor);
                    }
                }
                Ok(result)
            }
        }
    }

    /// Converts a polynomial to a canonical Expr using the supplied symbol.
    ///
    /// This is a semantic conversion, not a structural Expr round trip:
    /// constructor normalization can change operand shape.
    pub fn to_expr(&self, variable: &Symbol) -> Expr {
        let terms = self.coefficients().iter().enumerate().filter_map(|(power, coefficient)| {
            if coefficient.numerator().is_zero() {
                return None;
            }
            let scalar = Expr::from(coefficient.clone());
            if power == 0 {
                return Some(scalar);
            }
            let symbol = Expr::from(variable.clone());
            let variable_power = if power == 1 {
                symbol
            } else {
                Expr::pow(symbol, Expr::from(power))
            };
            Some(Expr::mul([scalar, variable_power]))
        });
        Expr::add(terms)
    }
}
