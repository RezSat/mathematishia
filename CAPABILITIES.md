# Implementation index

| Area | Source |
| --- | --- |
| Exact scalars, expressions and conditions | `crates/mathema/src/` |
| Analytic function nodes, real/complex domain-aware parsing | `crates/mathema/src/expr.rs`, `definedness.rs`, `crates/frontend/src/lib.rs` |
| Polynomial arithmetic, division and GCD | `crates/kernel/src/polynomial.rs`, `poly_division.rs` |
| Canonical bounded multivariate polynomials over Q | `crates/kernel/src/multivariate_polynomial.rs` |
| Explicit bounded Expr and DomainExpression conversion to multivariate Q-polynomials | `crates/kernel/src/multivariate_expr.rs` |
| Independently checked bounded polynomial expansion in an explicit ambient ring | `crates/kernel/src/expand_polynomial.rs` |
| Exact ordered multivariate polynomial division and checked remainder | `crates/kernel/src/multivariate_division.rs` |
| Verified final-result multivariate division explanation records | `crates/kernel/src/multivariate_derivation.rs` |
| Exact polynomial primitives and directed definite integrals | `crates/kernel/src/integration.rs` |
| Checked exact sin/cos/exp primitives for rational-affine arguments | `crates/kernel/src/elementary_integration.rs` |
| Verified bounded polynomial and affine analytic integration dispatch | `crates/kernel/src/integration_dispatch.rs` |
| Exact finite-point polynomial/rational-function limits with checked factor and sign evidence | `crates/kernel/src/limits.rs` |
| Exact real polynomial/rational inequalities with degree-two factors and canonical interval unions | `crates/kernel/src/inequalities.rs` |
| Extended polynomial GCD, Bézout verification, modular inverses | `crates/kernel/src/extended_gcd.rs` |
| Polynomial quotient rings and canonical residue arithmetic | `crates/kernel/src/polynomial_residue.rs` |
| Exact rational matrices, RREF, determinant, inverse and affine linear systems | `crates/kernel/src/exact_matrix.rs` |
| Square-free factorization | `crates/kernel/src/square_free.rs` |
| Certified rational factors and unresolved cofactors | `crates/kernel/src/rational_factorization.rs` |
| Exact partial complex root sets with unresolved polynomial cofactors | `crates/kernel/src/polynomial_solution_set.rs` |
| Small-prime polynomial arithmetic and irreducibility | `crates/kernel/src/finite_field.rs`, `modular_irreducibility.rs` |
| Finite-field factorization | `crates/kernel/src/finite_field_factorization.rs` |
| Monic quartic quadratic-factor search | `crates/kernel/src/quartic_factor.rs` |
| Coprime Hensel lifting and exact factor reconstruction | `crates/kernel/src/hensel_lift.rs`, `modular_reconstruction.rs` |
| Rational functions with domain exclusions | `crates/kernel/src/rational_function.rs` |
| Rational-expression conversion and simplification over Q(x) | `crates/kernel/src/rational_expression.rs` |
| Linear, quadratic and rational equations | `crates/kernel/src/solve_linear.rs`, `quadratic.rs`, `rational_equation.rs`, `rational_quadratic.rs` |
| Exact multivariable affine equation systems | `crates/kernel/src/linear_system_expr.rs` |
| Certified rational root brackets | `crates/kernel/src/root_isolation.rs` |
| Exact Sturm counts of distinct roots in rational non-root intervals | `crates/kernel/src/sturm_count.rs` |
| Exact quadratic roots | `crates/kernel/src/algebraic.rs` |
| Embedded quadratic fields, arithmetic, trace, norm and minimal-polynomial checks | `crates/kernel/src/quadratic_field.rs` |
| Certified abstract extensions (degree 2–8) | `crates/kernel/src/algebraic_extension.rs` |
| Exact minimal polynomials of extension elements | `crates/kernel/src/extension_minimal.rs` |
| Verified symbolic differentiation, including analytic chain rules | `crates/kernel/src/differentiate_expr.rs`, `context.rs` |
| Bounded exact analytic-constant rewrites with checked domain provenance | `crates/kernel/src/analytic_constants.rs` |
| Checked, typed problem outcomes and mathematical evidence adapters | `crates/kernel/src/problem_result.rs` |
| Assumptions and guarded rewrites | `crates/kernel/src/assumptions.rs`, `guarded_rewrite.rs`, `conditional_rewrite.rs` |
| Definedness and domain-preserving expression composition | `crates/mathema/src/definedness.rs` |
| Proof graph and dependency verification | `crates/kernel/src/proof_graph.rs` |
| Verified derivation rendering for supported proofs, equations and derivatives | `crates/kernel/src/derivation_render.rs` |
| Distribution and linear-step checking | `crates/kernel/src/lib.rs`, `verification.rs` |
| Domain-aware parser with exact decimal literals and selected implicit multiplication | `crates/frontend/src/lib.rs` |
| Domain-preserving substitution and proof-checked rewrites | `crates/kernel/src/domain_operations.rs` |
| CLI equation dispatch, root sets, polynomial expansion, supported integral sums and exact constants | `crates/cli/src/lib.rs`, `crates/cli/src/user_solve.rs` |
| Python bindings for checked polynomial roots, Sturm counts, polynomial expansion, integral sums and analytic identities | `crates/python/` |
| Compiled student-question corpus and independent exact mathematical regressions | `tests/problem_corpus/`, `crates/cli/tests/real_world_corpus.rs`, `crates/kernel/tests/metamorphic_regressions.rs`, `crates/kernel/tests/forged_evidence.rs` |
| Build and package checks | `.github/workflows/ci.yml` |

