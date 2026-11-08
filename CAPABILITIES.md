# Implementation index

This index lists the current mathematical capabilities and their principal
implementation points. The architecture is described in [ARCHITECTURE.md](ARCHITECTURE.md);
the longer-term plan is in [docs/design/cas-platform.md](docs/design/cas-platform.md).

| Capability | Scope | Implementation |
| --- | --- | --- |
| Exact scalar expressions | Integers, rationals, symbols, sums, products, powers; conservative normalization | `crates/mathema/src/` |
| Polynomial arithmetic | Univariate Q[x], division, GCD, formal derivative, square-free factors | `crates/kernel/src/polynomial.rs`, `poly_division.rs`, `square_free.rs` |
| Equation solving | Rational-linear, selected quadratic and rational equations with exclusions | `crates/kernel/src/solve_linear.rs`, `quadratic.rs`, `rational_equation.rs`, `rational_quadratic.rs` |
| Quadratic algebraic roots | Exact real irrational and complex quadratic roots, identity and verification; no general field operations | `crates/kernel/src/algebraic.rs` |
| Derivation checks | Distribution, linear-equation equivalence, derivative rules, root evidence and guarded reciprocal cancellation | `crates/kernel/src/verification.rs`, `differentiate_expr.rs`, `algebraic.rs`, `guarded_rewrite.rs` |
| Scalar assumptions | Typed nonzero, positive, real, integer and domain conditions; exact literal facts and limited premise implications | `crates/mathema/src/condition.rs`, `crates/kernel/src/assumptions.rs` |
| User interfaces | Bounded parser, CLI, experimental Python bindings | `crates/frontend/`, `crates/cli/`, `crates/python/` |
| Distribution | Cross-platform Rust tests, installed Python wheel tests and source distribution rebuild | `.github/workflows/ci.yml`, `docs/release/pypi.md` |

## Outstanding foundations

The immediate next task is a proof graph with typed nodes, dependencies,
relations, conditions, exact witnesses and independently checked edges.
Next are conditional rewriting over broader expression forms and a
definedness model for general reciprocals. General algebraic arithmetic,
irreducible polynomial factorization, analytic functions, certified numerics
and user-facing proof rendering remain outside the current implementation.

## Verification

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

The test suite and the CI workflow are the source of truth for passing
checks; this index documents availability, not a substitute for validation.
Update this file when a capability or its implementation location changes.
