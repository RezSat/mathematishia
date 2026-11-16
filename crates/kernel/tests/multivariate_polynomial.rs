use mathema::{Integer, Rational, Symbol};
use mathematishia_kernel::{MultivariateError as E, MultivariateLimits as L, MultivariatePolynomial as P};

fn s(value: &str) -> Symbol {
    Symbol::new(value).unwrap()
}
fn q(n: i64) -> Rational {
    Rational::from(Integer::from(n))
}
fn fraction(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
fn p(variables: &[&str], terms: &[(Vec<i32>, i64)]) -> P {
    P::try_new(
        variables.iter().map(|name| s(name)).collect(),
        terms.iter().map(|(powers, value)| (powers.clone(), q(*value))),
        L::default(),
    )
    .unwrap()
}

#[test]
fn canonical_zero_collection_cancellation_and_input_permutations() {
    let variables = vec![s("y"), s("x")];
    let input = vec![
        (vec![1, 0], q(2)),
        (vec![0, 1], q(-7)),
        (vec![1, 0], q(-2)),
        (vec![0, 1], q(7)),
    ];
    let zero = P::try_new(variables.clone(), input.clone(), L::default()).unwrap();
    assert!(zero.is_zero());
    assert_eq!(zero.total_degree(), None);
    assert_eq!(zero.degree_in(&s("x")), Ok(None));
    assert_eq!(zero.to_string(), "0");
    let mut reverse = input;
    reverse.reverse();
    assert_eq!(
        P::try_new(variables, reverse, L::default()).unwrap(),
        zero
    );
    assert_eq!(zero, P::zero(vec![s("x"), s("y")], L::default()).unwrap());
    assert_ne!(zero, P::zero(vec![], L::default()).unwrap());
}

#[test]
fn variable_order_reindexes_powers_and_rendering_is_reproducible() {
    let a = p(&["y", "x"], &[(vec![1, 2], 3), (vec![0, 0], -2)]);
    let b = p(&["x", "y"], &[(vec![2, 1], 3), (vec![0, 0], -2)]);
    assert_eq!(a, b);
    assert_eq!(a.variables(), &[s("x"), s("y")]);
    assert_eq!(a.total_degree(), Some(3));
    assert_eq!(a.degree_in(&s("x")), Ok(Some(2)));
    assert_eq!(a.degree_in(&s("y")), Ok(Some(1)));
    assert_eq!(a.coefficient(&[2, 1]), Ok(q(3)));
    assert_eq!(a.coefficient(&[1, 2]), Ok(q(0)));
    assert_eq!(a.to_string(), "-2 + 3*(\"x\")^2*(\"y\")");
    assert_eq!(a.to_string(), b.to_string());
}

#[test]
fn exact_arithmetic_differentiation_and_independent_substitution() {
    let x = p(&["x", "y"], &[(vec![1, 0], 1)]);
    let y = p(&["x", "y"], &[(vec![0, 1], 1)]);
    let one = P::constant(vec![s("y"), s("x")], q(1), L::default()).unwrap();
    let a = x.add(&one, L::default()).unwrap();
    let b = y.subtract(&one, L::default()).unwrap();
    let prod = a.multiply(&b, L::default()).unwrap();
    assert_eq!(prod.differentiate(&s("x"), L::default()).unwrap(), b);
    assert_eq!(prod.differentiate(&s("y"), L::default()).unwrap(), a);
    assert_eq!(prod.evaluate(&[q(2), q(4)], L::default()).unwrap(), q(9));
    let at_x = prod.substitute(&s("x"), &q(2), L::default()).unwrap();
    assert_eq!(at_x.variables(), &[s("y")]);
    assert_eq!(at_x.evaluate(&[q(4)], L::default()).unwrap(), q(9));
    let at_y = at_x.substitute(&s("y"), &q(4), L::default()).unwrap();
    assert!(at_y.variables().is_empty());
    assert_eq!(at_y.coefficient(&[]), Ok(q(9)));
    assert_eq!(
        prod.differentiate(&s("z"), L::default()),
        Err(E::UnknownVariable)
    );
}

#[test]
fn exact_fraction_coefficients_and_derivative() {
    let x = P::try_new(
        vec![s("x")],
        [(vec![2], fraction(2, 3)), (vec![0], fraction(-1, 6))],
        L::default(),
    )
    .unwrap();
    let derived = x.differentiate(&s("x"), L::default()).unwrap();
    assert_eq!(derived.coefficient(&[1]), Ok(fraction(4, 3)));
    assert_eq!(x.evaluate(&[fraction(1, 2)], L::default()), Ok(q(0)));
}

#[test]
fn invalid_inputs_and_distinct_ambient_rings() {
    assert_eq!(
        P::try_new(vec![s("x"), s("x")], [], L::default()),
        Err(E::DuplicateVariable)
    );
    assert_eq!(
        P::try_new(vec![s("x")], [(vec![-1], q(1))], L::default()),
        Err(E::NegativeExponent)
    );
    assert_eq!(
        P::try_new(vec![s("x")], [(vec![1, 0], q(1))], L::default()),
        Err(E::ExponentArity)
    );
    let a = p(&["x"], &[(vec![0], 1)]);
    let b = p(&["y"], &[(vec![0], 1)]);
    assert_eq!(a.add(&b, L::default()), Err(E::VariableMismatch));
    assert_eq!(a.multiply(&b, L::default()), Err(E::VariableMismatch));
    assert_eq!(a.evaluate(&[], L::default()), Err(E::ValueArity));
    assert_eq!(a.coefficient(&[]), Err(E::ExponentArity));
}

#[test]
fn resource_limits_reject_inputs_and_intermediates() {
    let tight = L {
        max_variables: 1,
        max_terms: 2,
        max_input_terms: 3,
        max_total_degree: 2,
        max_scalar_bits: 8,
        max_work_units: 100,
    };
    assert_eq!(
        P::zero(vec![s("x"), s("y")], tight),
        Err(E::VariableLimit)
    );
    assert_eq!(
        P::try_new(vec![s("x")], [(vec![3], q(1))], tight),
        Err(E::TotalDegreeLimit)
    );
    assert_eq!(
        P::try_new(vec![s("x")], [(vec![0], q(256))], tight),
        Err(E::ScalarBitLimit)
    );
    assert_eq!(
        P::try_new(
            vec![s("x")],
            [(vec![0], q(1)), (vec![1], q(1)), (vec![2], q(1))],
            tight
        ),
        Err(E::TermLimit)
    );
    assert_eq!(
        P::try_new(
            vec![s("x")],
            (0..4).map(|_| (vec![0], q(1))),
            tight
        ),
        Err(E::InputTermLimit)
    );
    let x = p(&["x"], &[(vec![1], 1)]);
    assert_eq!(x.multiply(&x.multiply(&x, L::default()).unwrap(), tight),
        Err(E::TotalDegreeLimit));
    let one = p(&["x"], &[(vec![0], 1)]);
    assert_eq!(
        one.add(&one, L { max_work_units: 0, ..tight }),
        Err(E::WorkLimit)
    );
    assert_eq!(
        one.add(&one, L { max_scalar_bits: 1, ..tight }),
        Err(E::ScalarBitLimit)
    );
}

fn sample(seed: &mut u64) -> P {
    let mut entries = Vec::new();
    for _ in 0..8 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        let powers = vec![(*seed % 3) as i32, ((*seed >> 4) % 3) as i32];
        let coefficient = ((*seed >> 10) % 9) as i64 - 4;
        entries.push((powers, coefficient));
    }
    p(&["y", "x"], &entries)
}