## Current boundaries

The parser now accepts exact finite decimals and selected implicit multiplication, including `2x`, `2(x+1)`, and `(x+1)(x-1)`. Ambiguous identifier juxtaposition, inequality comparisons, `abs`, and `sqrt` remain outside the accepted grammar.

The Rust kernel now has independently checked positive-integer polynomial expansion, an integration dispatcher for bounded sums of polynomial and supported affine analytic terms, and a public typed result contract that distinguishes exact, conditional, partial, unsupported and undefined cases. The existing CLI `expand` command remains distributive-step-only; `expandall` exposes complete bounded polynomial expansion. `solveauto`, `roots`, `integrate`, and `evalexact` expose selected checked operations, including polynomial and rational-affine sine/cosine/exponential integration sums. Python exposes typed counterparts for expansion and combined integration, plus root sets, Sturm counts and analytic constants. Both new expansion/integration interfaces preserve conditional results. Neither frontend yet uses a unified shared result wire contract; CLI JSON and general analytic integration remain pending.

The kernel computes finite rational-point limits of univariate rational functions, with exact factor orders, signed one-sided infinities, two-sided nonexistence and retained source exclusions. Real inequalities compare a checked rational function against zero; numerator, denominator and inherited restriction polynomials must each have degree at most two. Exact rational and certified quadratic-algebraic endpoints form canonical interval unions, with poles and removable holes excluded. Neither operation is exposed through CLI or Python yet; comparison syntax and source-domain preserving comparison adapters remain pending. Transcendental limits and higher-degree inequalities are not claimed.

The compiled CLI corpus contains 139 cases: 85 solved, 9 partial, 32 unsupported, and 13 explicit domain, input or resource refusals. These are regression sample counts, not universal mathematical coverage.


Analytic calls support `sin`, `cos`, `tan`, `exp` and natural `log` as unevaluated expressions. The domain-aware parser preserves restrictions through nested calls and structural normalization, with real or complex domain selection. The derivative verifier supports chain rules for sine, cosine and exponential. Tangent requires `cos(argument) != 0`; real logarithm requires a positive argument. Complex principal-log derivatives are not admitted because the current domain model does not encode its branch cut. Conditions remain unproved prerequisites, not claims of validity. CLI `diffreal` and Python `differentiate_expression_real` expose the checked real-domain result. The existing polynomial converter rejects function calls. A separate bounded one-step rewrite supports exactly `sin(0)=0`, `cos(0)=1`, `tan(0)=0`, `exp(0)=1` and `log(1)=0`, with independent rule checking and preserved input domain conditions; this is not arbitrary analytic evaluation.

Sparse multivariate polynomials store exact rational coefficients in a fixed, explicitly named ambient ring. Arithmetic, partial differentiation, evaluation and substitution have bounded input and work limits. Ordered lexicographic division returns a quotient for each divisor and a remainder, checked against the original polynomial by exact recomposition and leading-term nondivisibility. Divisor order affects results. No unique normal form is claimed for arbitrary divisors; Gröbner bases remain unimplemented. An explicit bounded `Expr` ↔ multivariate bridge now handles exact scalar and declared-symbol sums, products and strictly positive integer powers in a caller-selected ring, with inherited `DomainExpression` conditions retained. Zero, negative, noninteger and symbolic powers, undeclared symbols and function calls are rejected. The reverse `Expr` conversion preserves polynomial value, not original syntax.

Exact polynomial integration returns zero-constant primitives or definite rational values at rational bounds. Primitives and integral values are independently checked. A separate elementary integrator returns exact rational-multiple primitives for only `sin(ax+b)`, `cos(ax+b)` and `exp(ax+b)` with nonzero rational slope, retained source conditions and independently verified derivative evidence. General transcendental integration remains unsupported. Multivariable affine equation solving classifies empty, unique and affine-family solution sets, restricted to 16 equations and 16 variables.

Exact rational root-bracket refinement certifies a rational root or existence within an opposite-sign interval; same-sign intervals are unresolved. An exact bounded Sturm certificate now counts distinct real roots of square-free nonzero Q[x] inside rational intervals with non-root endpoints; repeated factors and root endpoints are explicitly rejected in this first increment. Neither operation isolates all roots. The derivation renderer checks supplied proof evidence before producing structured steps; it does not establish new mathematical rules. A separate bounded renderer checks multivariate division certificates and explains the final recomposition and reduced remainder, not unrecorded intermediate division steps or Gröbner uniqueness.

The rational-expression converter supports exact scalars, one variable, sums, products, and bounded integer powers. It preserves source-domain restrictions and feeds the existing quotient-rule and degree-two rational solver; see `docs/design/rational-expressions.md`.

The proof graph records expression inputs, typed condition nodes and rule evidence. Its verifier rejects invalid steps, unsupported conditions, forward dependencies and disconnected nodes. The derivation renderer emits checked steps for the supported expression, linear-equation and derivative evidence, including analytic derivative prerequisites. It does not establish new assumptions or prove general equations. `parse_domain_expression` and `parse_domain_statement` preserve the construction history of integer powers and supported analytic functions over the complex scalars; the `_in_domain` entrypoints select real or complex obligations. Legacy `parse_expression` and `parse_statement` return plain `Expr` values without restrictions discarded during normalization.

Partial polynomial root sets over C verify exact degree-one/two factor roots with multiplicity and preserve an explicit unresolved cofactor for all unsupported higher-degree root constructions. Completeness is asserted only when none remains; a factorization certificate alone does not imply all roots are constructible.

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
