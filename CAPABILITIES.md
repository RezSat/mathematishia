# Implementation index

| Area | Source |
| --- | --- |
| Exact scalars, expressions and conditions | `crates/mathema/src/` |
| Polynomial arithmetic, division and GCD | `crates/kernel/src/polynomial.rs`, `poly_division.rs` |
| Square-free factorization | `crates/kernel/src/square_free.rs` |
| Rational functions with domain exclusions | `crates/kernel/src/rational_function.rs` |
| Linear, quadratic and rational equations | `crates/kernel/src/solve_linear.rs`, `quadratic.rs`, `rational_equation.rs`, `rational_quadratic.rs` |
| Exact quadratic roots | `crates/kernel/src/algebraic.rs` |
| Symbolic differentiation | `crates/kernel/src/differentiate_expr.rs` |
| Assumption classification and guarded rewriting | `crates/kernel/src/assumptions.rs`, `guarded_rewrite.rs` |
| Proof graph and dependency verification | `crates/kernel/src/proof_graph.rs` |
| Distribution and linear-step checking | `crates/kernel/src/lib.rs`, `verification.rs` |
| Parser and command-line commands | `crates/frontend/`, `crates/cli/` |
| Python bindings | `crates/python/` |
| Build and package checks | `.github/workflows/ci.yml` |

## Current boundaries

The proof graph records exact expression inputs, condition nodes, rule evidence and typed parent references. Its verifier rejects invalid steps, unsupported conditions, forward dependencies and disconnected nodes. It does not yet compose equation proofs, establish new symbolic assumptions or infer general definedness.

General algebraic-number arithmetic, irreducible polynomial factorization, analytic functions, certified numerics and proof rendering remain to be implemented.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Update this index when a module is added or its supported scope changes.
