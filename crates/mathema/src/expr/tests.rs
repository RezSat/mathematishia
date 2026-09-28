use std::collections::{HashSet, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed};

use super::*;
use crate::EmptySymbolName;

fn symbol(name: &str) -> Expr {
    Symbol::new(name).unwrap().into()
}

fn rational(n: i64, d: i64) -> Expr {
    Rational::new(n.into(), d.into()).unwrap().into()
}

fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn symbol_names_are_exact_and_nonempty() {
    assert_eq!(Symbol::new(""), Err(EmptySymbolName));
    let error: &dyn std::error::Error = &EmptySymbolName;
    assert_eq!(error.to_string(), "a symbol name must be non-empty");
    for name in ["x", "α", "a + b", " ", "é", "e\u{301}", "\0"] {
        let value = Symbol::new(name).unwrap();
        assert_eq!(value.name(), name);
        assert_eq!(value.to_string(), name);
        assert_eq!(value.clone(), value);
        assert_eq!(hash(&value), hash(&Symbol::new(name.to_owned()).unwrap()));
    }
    let composed = Symbol::new("é").unwrap();
    let decomposed = Symbol::new("e\u{301}").unwrap();
    assert_ne!(composed, decomposed);
    assert_eq!(HashSet::from([composed, decomposed]).len(), 2);
    let mut names: Vec<_> = ["z", "α", "a", "A"].map(|n| Symbol::new(n).unwrap()).into();
    names.sort();
    assert_eq!(
        names.iter().map(Symbol::name).collect::<Vec<_>>(),
        ["A", "a", "z", "α"]
    );
}

#[test]
fn scalar_lifting_is_canonical() {
    assert!(matches!(
        Expr::from(Integer::from(2)).0.as_ref(),
        ExprNode::Integer(_)
    ));
    assert!(matches!(rational(1, 2).0.as_ref(), ExprNode::Rational(_)));
    assert_eq!(rational(6, 3), Expr::from(2));
    assert_eq!(rational(0, -7), Expr::from(0));
    assert_eq!(hash(&rational(6, 3)), hash(&Expr::from(2)));
    for value in [
        Expr::from(2i8),
        Expr::from(2i16),
        Expr::from(2i32),
        Expr::from(2i64),
        Expr::from(2i128),
        Expr::from(2isize),
        Expr::from(2u8),
        Expr::from(2u16),
        Expr::from(2u32),
        Expr::from(2u64),
        Expr::from(2u128),
        Expr::from(2usize),
    ] {
        assert_eq!(value, rational(2, 1));
    }
    assert_eq!(Expr::from(i128::MIN).to_string(), i128::MIN.to_string());
    assert_eq!(Expr::from(u128::MAX).to_string(), u128::MAX.to_string());
}

#[test]
fn equality_and_hashing_are_structural() {
    let x = symbol("x");
    let another_x = symbol("x");
    assert!(!Arc::ptr_eq(&x.0, &another_x.0));
    assert_eq!(x, another_x);
    assert_eq!(hash(&x), hash(&another_x));
    assert_ne!(x, symbol("y"));
    let shared = x.clone();
    assert!(Arc::ptr_eq(&x.0, &shared.0));
    for expr in [
        Expr::add([x.clone(), 2.into()]),
        Expr::mul([x.clone(), rational(3, 2)]),
        Expr::pow(x.clone(), 2.into()),
    ] {
        assert_eq!(hash(&expr), hash(&expr.clone()));
        assert_eq!(expr, expr.clone());
    }
    let a = Expr::pow(Expr::add([x.clone(), 2.into()]), symbol("y"));
    let b = Expr::pow(Expr::add([2.into(), symbol("x")]), symbol("y"));
    assert_eq!(a, b);
    assert_eq!(hash(&a), hash(&b));
    assert_ne!(a, Expr::pow(Expr::add([x, 2.into()]), symbol("z")));
}

