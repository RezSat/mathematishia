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
expression size. A bounded Mathema parser exists in the frontend, but general
rewriting, general simplification and a mature user-facing CAS API are not yet
implemented. The separate `Equation` value supports exact rational-linear
solving with typed equivalence steps, verified nonzero division and all three
solution-set classifications. `RationalPolynomial` supports canonical
univariate rational coefficients, exact arithmetic and Horner evaluation;
fallible bounded expression-to-polynomial conversion and conversion back to
semantic expressions are available. Polynomial long division and canonical
monic GCD are exact, with division-by-zero rejected. Exact formal polynomial
differentiation and verified square-free multiplicity decomposition are also
available; full irreducible factorization is not yet. A deliberately scoped
symbolic derivative supports constants, symbols, sums, products and positive
integer powers with independently verified typed rules. General function
calculus, assumptions and symbolic integration remain future work.

## First CLI commands

Use quoted arguments so your shell passes the complete Mathema expression:

```sh
cargo run -p mathematishia-cli -- expr '1/2+1/3'
cargo run -p mathematishia-cli -- expand '2*(x+3)' 4
cargo run -p mathematishia-cli -- solve '2*x+4=10' x
cargo run -p mathematishia-cli -- solveq 'x^2-5*x+6' x
cargo run -p mathematishia-cli -- roots2 'x^2-2' x
cargo run -p mathematishia-cli -- roots2 'x^2+1' x
cargo run -p mathematishia-cli -- poly '(x+1)^2' x
cargo run -p mathematishia-cli -- eval 'x^2+1/2' x '1/2'
cargo run -p mathematishia-cli -- divide 'x^2-1' 'x-1' x
cargo run -p mathematishia-cli -- gcd 'x^2-1' 'x-1' x
cargo run -p mathematishia-cli -- diff 'x^3+1/2*x' x
cargo run -p mathematishia-cli -- sqfree '2*(x-1)^2*(x+2)^3' x
cargo run -p mathematishia-cli -- diffexpr 'x*y+(x+1)^3' x
cargo run -p mathematishia-cli -- rat 'x^2-1' 'x-1' x
cargo run -p mathematishia-cli -- rateval 'x^2-1' 'x-1' x 2
cargo run -p mathematishia-cli -- ratdiff '1' 'x-1' x
```

The solver currently supports only exact rational-linear equations in one
selected variable. The CLI uses the checked evaluation context, including
per-call operation budgets and conservative intermediate scalar-bit checks.
The `diffexpr` command supports constants, independent symbols, sums,
products and positive integer powers (default upper bound 32), and returns
independently verified derivative steps with `derivative_of` relations.
Zero, negative, fractional and symbolic exponents are rejected rather than
assigned implicit domain assumptions. These checks are not hard process memory
or wall-clock isolation. See [Mathema's initial syntax](docs/design/mathema-surface.md).

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
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Licensed under Apache-2.0; see [LICENSE](LICENSE).

### Exact rational functions and original domains

The CLI `rat`, `rateval`, `ratdiff` and `ratop` commands accept separate
polynomial numerator and denominator inputs. GCD cancellation does not
silently fill domain holes: `(x^2-1)/(x-1)` is undefined at `x=1`, even
when its reduced numerator is `x+1`. The result lists explicit polynomial
nonzero conditions. General `/` expression syntax and multivariate rational
functions remain future work.

### Exact rational roots of quadratic polynomials

`solveq <POLYNOMIAL> <VARIABLE>` and Python
`solve_quadratic("x^2-5*x+6", "x")` find all **rational** roots of a
polynomial of degree at most two. The exact discriminant is checked using
arbitrary-precision integer and rational square roots, without approximate
floating-point tests. Missing rational roots do not imply that a polynomial
has no real or complex roots: `x^2-2` has no roots in Q, but it has roots
involving sqrt(2). Higher-degree inputs receive a typed unsupported-domain
error. All candidate roots are verified by exact substitution, and the
independent checker validates completeness with the exact discriminant.

### Domain-aware rational quadratic equations

`ratsolveq <N1> <D1> <N2> <D2> <VARIABLE>` and Python
`rational_solve_quadratic` solve rational equations whose reduced
numerator has degree at most two. They preserve the **original denominator
conditions**, even when GCD cancellation would otherwise hide a hole. For
`(x^2-5*x+6)/(x-2) = 0`, the only admissible rational root is `x=3`:
`x=2` is excluded by the original denominator. A zero residual means all
*admissible* rationals, not automatically all Q. An independent verifier
checks exact recomposition and the complete rational root set.

### Exact quadratic-algebraic numbers and explainable roots

The new roots2 command computes all complex roots of rational-coefficient
polynomials of degree at most two. Rational answers remain exact rationals.
Real irrational and nonreal quadratic roots are exact values identified by
an irreducible monic minimal polynomial and a branch (lower/upper real or
negative/positive imaginary). This is a canonical identity for this limited
degree-two domain, not a general implementation of algebraic-number arithmetic,
radical simplification, arbitrary complex ordering, or a full symbolic solver.

Each result includes a structured evidence sequence: monic normalization,
an exact discriminant, and a stable-ID classification of the quadratic formula.
The kernel checks that evidence independently. Python exposes these as
immutable dataclasses through mathematishia.algebraic_roots(...). The legacy
solveq rational-only interface remains available and unchanged. See
[the CAS engineering architecture](docs/design/cas-platform.md).

### Distributing a Python package

The Python source package already builds through maturin. Publishing the
first package to PyPI is a separate, gated release milestone: it needs a
final version/tag, a tested wheel matrix, a maintainer-controlled PyPI
account and Trusted Publisher configuration. We will not claim that the
package is on PyPI until its published artifacts have been installed and
tested. See [the publishing checklist](docs/release/pypi.md).
