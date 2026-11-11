# Mathematishia

Mathematishia is an experimental computer algebra system written in Rust. It provides exact arithmetic, symbolic expressions, polynomial algorithms, equation solvers, and checkable records of mathematical transformations.

The Rust kernel is shared by the command-line tool and Python bindings. Operations have explicit domain restrictions; for example, rational functions retain excluded denominator zeros after cancellation.

## Build and test

Requires a Rust toolchain supporting edition 2024.

```sh
cargo build --workspace --locked
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

## Command-line examples

```sh
cargo run -p mathematishia-cli -- expr '1/2+1/3'
cargo run -p mathematishia-cli -- rsimplify '(x^2-1)/(x-1)' x
cargo run -p mathematishia-cli -- rdiff '(x^2-1)/(x-1)' x
cargo run -p mathematishia-cli -- rsolve '(x^2-1)/(x-1)=3' x
cargo run -p mathematishia-cli -- domain '0*x^-1=0*y^-1'
cargo run -p mathematishia-cli -- subdomain '0*x^-1' x 0
cargo run -p mathematishia-cli -- rewrite '(x+1)*(x+1)^-1' 'x+1'
cargo run -p mathematishia-cli -- expand '2*(x+3)' 4
cargo run -p mathematishia-cli -- solve '2*x+4=10' x
cargo run -p mathematishia-cli -- roots2 'x^2-2' x
cargo run -p mathematishia-cli -- roots2 'x^2+1' x
cargo run -p mathematishia-cli -- qvalue 'x^2-2' upper_real 1 2 x
cargo run -p mathematishia-cli -- qop mul 'x^2+1' positive_imaginary 0 1 0 1 x
cargo run -p mathematishia-cli -- modop mul x x 'x^2+1' x
cargo run -p mathematishia-cli -- divide 'x^2-1' 'x-1' x
cargo run -p mathematishia-cli -- egcd 'x^3-1' 'x^2-1' x
cargo run -p mathematishia-cli -- modinv x 'x^2+1' x
cargo run -p mathematishia-cli -- sqfree '2*(x-1)^2*(x+2)^3' x
cargo run -p mathematishia-cli -- factor '(x-1)*(x^2+1)' x
cargo run -p mathematishia-cli -- matdet '2,1;5,3'
cargo run -p mathematishia-cli -- matsolve '2,1;5,3' '5,13'
cargo run -p mathematishia-cli -- factor '(x^3+x+1)*(x^3+x^2+1)' x
cargo run -p mathematishia-cli -- fieldmin 'x^3-2' 'x^2' x
cargo run -p mathematishia-cli -- fieldpow 'x^3-2' x -1 x
cargo run -p mathematishia-cli -- diffexpr 'x*y+(x+1)^3' x
```

The CLI also exposes polynomial evaluation and GCD, rational-function operations, and rational equation solving. Run `cargo run -p mathematishia-cli -- --help` for the command reference.

## Matrices

Rational matrices support exact determinants, inverses, row reduction,
rank, multiplication and linear systems. A system can have no solution,
one solution or an affine family of solutions. CLI matrix rows are
semicolon-separated; entries within each row are comma-separated.
The current matrix solver limits each dimension to 16.

## Python

The native bindings use PyO3 and maturin. Install the package from a source checkout with Python 3.10+ and Rust:

```sh
python -m pip install ./crates/python
```

```python
import mathematishia as m

solution = m.solve("2*x+4=10", "x")
roots = m.algebraic_roots("x^2-2", "x")
for root in roots.roots:
    print(root.kind, root.branch, root.minimal_polynomial)
```

See [the Python API notes](crates/python/README.md) for supported operations.

## Implementation

| Crate | Purpose |
| --- | --- |
| `mathema` | Arbitrary-precision scalars, symbols, immutable expression trees, typed conditions |
| `mathematishia-kernel` | Polynomial and rational-function algorithms, equation solving, algebraic roots, derivation verification |
| `mathema-frontend` | Bounded expression and equation parser |
| `mathematishia-cli` | Command-line interface |
| `mathematishia` (Python) | Native bindings to the Rust kernel |

The parser supports exact scalar and symbolic division with explicit multiplication.
The `rsimplify`, `rdiff`, `reval`, and `rsolve` commands handle univariate
rational expressions over Q, preserving excluded denominator zeros after
cancellation. Solving currently returns admissible rational roots when the
residual numerator has degree at most two.

Domain-aware parsing retains restrictions removed by structural normalization;
`domain`, `subdomain`, and `rewrite` expose them. Some older expression
operations do not preserve this provenance.

`factor` returns certified irreducible factors and, where necessary, unresolved
cofactors. In addition to rational roots and quadratics, it handles bounded
quartic splitting, modular irreducibility certificates, and selected higher-degree
factorizations reconstructed by Hensel lifting. It is not yet a complete
factorizer over Q.

`fieldmin`, `fieldpow`, and `fieldop` operate in abstract extensions with
certified irreducible defining polynomials of degree two through eight.
They do not select real or complex embeddings. The `qvalue` interface handles
embedded quadratic arithmetic separately. Cross-field arithmetic and analytic
functions are not implemented.

Resource limits bound selected algebraic operations; they are not process
memory or execution-time limits.

For implemented operations and source locations, see [CAPABILITIES.md](CAPABILITIES.md). Architectural contracts are in [docs/design](docs/design), with planned work in [ROADMAP.md](ROADMAP.md).

Licensed under [Apache-2.0](LICENSE).