#[test]
fn sums_flatten_fold_and_handle_identities() {
    let (x, y, z) = (symbol("x"), symbol("y"), symbol("z"));
    assert_eq!(Expr::add([]), 0.into());
    assert_eq!(Expr::add([x.clone()]), x);
    assert_eq!(Expr::add([x.clone(), 0.into()]), x);
    assert_eq!(
        Expr::add([1.into(), rational(1, 2), x.clone()]),
        Expr::add([rational(3, 2), x.clone()])
    );
    assert_eq!(Expr::add([rational(1, 2), rational(3, 2)]), 2.into());
    assert_eq!(Expr::add([rational(-1, 2), rational(1, 2), x.clone()]), x);
    let nested = Expr::add([
        x.clone(),
        Expr::add([y.clone(), Expr::add([z.clone(), 0.into()])]),
    ]);
    let flat = Expr::add([z, x, y]);
    assert_eq!(nested, flat);
    assert_eq!(hash(&nested), hash(&flat));
    assert_invariants(&nested);
}

#[test]
fn products_flatten_fold_and_handle_identities() {
    let (x, y, z) = (symbol("x"), symbol("y"), symbol("z"));
    assert_eq!(Expr::mul([]), 1.into());
    assert_eq!(Expr::mul([x.clone()]), x);
    assert_eq!(Expr::mul([x.clone(), 1.into()]), x);
    assert_eq!(
        Expr::mul([2.into(), rational(3, 4), x.clone()]),
        Expr::mul([rational(3, 2), x.clone()])
    );
    assert_eq!(Expr::mul([2.into(), rational(1, 2), x.clone()]), x);
    assert_eq!(Expr::mul([4.into(), rational(1, 2)]), 2.into());
    let nested = Expr::mul([
        x.clone(),
        Expr::mul([y.clone(), Expr::mul([z.clone(), 1.into()])]),
    ]);
    let flat = Expr::mul([z, x, y]);
    assert_eq!(nested, flat);
    assert_eq!(hash(&nested), hash(&flat));
    assert_invariants(&nested);
    let power = Expr::pow(0.into(), 0.into());
    assert_eq!(
        Expr::mul([power.clone(), nested.clone(), 0.into()]),
        0.into()
    );
    assert_eq!(Expr::mul([0.into(), nested, power]), 0.into());
}

#[test]
fn scalar_folding_exceeds_primitive_widths() {
    let large = Integer::from(u128::MAX) + Integer::one();
    assert_eq!(
        Expr::add([large.clone().into(), (-&large).into()]),
        0.into()
    );
    assert_eq!(
        Expr::mul([large.clone().into(), large.clone().into()]),
        Expr::from(&large * &large)
    );
    let fraction = Rational::new(large.clone(), 3.into()).unwrap();
    assert_eq!(
        Expr::add([
            fraction.clone().into(),
            fraction.clone().into(),
            fraction.into()
        ]),
        Expr::from(large)
    );
}

#[test]
fn symbolic_algebra_is_not_performed() {
    let (x, a, b) = (symbol("x"), symbol("a"), symbol("b"));
    assert_ne!(
        Expr::add([x.clone(), x.clone()]),
        Expr::mul([2.into(), x.clone()])
    );
    assert_ne!(
        Expr::mul([x.clone(), x.clone()]),
        Expr::pow(x.clone(), 2.into())
    );
    assert_ne!(
        Expr::add([
            Expr::mul([2.into(), x.clone()]),
            Expr::mul([3.into(), x.clone()])
        ]),
        Expr::mul([5.into(), x.clone()])
    );
    assert_ne!(
        Expr::mul([
            Expr::pow(x.clone(), a.clone()),
            Expr::pow(x.clone(), b.clone())
        ]),
        Expr::pow(x.clone(), Expr::add([a.clone(), b.clone()]))
    );
    assert_ne!(
        Expr::mul([x.clone(), Expr::add([a.clone(), b.clone()])]),
        Expr::add([Expr::mul([x.clone(), a]), Expr::mul([x, b])])
    );
}

#[test]
fn powers_only_remove_exponent_one() {
    let x = symbol("x");
    assert_eq!(Expr::pow(x.clone(), 1.into()), x);
    assert_eq!(Expr::pow(x.clone(), rational(3, 3)), x);
    for (base, exponent) in [
        (x.clone(), 0.into()),
        (0.into(), 0.into()),
        (2.into(), 10.into()),
        (2.into(), (-1).into()),
        (1.into(), x.clone()),
        (0.into(), x.clone()),
        (Expr::pow(x, 2.into()), 3.into()),
    ] {
        let result = Expr::pow(base.clone(), exponent.clone());
        assert!(matches!(result.0.as_ref(), ExprNode::Pow(b, e) if b == &base && e == &exponent));
    }
}

