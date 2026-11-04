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
- `crates/frontend`: the first bounded Mathema expression/equation parser and lowering.
- `crates/cli`: a CLI that uses the frontend and kernel without duplicating mathematics.
- `crates/python`: native PyO3/maturin Python package over the same checked kernel.
- `crates/kernel`: mathematical transformations, structured derivation steps, and
  the initial exact rational-polynomial domain.
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
Example tests and property tests cover these invariants. The kernel inspects
expressions through a read-only semantic view and provides exact single-symbol
substitution and bounded, deterministic distributive expansion with a chain of
justified steps. The expansion budget counts steps but does not bound memory or
expression size. There is no parser, general rewriting, general simplification engine,
or user-facing CAS API yet. A separate Equation value supports a deliberately
bounded exact univariate linear solver, with typed equivalent-equation steps,
verified nonzero division, and three solution-set classifications. RationalPolynomial supports canonical
univariate rational coefficients, exact arithmetic, and Horner evaluation;
fallible bounded expression-to-polynomial conversion and conversion back to
semantic expressions are available. Polynomial long division and canonical
monic GCD are exact, with division-by-zero rejected.


## First CLI commands

Use quoted arguments so your shell passes the complete Mathema expression:

```sh
cargo run -p mathematishia-cli -- expr '1/2+1/3'
cargo run -p mathematishia-cli -- expand '2*(x+3)' 4
cargo run -p mathematishia-cli -- solve '2*x+4=10' x
cargo run -p mathematishia-cli -- poly '(x+1)^2' x
cargo run -p mathematishia-cli -- eval 'x^2+1/2' x '1/2'
cargo run -p mathematishia-cli -- divide 'x^2-1' 'x-1' x
cargo run -p mathematishia-cli -- gcd 'x^2-1' 'x-1' x
```

The solver currently supports only exact rational-linear equations in one
selected variable. The CLI uses the explicit checked evaluation context,
including structural limits and independent derivation checks. Expansion is
step-limited; these checks are not hard memory, wall-clock or intermediate
allocation bounds. See [Mathema's initial syntax](docs/design/mathema-surface.md).

## Python bindings

An experimental native Python package is available in `crates/python`, with
immutable typed results, Python `fractions.Fraction`, and distinct syntax,
unsupported-mathematics, resource-limit and verification exceptions. It is
built from the Rust kernel, not a Python algebra rewrite.

```sh
python -m pip install ./crates/python
python -c "import mathematishia as m; print(m.solve('2*x+4=10', 'x'))"
```

A dedicated GitHub Actions job builds a wheel, installs it in isolation and
runs the Python smoke suite. Python 3.10+ is required.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Licensed under Apache-2.0; see [LICENSE](LICENSE).
