use mathema::{Integer, Rational, Symbol};
use mathematishia_kernel::{
    MultivariateDivisionError, MultivariateLimits, MultivariatePolynomial, divide_multivariate,
    verify_multivariate_division,
};

fn q(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}
fn s(v: &str) -> Symbol {
    Symbol::new(v).unwrap()
}
fn p(terms: &[(i32, i32, i64)]) -> MultivariatePolynomial {
    MultivariatePolynomial::try_new(
        vec![s("x"), s("y")],
        terms.iter().map(|&(a, b, c)| (vec![a, b], q(c))),
        MultivariateLimits::default(),
    )
    .unwrap()
}

#[test]
fn exact_division_and_nontrivial_remainder() {
    let limits = MultivariateLimits::default();
    let dividend = p(&[(2, 0, 1), (0, 1, 1)]);
    let divisor = p(&[(1, 0, 1), (0, 0, 1)]);
    let result = divide_multivariate(&dividend, std::slice::from_ref(&divisor), limits).unwrap();
    assert_eq!(result.quotients(), &[p(&[(1, 0, 1), (0, 0, -1)])]);
    assert_eq!(result.remainder(), &p(&[(0, 1, 1), (0, 0, 1)]));
    assert!(verify_multivariate_division(&dividend, &[divisor], &result, limits).unwrap());

    let divisible = p(&[(2, 0, 1), (1, 1, 1), (1, 0, 1), (0, 1, 1)]);
    let divisor = p(&[(1, 0, 1), (0, 1, 1)]);
    let result = divide_multivariate(&divisible, std::slice::from_ref(&divisor), limits).unwrap();
    assert_eq!(result.quotients(), &[p(&[(1, 0, 1), (0, 0, 1)])]);
    assert!(result.remainder().is_zero());
    assert!(verify_multivariate_division(&divisible, &[divisor], &result, limits).unwrap());
}

#[test]
fn divisor_order_is_explicit_and_result_is_deterministic() {
    let limits = MultivariateLimits::default();
    let dividend = p(&[(1, 1, 1)]);
    let x = p(&[(1, 0, 1)]);
    let y = p(&[(0, 1, 1)]);
    let first = divide_multivariate(&dividend, &[x.clone(), y.clone()], limits).unwrap();
    let second = divide_multivariate(&dividend, &[y.clone(), x.clone()], limits).unwrap();
    assert_eq!(first.quotients(), &[y.clone(), p(&[])]);
    assert_eq!(second.quotients(), &[x.clone(), p(&[])]);
    assert!(first.remainder().is_zero() && second.remainder().is_zero());
    assert!(
        verify_multivariate_division(&dividend, &[x.clone(), y.clone()], &first, limits).unwrap()
    );
    assert!(verify_multivariate_division(&dividend, &[y, x], &second, limits).unwrap());
}

#[test]
fn rejects_empty_zero_and_incompatible_divisors() {
    let limits = MultivariateLimits::default();
    let f = p(&[(1, 0, 1)]);
    assert_eq!(
        divide_multivariate(&f, &[], limits),
        Err(MultivariateDivisionError::NoDivisors)
    );
    assert_eq!(
        divide_multivariate(&f, &[p(&[])], limits),
        Err(MultivariateDivisionError::ZeroDivisor)
    );
    let other = MultivariatePolynomial::constant(vec![s("z")], q(1), limits).unwrap();
    assert_eq!(
        divide_multivariate(&f, &[other], limits),
        Err(MultivariateDivisionError::Polynomial(
            mathematishia_kernel::MultivariateError::VariableMismatch,
        ))
    );
}

#[test]
fn seeded_exact_recomposition_across_small_polynomials() {
    let limits = MultivariateLimits::default();
    let divisors = [p(&[(1, 0, 1), (0, 1, 1)]), p(&[(0, 1, 1), (0, 0, 1)])];
    let mut seed = 57_u64;
    for _ in 0..120 {
        let mut terms = Vec::new();
        for a in 0..=3 {
            for b in 0..=2 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let c = ((seed >> 33) % 9) as i64 - 4;
                if c != 0 {
                    terms.push((a, b, c));
                }
            }
        }
        let f = p(&terms);
        let result = divide_multivariate(&f, &divisors, limits).unwrap();
        assert!(verify_multivariate_division(&f, &divisors, &result, limits).unwrap());
        assert_eq!(result, divide_multivariate(&f, &divisors, limits).unwrap());
    }
}

#[test]
fn zero_polynomial_remains_in_ambient_ring() {
    let limits = MultivariateLimits::default();
    let zero = p(&[]);
    let divisor = p(&[(1, 0, 1)]);
    let result = divide_multivariate(&zero, std::slice::from_ref(&divisor), limits).unwrap();
    assert_eq!(result.remainder(), &zero);
    assert_eq!(result.quotients(), std::slice::from_ref(&zero));
    assert_eq!(result.steps(), 0);
    assert!(verify_multivariate_division(&zero, &[divisor], &result, limits).unwrap());
}