#[test]
fn deterministic_grid_distributivity_and_permutation_invariance() {
    let mut seed = 0x87fa_aad3_19e4_22b1_u64;
    for _ in 0..80 {
        let a = sample(&mut seed);
        let b = sample(&mut seed);
        let c = sample(&mut seed);
        let left = a.multiply(&b.add(&c, L::default()).unwrap(), L::default()).unwrap();
        let right = a
            .multiply(&b, L::default())
            .unwrap()
            .add(&a.multiply(&c, L::default()).unwrap(), L::default())
            .unwrap();
        assert_eq!(left, right);
        for x in -2..=2 {
            for y in -2..=2 {
                let point = [q(x), q(y)];
                let av = a.evaluate(&point, L::default()).unwrap();
                let bv = b.evaluate(&point, L::default()).unwrap();
                let cv = c.evaluate(&point, L::default()).unwrap();
                let independent = &av * &(&bv + &cv);
                assert_eq!(left.evaluate(&point, L::default()).unwrap(), independent);
            }
        }
        let reversed_terms = a
            .terms()
            .iter()
            .rev()
            .map(|(exponents, value)| {
                (exponents.iter().map(|power| *power as i32).collect(), value.clone())
            });
        assert_eq!(
            P::try_new(a.variables().to_vec(), reversed_terms, L::default()).unwrap(),
            a
        );
    }
}
