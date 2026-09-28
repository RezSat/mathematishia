# Mathematishia

Mathematishia is an open-source computer algebra system with an initial exact
scalar foundation. Its kernel is intended to provide deterministic
mathematical computation, exact results where practical, explicit numerical
approximation, and structured deterministic derivations without AI inside the
mathematical kernel.

The design uses specialized representations for mathematical domains and
requires benchmark evidence before adding performance complexity.

## Repository

- `crates/mathema`: exact `Integer` and canonical `Rational` semantic values.
- `crates/kernel`: future mathematical algorithms and derivations.
- `docs/design`: architecture contracts; `docs/research`: architecture and
  provenance records.

`mathema` supports arbitrary-precision integers and reduced rationals with exact
negation, addition, subtraction, multiplication, equality, ordering, hashing,
and basic human formatting. Rational construction rejects zero denominators.
Example tests and property tests cover these value invariants. There is no
stable API, symbolic expression system, or CAS user interface yet.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Licensed under Apache-2.0; see [LICENSE](LICENSE).
