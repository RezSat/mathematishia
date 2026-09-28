# Mathematishia

Mathematishia is an open-source computer algebra system with an initial exact
scalar and minimal symbolic expression foundation. Its kernel is intended to provide deterministic
mathematical computation, exact results where practical, explicit numerical
approximation, and structured deterministic derivations without AI inside the
mathematical kernel.

The design uses specialized representations for mathematical domains and
requires benchmark evidence before adding performance complexity.

## Repository

- `crates/mathema`: exact scalars, free symbols, and immutable symbolic expressions.
- `crates/kernel`: future mathematical algorithms and derivations.
- `docs/design`: architecture contracts; `docs/research`: architecture and
  provenance records.

`mathema` supports arbitrary-precision integers and reduced rationals with exact
negation, addition, subtraction, multiplication, equality, ordering, hashing,
and basic human formatting. Rational construction rejects zero denominators.
It also provides exact-name `Symbol` values and opaque immutable `Expr` values
with Integer, Rational, Symbol, Add, Mul, and Pow forms. Constructors flatten
sums/products, fold exact scalars, apply zero/one identities, and sort operands
deterministically. Powers only remove an exponent of one. Equality and hashing
are structural; display is basic human formatting, not serialization.
Example tests and property tests cover these invariants. There is no parser,
substitution, rewriting, simplification engine, solver, or user-facing CAS API yet.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Licensed under Apache-2.0; see [LICENSE](LICENSE).
