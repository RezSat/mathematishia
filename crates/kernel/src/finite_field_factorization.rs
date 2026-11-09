//! Deterministic bounded factorization over small finite prime fields.
//!
//! Monic divisors are enumerated in degree order. Exhaustive enumeration
//! proves irreducibility; a capped search returns an unresolved cofactor.

use crate::{FiniteFieldError, FiniteFieldPolynomial};

const MAX_FACTOR_DEGREE: usize = 8;
const MAX_CANDIDATES: usize = 20_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteFieldFactor {
    polynomial: FiniteFieldPolynomial,
    multiplicity: usize,
}

impl FiniteFieldFactor {
    pub fn polynomial(&self) -> &FiniteFieldPolynomial {
        &self.polynomial
    }

    pub fn multiplicity(&self) -> usize {
        self.multiplicity
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteFieldFactorization {
    prime: u32,
    unit: u32,
    factors: Vec<FiniteFieldFactor>,
    unresolved: Vec<FiniteFieldFactor>,
}

impl FiniteFieldFactorization {
    pub fn prime(&self) -> u32 { self.prime }
    pub fn unit(&self) -> u32 { self.unit }
    pub fn factors(&self) -> &[FiniteFieldFactor] { &self.factors }
    pub fn unresolved(&self) -> &[FiniteFieldFactor] { &self.unresolved }
    pub fn is_complete(&self) -> bool { self.unresolved.is_empty() }
}

fn one(prime: u32) -> FiniteFieldPolynomial {
    FiniteFieldPolynomial::new(prime, vec![1]).expect("validated prime")
}

fn record(
    target: &mut Vec<FiniteFieldFactor>,
    polynomial: FiniteFieldPolynomial,
) {
    if let Some(existing) = target.iter_mut().find(|factor| factor.polynomial == polynomial) {
        existing.multiplicity += 1;
    } else {
        target.push(FiniteFieldFactor { polynomial, multiplicity: 1 });
    }
}

fn factor_recursive(
    polynomial: FiniteFieldPolynomial,
    result: &mut FiniteFieldFactorization,
) -> Result<(), FiniteFieldError> {
    let Some(degree) = polynomial.degree() else {
        return Ok(());
    };
    if degree == 0 {
        return Ok(());
    }
    if degree == 1 || polynomial.is_irreducible()? {
        record(&mut result.factors, polynomial);
        return Ok(());
    }
    if degree > MAX_FACTOR_DEGREE {
        record(&mut result.unresolved, polynomial);
        return Ok(());
    }
    let prime = polynomial.prime();
    let mut attempts = 0usize;
    for divisor_degree in 1..=degree / 2 {
        let Some(candidate_count) = usize::try_from(prime)
            .ok()
            .and_then(|p| p.checked_pow(divisor_degree as u32))
        else {
            record(&mut result.unresolved, polynomial);
            return Ok(());
        };
        if attempts.saturating_add(candidate_count) > MAX_CANDIDATES {
            record(&mut result.unresolved, polynomial);
            return Ok(());
        }
        for index in 0..candidate_count {
            attempts += 1;
            let mut number = index;
            let mut coefficients = Vec::with_capacity(divisor_degree + 1);
            for _ in 0..divisor_degree {
                coefficients.push((number % prime as usize) as i64);
                number /= prime as usize;
            }
            coefficients.push(1);
            let candidate = FiniteFieldPolynomial::new(prime, coefficients)?;
            let (quotient, remainder) = polynomial.divide_with_remainder(&candidate)?;
            if remainder.is_zero() {
                factor_recursive(candidate, result)?;
                factor_recursive(quotient, result)?;
                return Ok(());
            }
        }
    }
    // This is the mathematically exhaustive case. The independent Rabin
    // test must agree before an irreducibility claim is recorded.
    if polynomial.is_irreducible()? {
        record(&mut result.factors, polynomial);
    } else {
        record(&mut result.unresolved, polynomial);
    }
    Ok(())
}

/// Independent check of the product, factor labels and every claimed
/// irreducibility; unresolved factors are checked but not certified.
pub fn verify_finite_field_factorization(
    original: &FiniteFieldPolynomial,
    result: &FiniteFieldFactorization,
) -> Result<bool, FiniteFieldError> {
    if original.is_zero() || original.prime() != result.prime
        || original.coefficients().last() != Some(&result.unit)
        || result.unit == 0
    {
        return Ok(false);
    }
    let mut product = one(original.prime()).scale(result.unit);
    let mut seen = Vec::new();
    for factor in result.factors.iter().chain(result.unresolved.iter()) {
        if factor.multiplicity == 0 || factor.polynomial.prime() != original.prime()
            || factor.polynomial.degree().is_none_or(|degree| degree == 0)
            || factor.polynomial.coefficients().last() != Some(&1)
            || seen.contains(&factor.polynomial)
        {
            return Ok(false);
        }
        seen.push(&factor.polynomial);
        for _ in 0..factor.multiplicity {
            product = product.multiply(&factor.polynomial)?;
        }
    }
    if product != *original {
        return Ok(false);
    }
    for factor in &result.factors {
        if !factor.polynomial.is_irreducible()? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Attempts a complete exact factorization. The current enumerator is
/// exhaustive for admitted small cases, and explicitly partial otherwise.
pub fn factor_finite_field(
    polynomial: &FiniteFieldPolynomial,
) -> Result<FiniteFieldFactorization, FiniteFieldError> {
    if polynomial.is_zero() {
        return Err(FiniteFieldError::ZeroDivisor);
    }
    let unit = *polynomial.coefficients().last().expect("nonzero polynomial");
    let mut result = FiniteFieldFactorization {
        prime: polynomial.prime(),
        unit,
        factors: Vec::new(),
        unresolved: Vec::new(),
    };
    factor_recursive(polynomial.monic(), &mut result)?;
    let order = |factor: &FiniteFieldFactor| {
        (factor.polynomial.degree().unwrap_or(0),factor.polynomial.coefficients().to_vec())
    };
    result.factors.sort_by_key(&order);
    result.unresolved.sort_by_key(order);
    if !verify_finite_field_factorization(polynomial, &result)? {
        return Err(FiniteFieldError::DegreeLimit);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(prime: u32, coefficients: &[i64]) -> FiniteFieldPolynomial {
        FiniteFieldPolynomial::new(prime, coefficients.to_vec()).unwrap()
    }

    #[test]
    fn factors_repeated_and_irreducible_components() {
        let one_plus_x = p(2, &[1,1]);
        let irreducible = p(2, &[1,1,1]);
        let f = one_plus_x.multiply(&one_plus_x).unwrap().multiply(&irreducible).unwrap();
        let result = factor_finite_field(&f).unwrap();
        assert!(result.is_complete());
        assert!(verify_finite_field_factorization(&f, &result).unwrap());
        assert_eq!(result.factors().len(), 2);
        assert!(result.factors().iter().any(|factor|
            factor.polynomial() == &one_plus_x && factor.multiplicity() == 2));
        assert!(result.factors().iter().any(|factor|
            factor.polynomial() == &irreducible && factor.multiplicity() == 1));
    }

    #[test]
    fn factors_quartic_without_linear_roots() {
        let f=p(3, &[1,0,0,0,1]);
        let result=factor_finite_field(&f).unwrap();
        assert!(result.is_complete());
        assert_eq!(result.factors().len(),2);
        assert!(result.factors().iter().all(|x|x.polynomial().degree()==Some(2)));
        assert!(verify_finite_field_factorization(&f,&result).unwrap());
    }

    #[test]
    fn rejects_modified_factor_evidence() {
        let f=p(3,&[1,0,0,0,1]);
        let mut result=factor_finite_field(&f).unwrap();
        result.factors.pop();
        assert!(!verify_finite_field_factorization(&f,&result).unwrap());
    }

    #[test]
    fn grid_reconstructs_every_degree_four_binary_polynomial() {
        for bits in 0..16 {
            let mut coefficients=(0..4).map(|k|((bits>>k)&1) as i64).collect::<Vec<_>>();
            coefficients.push(1);
            let f=p(2,&coefficients);
            let result=factor_finite_field(&f).unwrap();
            assert!(result.is_complete());
            assert!(verify_finite_field_factorization(&f,&result).unwrap());
        }
    }
}
