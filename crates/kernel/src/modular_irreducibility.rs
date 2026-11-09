//! Modular irreducibility certificates for rational polynomials.
//!
//! A primitive integer polynomial of unchanged degree that remains
//! irreducible modulo a prime is irreducible over Q (Gauss's lemma).

use mathema::{Integer, Rational};

use crate::{FiniteFieldError, FiniteFieldPolynomial, RationalPolynomial};

const SEARCH_PRIMES: [u32; 15] = [
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47,
];

/// Exact primitive integer coefficients after clearing rational denominators.
fn primitive(poly: &RationalPolynomial) -> Option<Vec<i64>> {
    let lcm = poly.coefficients()
        .iter()
        .fold(Integer::one(), |product, c| product.lcm(c.denominator()));
    let integer = poly.coefficients().iter()
        .map(|c| {
            let multiple = lcm.divide_exact(c.denominator())?;
            Some(&multiple * c.numerator())
        })
        .collect::<Option<Vec<_>>>()?;
    let gcd = integer.iter().fold(Integer::zero(), |g, x| g.gcd(x));
    if gcd.is_zero() {
        return None;
    }
    integer.iter().map(|x| x.divide_exact(&gcd)?.try_i64()).collect()
}

fn reduction(poly: &RationalPolynomial, prime: u32) -> Option<FiniteFieldPolynomial> {
    let integers = primitive(poly)?;
    let reduced = FiniteFieldPolynomial::new(prime, integers).ok()?;
    (reduced.degree() == poly.degree()).then_some(reduced)
}

/// A checkable modular irreducibility witness. The record is not a full
/// factorization of the polynomial over F_p.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModularIrreducibilityCertificate {
    prime: u32,
    reduced: FiniteFieldPolynomial,
}

impl ModularIrreducibilityCertificate {
    pub fn prime(&self) -> u32 { self.prime }
    pub fn reduced(&self) -> &FiniteFieldPolynomial { &self.reduced }
}

/// Verifies the primitive polynomial, the degree-preserving reduction, and
/// Rabin's irreducibility criterion independently of the prime search.
pub fn verify_modular_irreducibility(
    polynomial: &RationalPolynomial,
    certificate: &ModularIrreducibilityCertificate,
) -> bool {
    if polynomial.degree().is_none_or(|d| d < 2) {
        return false;
    }
    let Some(expected) = reduction(polynomial, certificate.prime) else {
        return false;
    };
    expected == certificate.reduced
        && expected.is_irreducible() == Ok(true)
}

/// Searches a fixed list of small primes for a mathematically sufficient
/// certificate. None means unproven, not reducible.
pub fn modular_irreducibility_witness(
    polynomial: &RationalPolynomial,
) -> Option<ModularIrreducibilityCertificate> {
    if polynomial.degree().is_none_or(|d| d < 2) {
        return None;
    }
    SEARCH_PRIMES.iter().copied().find_map(|prime| {
        let reduced = reduction(polynomial, prime)?;
        if reduced.is_irreducible().ok()? {
            Some(ModularIrreducibilityCertificate { prime, reduced })
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(values: &[i64]) -> RationalPolynomial {
        RationalPolynomial::new(values.iter()
            .map(|&n| Rational::from(Integer::from(n))).collect())
    }

    #[test]
    fn quartic_can_be_certified_modulo_two() {
        let quartic = p(&[1,1,0,0,1]);
        let witness = modular_irreducibility_witness(&quartic).unwrap();
        assert_eq!(witness.prime, 2);
        assert!(verify_modular_irreducibility(&quartic, &witness));
        let mut forged = witness.clone();
        forged.prime = 3;
        assert!(!verify_modular_irreducibility(&quartic, &forged));
        assert!(!verify_modular_irreducibility(&p(&[1,0,0,0,1]), &witness));
    }

    #[test]
    fn reducible_polynomial_has_no_valid_modular_witness() {
        let f = p(&[1,0,2,0,1]);
        assert!(modular_irreducibility_witness(&f).is_none());
    }

    #[test]
    fn large_or_zero_leading_reduction_is_not_a_proof() {
        let f = p(&[1,1,0,0,2]);
        let witness = modular_irreducibility_witness(&f);
        if let Some(value) = witness {
            assert!(verify_modular_irreducibility(&f, &value));
        }
        assert!(modular_irreducibility_witness(&p(&[1])).is_none());
    }
}
