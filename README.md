# Mathematishia

Mathematishia is an open-source computer algebra system in its architecture and
repository foundation stage. Its kernel is intended to provide deterministic
mathematical computation, exact results where practical, explicit numerical
approximation, and structured deterministic derivations without AI inside the
mathematical kernel.

The design uses specialized representations for mathematical domains and
requires benchmark evidence before adding performance complexity.

## Repository

- `crates/mathema`: future syntax-independent semantic representation.
- `crates/kernel`: future mathematical algorithms and derivations.
- `docs/design`: architecture contracts; `docs/research`: architecture and
  provenance records.

There is no stable API or useful computer algebra functionality yet.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Licensed under Apache-2.0; see [LICENSE](LICENSE).
