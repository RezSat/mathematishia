# Implementation index

| Area | Source |
| --- | --- |
| Exact scalars, expressions and conditions | `crates/mathema/src/` |
| Polynomial arithmetic, division and GCD | `crates/kernel/src/polynomial.rs`, `poly_division.rs` |
| Square-free factorization | `crates/kernel/src/square_free.rs` |
| Rational functions with domain exclusions | `crates/kernel/src/rational_function.rs` |
| Rational-expression conversion and simplification over Q(x) | `crates/kernel/src/rational_expression.rs` |
| Linear, quadratic and rational equations | `crates/kernel/src/solve_linear.rs`, `quadratic.rs`, `rational_equation.rs`, `rational_quadratic.rs` |
| Exact quadratic roots | `crates/kernel/src/algebraic.rs` |
| Symbolic differentiation | `crates/kernel/src/differentiate_expr.rs` |
| Assumptions and guarded rewrites | `crates/kernel/src/assumptions.rs`, `guarded_rewrite.rs`, `conditional_rewrite.rs` |
| Definedness and domain-preserving expression composition | `crates/mathema/src/definedness.rs` |
| Proof graph and dependency verification | `crates/kernel/src/proof_graph.rs` |
| Distribution and linear-step checking | `crates/kernel/src/lib.rs`, `verification.rs` |
| Domain-aware expression/equation parser | `crates/frontend/src/lib.rs` |
| Domain-preserving substitution and proof-checked rewrites | `crates/kernel/src/domain_operations.rs` |
| Parser and command-line commands | `crates/frontend/`, `crates/cli/` |
| Python bindings | `crates/python/` |
| Build and package checks | `.github/workflows/ci.yml` |

## Current boundaries

The rational-expression converter supports exact scalars, one variable, sums, products, and bounded integer powers. It preserves source-domain restrictions and feeds the existing quotient-rule and degree-two rational solver; see `docs/design/rational-expressions.md`.

The proof graph records exact expression inputs, condition nodes, rule evidence and typed parent references, including composite cancellation with multiple conditions. Its verifier rejects invalid steps, unsupported conditions, forward dependencies and disconnected nodes. It does not yet compose equation proofs, establish new symbolic assumptions or cover fractional/analytic definedness. `parse_domain_expression` and `parse_domain_statement` retain construction-time restrictions for integer powers. The legacy `parse_expression` and `parse_statement` interfaces still return ordinary `Expr` values; consumers must opt into the domain-aware path.

General algebraic-number arithmetic, irreducible polynomial factorization, analytic functions, certified numerics and proof rendering remain to be implemented.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Update this index when a module is added or its supported scope changes.