#[test]
fn canonical_rank_and_recursive_lexicographic_order() {
    let (x, y) = (symbol("x"), symbol("y"));
    let powers = [
        Expr::pow(rational(1, 2), x.clone()),
        Expr::pow(1.into(), x.clone()),
        Expr::pow(x.clone(), 2.into()),
        Expr::pow(x.clone(), 3.into()),
    ];
    let product = Expr::mul([x.clone(), y.clone()]);
    let sum =
        Expr::add(
            powers
                .iter()
                .rev()
                .cloned()
                .chain([product, y.clone(), 7.into(), x.clone()]),
        );
    assert_eq!(
        sum.to_string(),
        "7 + x + y + (1/2) ^ x + 1 ^ x + x ^ 2 + x ^ 3 + x * y"
    );
    let short_sum = Expr::add([1.into(), x.clone()]);
    let long_sum = Expr::add([1.into(), x.clone(), y.clone()]);
    let different_sum = Expr::add([2.into(), x.clone()]);
    assert_eq!(
        Expr::mul([
            different_sum,
            long_sum,
            short_sum,
            powers[2].clone(),
            y.clone()
        ])
        .to_string(),
        "y * x ^ 2 * (1 + x) * (1 + x + y) * (2 + x)"
    );
    let short_product = Expr::mul([2.into(), x.clone()]);
    let long_product = Expr::mul([2.into(), x.clone(), y]);
    let different_product = Expr::mul([3.into(), x]);
    assert_eq!(
        Expr::add([different_product, long_product, short_product]).to_string(),
        "2 * x + 2 * x * y + 3 * x"
    );
}

#[test]
fn display_preserves_precedence_and_power_scope() {
    let (x, y) = (symbol("x"), symbol("y"));
    let sum = Expr::add([x.clone(), 1.into()]);
    for (expr, expected) in [
        (x.clone(), "x"),
        (rational(3, 2), "3/2"),
        (Expr::add([y.clone(), x.clone()]), "x + y"),
        (Expr::mul([x.clone(), 2.into()]), "2 * x"),
        (Expr::mul([sum.clone(), y.clone()]), "y * (1 + x)"),
        (Expr::pow(sum, 2.into()), "(1 + x) ^ 2"),
        (
            Expr::pow(x.clone(), Expr::add([y.clone(), 1.into()])),
            "x ^ (1 + y)",
        ),
        (
            Expr::pow(Expr::mul([x.clone(), y.clone()]), 2.into()),
            "(x * y) ^ 2",
        ),
        (
            Expr::pow(x.clone(), Expr::mul([2.into(), y.clone()])),
            "x ^ (2 * y)",
        ),
        (
            Expr::pow(Expr::pow(x.clone(), y.clone()), 2.into()),
            "(x ^ y) ^ 2",
        ),
        (Expr::pow(x.clone(), Expr::pow(y, 2.into())), "x ^ (y ^ 2)"),
        (Expr::pow((-2).into(), rational(1, 2)), "(-2) ^ (1/2)"),
        (Expr::pow(rational(-1, 2), (-3).into()), "(-1/2) ^ (-3)"),
        (Expr::add([(-1).into(), x]), "-1 + x"),
        (symbol("x + y"), "\"x + y\""),
        (symbol("2"), "\"2\""),
        (symbol("a\"b\n"), "\"a\\\"b\\n\""),
    ] {
        assert_eq!(expr.to_string(), expected);
    }
}

fn assert_invariants(expr: &Expr) {
    match expr.0.as_ref() {
        ExprNode::Rational(value) => assert!(!value.denominator().is_one()),
        ExprNode::Add(children) | ExprNode::Mul(children) => {
            let sum = matches!(expr.0.as_ref(), ExprNode::Add(_));
            assert!(children.len() >= 2);
            assert!(
                children
                    .windows(2)
                    .all(|p| p[0].canonical_cmp(&p[1]) != Ordering::Greater)
            );
            assert!(children.iter().filter(|c| c.scalar().is_some()).count() <= 1);
            for child in children {
                assert!(!matches!(
                    (sum, child.0.as_ref()),
                    (true, ExprNode::Add(_)) | (false, ExprNode::Mul(_))
                ));
                if let Some(scalar) = child.scalar() {
                    assert!(!scalar.numerator().is_zero());
                    if !sum {
                        assert_ne!(scalar, Rational::from(Integer::one()));
                    }
                }
                assert_invariants(child);
            }
        }
        ExprNode::Pow(base, exponent) => {
            assert_ne!(exponent, &Expr::from(1));
            assert_invariants(base);
            assert_invariants(exponent);
        }
        _ => {}
    }
}

