use mathema::{Integer, Rational};
use mathematishia_kernel::RationalPolynomial;

fn q(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn p(terms: &[(i64, i64)]) -> RationalPolynomial {
    RationalPolynomial::new(
        terms
            .iter()
            .map(|&(numerator, denominator)| q(numerator, denominator))
            .collect(),
    )
}

#[test]
fn canonicalizes_trailing_zeros_and_zero_polynomials() {
    let zero = RationalPolynomial::zero();
    assert_eq!(p(&[]), zero);
    assert_eq!(p(&[(0, 1), (0, 1), (0, 1)]), zero);
    assert_eq!(zero.degree(), None);
    assert!(zero.coefficients().is_empty());
    assert!(zero.is_zero());

    let polynomial = p(&[(3, 1), (0, 1), (2, 1), (0, 1), (0, 1)]);
    assert_eq!(polynomial.degree(), Some(2));
    assert_eq!(
        polynomial.coefficients(),
        &[q(3, 1), q(0, 1), q(2, 1)]
    );
    assert!(!polynomial.is_zero());
    assert_eq!(p(&[(5, 2), (0, 1)]), p(&[(5, 2)]));
}

#[test]
fn canonicalizes_rational_coefficients_and_preserves_internal_zeros() {
    let polynomial = p(&[(2, 4), (0, 1), (-6, -9)]);
    assert_eq!(
        polynomial.coefficients(),
        &[q(1, 2), q(0, 1), q(2, 3)]
    );
    assert_eq!(polynomial.degree(), Some(2));
}

#[test]
fn adds_and_subtracts_polynomials_without_losing_degree_information() {
    let a = p(&[(1, 2), (2, 1), (3, 4)]);
    let b = p(&[(-1, 2), (3, 1), (-3, 4)]);
    assert_eq!(a.add(&b), p(&[(0, 1), (5, 1)]));
    assert_eq!(a.subtract(&a), RationalPolynomial::zero());
    assert_eq!(a.subtract(&b), p(&[(1, 1), (-1, 1), (3, 2)]));
    assert_eq!(a.add(&RationalPolynomial::zero()), a);
    assert_eq!(
        RationalPolynomial::zero().subtract(&a),
        p(&[(-1, 2), (-2, 1), (-3, 4)])
    );
}

#[test]
fn multiplies_exactly_and_handles_zero_and_constants() {
    let x_plus_one = p(&[(1, 1), (1, 1)]);
    let x_minus_one = p(&[(-1, 1), (1, 1)]);
    assert_eq!(
        x_plus_one.multiply(&x_minus_one),
        p(&[(-1, 1), (0, 1), (1, 1)])
    );
    assert_eq!(
        x_minus_one.multiply(&x_plus_one),
        x_plus_one.multiply(&x_minus_one)
    );

    let zero = RationalPolynomial::zero();
    assert_eq!(x_plus_one.multiply(&zero), zero);
    assert_eq!(zero.multiply(&x_plus_one), zero);
    assert_eq!(x_plus_one.multiply(&p(&[(1, 1)])), x_plus_one);

    let product = p(&[(1, 2), (1, 3)]).multiply(&p(&[(3, 4), (2, 5)]));
    assert_eq!(product, p(&[(3, 8), (9, 20), (2, 15)]));
}

#[test]
fn preserves_sparse_holes_during_multiplication() {
    let x_squared = p(&[(0, 1), (0, 1), (1, 1)]);
    let shifted = x_squared.multiply(&p(&[(1, 1), (0, 1), (1, 1)]));
    assert_eq!(
        shifted,
        p(&[(0, 1), (0, 1), (1, 1), (0, 1), (1, 1)])
    );
}

#[test]
fn evaluates_using_exact_horner_arithmetic() {
    let polynomial = p(&[(1, 3), (1, 2), (3, 4)]);
    assert_eq!(polynomial.evaluate(&q(2, 3)), q(1, 1));
    assert_eq!(polynomial.evaluate(&q(0, 1)), q(1, 3));
    assert_eq!(polynomial.evaluate(&q(-1, 1)), q(7, 12));
    assert_eq!(RationalPolynomial::zero().evaluate(&q(7, 3)), q(0, 1));
}

#[test]
fn arithmetic_obeys_evaluation_homomorphism_for_sampled_exact_values() {
    let polynomials = [
        RationalPolynomial::zero(),
        p(&[(1, 2)]),
        p(&[(0, 1), (-1, 3), (2, 1)]),
        p(&[(2, 1), (0, 1), (1, 4)]),
    ];
    let points = [q(-2, 1), q(0, 1), q(1, 2), q(3, 1)];

    for a in &polynomials {
        for b in &polynomials {
            for x in &points {
                let a_value = a.evaluate(x);
                let b_value = b.evaluate(x);
                assert_eq!(a.add(b).evaluate(x), &a_value + &b_value);
                assert_eq!(a.subtract(b).evaluate(x), &a_value - &b_value);
                assert_eq!(a.multiply(b).evaluate(x), &a_value * &b_value);
            }
        }
    }
}

#[test]
fn addition_and_multiplication_are_associative_on_small_examples() {
    let a = p(&[(2, 3), (1, 2)]);
    let b = p(&[(3, 2), (-1, 3), (1, 1)]);
    let c = p(&[(-1, 4), (0, 1), (2, 1)]);

    assert_eq!(a.add(&b).add(&c), a.add(&b.add(&c)));
    assert_eq!(
        a.multiply(&b).multiply(&c),
        a.multiply(&b.multiply(&c))
    );
    assert_eq!(
        a.multiply(&b.add(&c)),
        a.multiply(&b).add(&a.multiply(&c))
    );
}

#[test]
fn supports_coefficients_beyond_fixed_width_integers() {
    let large = Rational::from(Integer::from(u128::MAX));
    let polynomial = RationalPolynomial::new(vec![large.clone(), large.clone()]);
    let result = polynomial.evaluate(&q(1, 1));
    assert_eq!(result, &large + &large);
    assert_eq!(polynomial.multiply(&polynomial).degree(), Some(2));
}

#[test]
fn arithmetic_does_not_mutate_operands() {
    let a = p(&[(1, 1), (2, 1)]);
    let b = p(&[(3, 4), (0, 1), (2, 3)]);
    let a_snapshot = a.clone();
    let b_snapshot = b.clone();
    let _ = a.add(&b);
    let _ = a.subtract(&b);
    let _ = a.multiply(&b);
    let _ = a.evaluate(&q(5, 2));
    assert_eq!(a, a_snapshot);
    assert_eq!(b, b_snapshot);
}
