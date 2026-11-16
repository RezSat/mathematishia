# Implementation index

| Area | Source |
| --- | --- |
| Exact scalars, expressions and conditions | `crates/mathema/src/` |
| Analytic function nodes, real/complex domain-aware parsing | `crates/mathema/src/expr.rs`, `definedness.rs`, `crates/frontend/src/lib.rs` |
| Polynomial arithmetic, division and GCD | `crates/kernel/src/polynomial.rs`, `poly_division.rs` |
| Canonical bounded multivariate polynomials over Q | `crates/kernel/src/multivariate_polynomial.rs` |
| Exact ordered multivariate polynomial division and checked remainder | `crates/kernel/src/multivariate_division.rs` |
| Exact polynomial primitives and directed definite integrals | `crates/kernel/src/integration.rs` |
| Extended polynomial GCD, Bézout verification, modular inverses | `crates/kernel/src/extended_gcd.rs` |
| Polynomial quotient rings and canonical residue arithmetic | `crates/kernel/src/polynomial_residue.rs` |
| Exact rational matrices, RREF, determinant, inverse and affine linear systems | `crates/kernel/src/exact_matrix.rs` |
| Square-free factorization | `crates/kernel/src/square_free.rs` |
| Certified rational factors and unresolved cofactors | `crates/kernel/src/rational_factorization.rs` |
| Small-prime polynomial arithmetic and irreducibility | `crates/kernel/src/finite_field.rs`, `modular_irreducibility.rs` |
| Finite-field factorization | `crates/kernel/src/finite_field_factorization.rs` |
| Monic quartic quadratic-factor search | `crates/kernel/src/quartic_factor.rs` |
| Coprime Hensel lifting and exact factor reconstruction | `crates/kernel/src/hensel_lift.rs`, `modular_reconstruction.rs` |
| Rational functions with domain exclusions | `crates/kernel/src/rational_function.rs` |
| Rational-expression conversion and simplification over Q(x) | `crates/kernel/src/rational_expression.rs` |
| Linear, quadratic and rational equations | `crates/kernel/src/solve_linear.rs`, `quadratic.rs`, `rational_equation.rs`, `rational_quadratic.rs` |
| Exact multivariable affine equation systems | `crates/kernel/src/linear_system_expr.rs` |
| Certified rational root brackets | `crates/kernel/src/root_isolation.rs` |
| Exact quadratic roots | `crates/kernel/src/algebraic.rs` |
| Embedded quadratic fields, arithmetic, trace, norm and minimal-polynomial checks | `crates/kernel/src/quadratic_field.rs` |
| Certified abstract extensions (degree 2–8) | `crates/kernel/src/algebraic_extension.rs` |
| Exact minimal polynomials of extension elements | `crates/kernel/src/extension_minimal.rs` |
| Verified symbolic differentiation, including analytic chain rules | `crates/kernel/src/differentiate_expr.rs`, `context.rs` |
| Assumptions and guarded rewrites | `crates/kernel/src/assumptions.rs`, `guarded_rewrite.rs`, `conditional_rewrite.rs` |
| Definedness and domain-preserving expression composition | `crates/mathema/src/definedness.rs` |
| Proof graph and dependency verification | `crates/kernel/src/proof_graph.rs` |
| Verified derivation rendering for supported proofs, equations and derivatives | `crates/kernel/src/derivation_render.rs` |
| Distribution and linear-step checking | `crates/kernel/src/lib.rs`, `verification.rs` |
| Domain-aware expression/equation parser | `crates/frontend/src/lib.rs` |
| Domain-preserving substitution and proof-checked rewrites | `crates/kernel/src/domain_operations.rs` |
| Parser and command-line commands | `crates/frontend/`, `crates/cli/` |
| Python bindings | `crates/python/` |
| Build and package checks | `.github/workflows/ci.yml` |

## Current boundaries

Analytic calls support `sin`, `cos`, `tan`, `exp` and natural `log` as unevaluated expressions. The domain-aware parser preserves restrictions through nested calls and structural normalization, with real or complex domain selection. The derivative verifier supports chain rules for sine, cosine and exponential. Tangent requires `cos(argument) != 0`; real logarithm requires a positive argument. Complex principal-log derivatives are not admitted because the current domain model does not encode its branch cut. Conditions remain unproved prerequisites, not claims of validity. CLI `diffreal` and Python `differentiate_expression_real` expose the checked real-domain result. The existing polynomial converter rejects function calls.

Sparse multivariate polynomials store exact rational coefficients in a fixed, explicitly named ambient ring. Arithmetic, partial differentiation, evaluation and substitution have bounded input and work limits. Ordered lexicographic division returns a quotient for each divisor and a remainder, checked against the original polynomial by exact recomposition and leading-term nondivisibility. Divisor order affects results. No unique normal form is claimed for arbitrary divisors; Gröbner bases and general `Expr` conversion remain unimplemented.

Exact polynomial integration returns zero-constant primitives or definite rational values at rational bounds. Primitives and integral values are independently checked. Multivariable affine equation solving classifies empty, unique and affine-family solution sets, restricted to 16 equations and 16 variables.

Exact rational root-bracket refinement certifies a rational root or existence within an opposite-sign interval; same-sign intervals are unresolved. It does not count or isolate every root. The derivation renderer checks supplied proof evidence before producing structured steps; it does not establish new mathematical rules.

The rational-expression converter supports exact scalars, one variable, sums, products, and bounded integer powers. It preserves source-domain restrictions and feeds the existing quotient-rule and degree-two rational solver; see `docs/design/rational-expressions.md`.

The proof graph records expression inputs, typed condition nodes and rule evidence. Its verifier rejects invalid steps, unsupported conditions, forward dependencies and disconnected nodes. The derivation renderer emits checked steps for the supported expression, linear-equation and derivative evidence, including analytic derivative prerequisites. It does not establish new assumptions or prove general equations. `parse_domain_expression` and `parse_domain_statement` preserve the construction history of integer powers and supported analytic functions over the complex scalars; the `_in_domain` entrypoints select real or complex obligations. Legacy `parse_expression` and `parse_statement` return plain `Expr` values without restrictions discarded during normalization.

Rational factorization first removes repeated factors. It uses the rational root theorem, exact quadratic discriminants, exhaustive bounded monic quartic splitting, and finite-field irreducibility certificates. Supported monic integral polynomials through degree eight can also be split by square-free modular factorization, coprime Hensel lifts and exact division. None of these bounds imply complete factorization for arbitrary inputs. When no factor or irreducibility proof is found, the remainder is unresolved.

Small-prime polynomial arithmetic admits primes through 251 and input degrees through 32. The finite-field factorizer uses capped divisor enumeration through degree eight. Hensel reconstruction is restricted to bounded monic integer polynomials and a fixed small-prime search.

Exact rational matrices permit up to 16 rows and 16 columns. Inverse candidates are checked against both multiplication identities. Solvers distinguish unique, inconsistent and infinite families. This is exact linear algebra over Q; eigenvalues, eigenvectors, symbolic matrices and floating-point matrix algorithms remain outside the implemented scope.

The quotient-ring type does not assume irreducibility. Embedded quadratic fields use explicit root branches. Abstract algebraic fields accept irreducibility-certified defining polynomials of degree two through eight but do not have selected complex embeddings. General factorization, arithmetic across distinct field presentations, analytic numerical evaluation, general analytic integration, comprehensive real-root isolation and general theorem proving remain unimplemented.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Update this index when a module is added or its supported scope changes.