fn atom() -> impl Strategy<Value = Expr> {
    prop_oneof![
        (-20i64..=20).prop_map(Expr::from),
        (-20i64..=20, 1i64..=9).prop_map(|(n, d)| rational(n, d)),
        prop::sample::select(vec!["x", "y", "α", "a + b"]).prop_map(symbol),
    ]
}

// Fixed, shallow combinations exercise all node forms without recursive generation.
fn shallow() -> impl Strategy<Value = Expr> {
    prop_oneof![
        atom(),
        (atom(), atom()).prop_map(|(a, b)| Expr::add([a, b])),
        (atom(), atom()).prop_map(|(a, b)| Expr::mul([a, b])),
        (atom(), atom()).prop_map(|(a, b)| Expr::pow(a, b)),
    ]
}

proptest! {
    #![proptest_config(Config {
        cases: 128,
        rng_seed: RngSeed::Fixed(0x45585052),
        rng_algorithm: RngAlgorithm::ChaCha,
        ..Config::default()
    })]

    #[test]
    fn add_commutes(a in shallow(), b in shallow()) {
        prop_assert_eq!(Expr::add([a.clone(), b.clone()]), Expr::add([b, a]));
    }

    #[test]
    fn mul_commutes(a in shallow(), b in shallow()) {
        prop_assert_eq!(Expr::mul([a.clone(), b.clone()]), Expr::mul([b, a]));
    }

    #[test]
    fn add_associates(a in shallow(), b in shallow(), c in shallow()) {
        prop_assert_eq!(Expr::add([Expr::add([a.clone(), b.clone()]), c.clone()]),
            Expr::add([a, Expr::add([b, c])]));
    }

    #[test]
    fn mul_associates(a in shallow(), b in shallow(), c in shallow()) {
        prop_assert_eq!(Expr::mul([Expr::mul([a.clone(), b.clone()]), c.clone()]),
            Expr::mul([a, Expr::mul([b, c])]));
    }

    #[test]
    fn identities_and_annihilation(a in shallow()) {
        prop_assert_eq!(Expr::add([a.clone(), 0.into()]), a.clone());
        prop_assert_eq!(Expr::mul([a.clone(), 1.into()]), a.clone());
        prop_assert_eq!(Expr::mul([a.clone(), 0.into()]), Expr::from(0));
        prop_assert_eq!(Expr::pow(a.clone(), 1.into()), a);
    }

    #[test]
    fn permutations_preserve_structure_hash_and_display(
        operands in prop::collection::vec(shallow(), 0..8), rotation in 0usize..8
    ) {
        let mut reordered = operands.clone();
        reordered.reverse();
        if !reordered.is_empty() {
            let amount = rotation % reordered.len();
            reordered.rotate_left(amount);
        }
        for (a, b) in [
            (Expr::add(operands.clone()), Expr::add(reordered.clone())),
            (Expr::mul(operands), Expr::mul(reordered)),
        ] {
            assert_invariants(&a);
            assert_invariants(&b);
            prop_assert_eq!(hash(&a), hash(&b));
            prop_assert_eq!(a.to_string(), b.to_string());
            prop_assert_eq!(a, b);
        }
    }

    #[test]
    fn canonical_comparator_is_total_and_agrees_with_equality(a in shallow(), b in shallow(), c in shallow()) {
        let ab = a.canonical_cmp(&b);
        let bc = b.canonical_cmp(&c);
        prop_assert_eq!(ab, b.canonical_cmp(&a).reverse());
        prop_assert_eq!(ab == Ordering::Equal, a == b);
        if ab != Ordering::Greater && bc != Ordering::Greater {
            prop_assert!(a.canonical_cmp(&c) != Ordering::Greater);
        }
    }

    #[test]
    fn rational_wholes_lift_like_integers(n in -1000i64..=1000, d in 1i64..=100) {
        let whole = rational(n * d, d);
        prop_assert_eq!(hash(&whole), hash(&Expr::from(n)));
        prop_assert_eq!(whole, Expr::from(n));
    }
}
