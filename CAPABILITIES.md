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
| Complete certified real-root isolation for unrestricted nonzero square-free Q[x] | `crates/kernel/src/real_root_isolation.rs` |
| Source-bound rational equation solution-set certificates | `crates/kernel/src/solution_certificates.rs` |
| Checked symbolic domain preconditions and retained restrictions | `crates/kernel/src/analytic_semantics.rs` |
| Exact quadratic roots | `crates/kernel/src/algebraic.rs` |
| Embedded quadratic fields, arithmetic, trace, norm and minimal-polynomial checks | `crates/kernel/src/quadratic_field.rs` |
| Certified abstract extensions (degree 2–8) | `crates/kernel/src/algebraic_extension.rs` |
| Exact minimal polynomials of extension elements | `crates/kernel/src/extension_minimal.rs` |
| Verified symbolic differentiation, including analytic chain rules | `crates/kernel/src/differentiate_expr.rs`, `context.rs` |
| Certified Gröbner bases and ideal membership | `crates/kernel/src/groebner.rs` |
| Exact real affine absolute-value equations | `crates/kernel/src/absolute_equations.rs` |
| Checked special-angle trigonometry | `crates/kernel/src/trigonometry.rs` |
| Domain-selected polynomial equations | `crates/kernel/src/polynomial_equation_dispatch.rs` |
| Rational calculus identity checks | `crates/kernel/src/calculus_rule_checks.rs` |
| Checked structural step records | `crates/kernel/src/step_records.rs` |
| Bounded exact analytic-constant rewrites with checked domain provenance | `crates/kernel/src/analytic_constants.rs` |
| Checked, typed problem outcomes and versioned JSON | `crates/kernel/src/problem_result.rs`, `crates/kernel/src/problem_result/wire.rs` |
| Shared text-to-request adapter | `crates/frontend/src/problem.rs` |
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

The parser now accepts exact finite decimals and selected implicit multiplication, including `2x`, `2(x+1)`, and `(x+1)(x-1)`. A separate real-comparison parser accepts one `<`, `<=`, `>`, `>=`, `≤` or `≥` while preserving both operands' domains. The general expression parser still refuses chained comparisons, ambiguous identifier juxtaposition, `abs`, and `sqrt`; the dedicated absolute-value equation adapter accepts bounded affine chains.

The Rust kernel now has independently checked positive-integer polynomial expansion, an integration dispatcher for bounded sums of polynomial and supported affine analytic terms, and a public typed result contract that distinguishes exact, conditional, partial, unsupported and undefined cases. The existing CLI `expand` command remains distributive-step-only; `expandall` exposes complete bounded polynomial expansion. `solveauto`, `roots`, `integrate`, and `evalexact` expose selected checked operations, including polynomial and rational-affine sine/cosine/exponential integration sums. Python exposes typed counterparts for expansion and combined integration, plus root sets, Sturm counts and analytic constants. Both new expansion/integration interfaces preserve conditional results. The shared versioned result is available through CLI `--json` on `solveauto`, `roots`, `expandall`, indefinite `integrate`, `limit`, `inequality`, `evalexact`, `diffreal` and `diffexpr`, and Python `problem(*args)`, `solveauto` and `evalexact`. Exact rational values are encoded as integer strings; algebraic values retain minimal polynomials and branches. Legacy text commands and typed Python APIs remain available. Not every legacy operation has a schema v1 representation.

The kernel computes finite rational-point limits of univariate rational functions, with exact factor orders, signed one-sided infinities, two-sided nonexistence and retained source exclusions. Real inequalities compare a checked rational function against zero; numerator, denominator and inherited restriction polynomials must each have degree at most two. Exact rational and certified quadratic-algebraic endpoints form canonical interval unions, with poles and removable holes excluded. Both operations are exposed through CLI `limit` and `inequality` (also `solve` / `solveauto` comparisons) and Python `limit` and `solve_inequality`. They preserve source exclusions; limit answers do not fill holes, and inequality intervals include only admissible points. Transcendental limits and higher-degree inequalities are not claimed.

The compiled CLI corpus contains 170 cases: 116 solved, 11 partial, 26 unsupported, and 17 explicit domain, input or resource refusals. These are regression sample counts, not universal mathematical coverage.


Analytic calls support `sin`, `cos`, `tan`, `exp` and natural `log` as unevaluated expressions. The domain-aware parser preserves restrictions through nested calls and structural normalization, with real or complex domain selection. The derivative verifier supports chain rules for sine, cosine and exponential. Tangent requires `cos(argument) != 0`; real logarithm requires a positive argument. Complex principal-log derivatives are not admitted because the current domain model does not encode its branch cut. Conditions remain unproved prerequisites, not claims of validity. CLI `diffreal` and Python `differentiate_expression_real` expose the checked real-domain result. The existing polynomial converter rejects function calls. A separate bounded one-step rewrite supports exactly `sin(0)=0`, `cos(0)=1`, `tan(0)=0`, `exp(0)=1` and `log(1)=0`, with independent rule checking and preserved input domain conditions; this is not arbitrary analytic evaluation.

Sparse multivariate polynomials store exact rational coefficients in a fixed, explicitly named ambient ring. Arithmetic, partial differentiation, evaluation and substitution have bounded input and work limits. Ordered lexicographic division returns a quotient for each divisor and a remainder, checked against the original polynomial by exact recomposition and leading-term nondivisibility. Divisor order affects results. No unique normal form is claimed for arbitrary divisors; bounded independently checked Gröbner bases establish formal ideal membership. An explicit bounded `Expr` ↔ multivariate bridge now handles exact scalar and declared-symbol sums, products and strictly positive integer powers in a caller-selected ring, with inherited `DomainExpression` conditions retained. Zero, negative, noninteger and symbolic powers, undeclared symbols and function calls are rejected. The reverse `Expr` conversion preserves polynomial value, not original syntax.

Exact polynomial integration returns zero-constant primitives or definite rational values at rational bounds. Primitives and integral values are independently checked. A separate elementary integrator returns exact rational-multiple primitives for only `sin(ax+b)`, `cos(ax+b)` and `exp(ax+b)` with nonzero rational slope, retained source conditions and independently verified derivative evidence. Bounded polynomial times affine exponential integration additionally uses exact polynomial identity verification, exposed by the same dispatcher. General transcendental integration remains unsupported. Multivariable affine equation solving classifies empty, unique and affine-family solution sets, restricted to 16 equations and 16 variables.

Exact rational root-bracket refinement certifies a rational root or existence within an opposite-sign interval; same-sign intervals are unresolved. An exact bounded Sturm certificate now counts distinct real roots of square-free nonzero Q[x] inside rational intervals with non-root endpoints; repeated factors and root endpoints are explicitly rejected in this first increment. Neither of those operations isolates all roots. The separate `isolate` command and Python `isolate_real_roots` return sorted, strictly disjoint dyadic brackets containing exactly one root each, with certified global coverage and exact midpoint error bounds, for unrestricted nonzero square-free Q[x]. Repeated factors and retained restrictions are refused. CLI `sturm`, `rootbracket`, `linsystem` and `mvdivide` have matched Python APIs. The derivation renderer checks supplied proof evidence before producing structured steps; it does not establish new mathematical rules. A separate bounded renderer checks multivariate division certificates and explains the final recomposition and reduced remainder, not unrecorded intermediate division steps or Gröbner uniqueness.

The source-bound solution certificate independently verifies exact rational or quadratic roots against both original rational-equation operands and checks multiplicities and unresolved cofactors. It does not perform general factor search. The semantic precondition report assesses the current real/complex scalar grammar and historical obligations; it is not a domain-conversion or equivalence certificate, and the caller must supply the original construction domain.

The rational-expression converter supports exact scalars, one variable, sums, products, and bounded integer powers. It preserves source-domain restrictions and feeds the existing quotient-rule and degree-two rational solver; see `docs/design/rational-expressions.md`.

The proof graph records expression inputs, typed condition nodes and rule evidence. Its verifier rejects invalid steps, unsupported conditions, forward dependencies and disconnected nodes. The derivation renderer emits checked steps for the supported expression, linear-equation and derivative evidence, including analytic derivative prerequisites. It does not establish new assumptions or prove general equations. `parse_domain_expression` and `parse_domain_statement` preserve the construction history of integer powers and supported analytic functions over the complex scalars; the `_in_domain` entrypoints select real or complex obligations. Legacy `parse_expression` and `parse_statement` return plain `Expr` values without restrictions discarded during normalization.

Partial polynomial root sets over C verify exact degree-one/two factor roots with multiplicity and preserve an explicit unresolved cofactor for all unsupported higher-degree root constructions. Completeness is asserted only when none remains; a factorization certificate alone does not imply all roots are constructible.

Rational factorization first removes repeated factors. It uses the rational root theorem, exact quadratic discriminants, exhaustive bounded monic quartic splitting, and finite-field irreducibility certificates. Supported monic integral polynomials through degree eight can also be split by square-free modular factorization, coprime Hensel lifts and exact division. None of these bounds imply complete factorization for arbitrary inputs. When no factor or irreducibility proof is found, the remainder is unresolved.

Small-prime polynomial arithmetic admits primes through 251 and input degrees through 32. The finite-field factorizer uses capped divisor enumeration through degree eight. Hensel reconstruction is restricted to bounded monic integer polynomials and a fixed small-prime search.

Exact rational matrices permit up to 16 rows and 16 columns. Inverse candidates are checked against both multiplication identities. Solvers distinguish unique, inconsistent and infinite families. This is exact linear algebra over Q; eigenvalues, eigenvectors, symbolic matrices and floating-point matrix algorithms remain outside the implemented scope.

The quotient-ring type does not assume irreducibility. Embedded quadratic fields use explicit root branches. Abstract algebraic fields accept irreducibility-certified defining polynomials of degree two through eight but do not have selected complex embeddings. General factorization, arithmetic across distinct field presentations, analytic numerical evaluation, general analytic integration, isolation with repeated factors or source restrictions, and general theorem proving remain outside the supported scope.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Update this index when a module is added or its supported scope changes.

## Checked algebra, angles and derivations

| CLI | Python | Supported mathematical scope |
| --- | --- | --- |
| `rootsfield` | `solve_polynomial_in_domain` | Unrestricted Q[x], explicit Q/R/C solution domain; checked factors and unresolved cofactors. |
| `groebner`, `idealmember` | `groebner_basis`, `ideal_membership` | Formal rational polynomial ideals; at most 3 variables, 8 generators and 12 basis elements. Lexicographic, not necessarily reduced or monic. |
| `trig`, `trigzeros` | `special_angle`, `trig_zero_examples` | Exact rational multiples of pi, supported unit-circle angles; tangent poles undefined, sine/cosine zero examples partial. |
| `abssolve` | `solve_absolute` | One total real affine absolute-value chain per side, at most 8 layers total; complete closed interval unions. |
| `checkcalculus` | `check_calculus` | Verify rational identities or derivatives with retained exclusions; candidate must cover the source domain. |
| `steps` | `derivation_records`, `derivation_records_json` | Checked linear, derivative and distribution records; source restrictions and actual dependencies. |
| `divisiontrace` | `division_trace_json` | Actual ordered polynomial reduction trace, independently verified against the source. |
| `solvefield` | `problem('solvefield', ...)` | Source-bound rational equations over Q/R/C, complete through quadratic residuals; higher-degree unresolved cofactors remain partial. |
| `natural` | `problem('natural', ...)` | Opt-in special angles and affine absolute-value equations; radicals not yet admitted. |

`rootsfield`, `solvefield`, `natural`, `groebner`, `idealmember`,
`trig`, `trigzeros` and `abssolve` expose the shared checked result schema
through CLI JSON and Python `problem(*args)`. Existing text/typed interfaces
remain available. `steps --json` and `divisiontrace` expose separately
verified version-1 trace envelopes; the legacy division result is still final-only.
The ordered trace records actual reductions and remainder moves over unrestricted
Q-polynomials (at most 8 variables/divisors, degree 32, 64 terms and 128 steps).
It does not claim a unique normal form for arbitrary divisors.

Explicit construction-domain provenance is retained on bound values and
checked in the shared dispatcher and selected calculus, substitution and
rewrite paths. Strict source-bound provenance rejects external or unbound history; legacy unbound expressions cannot attest a lost construction domain. Cancellation requires checked source conditions. The ordinary symbol pi is not silently reinterpreted. Opt-in natural input
supports selected special angles and total affine absolute-value equations. Unrestricted periodic solution families, arbitrary piecewise
or absolute-value expressions, general analytic calculus and universal result
serialization remain unsupported; the bounded school routes below extend this scope.

## Wave 2: opt-in checked school strategies

The `school` CLI and Python `school(*args)` / `school_json(*args)` expose the following bounded native algorithms. Existing command contracts and limits are unchanged. Source construction is explicitly bound before normalization; retained conditions are not discarded.

| Operation | Checked scope | Evidence |
| --- | --- | --- |
| `school sqrt` | Real principal roots of rational constants, exact quadratic irrational roots, and syntactic squares | Exact branch / absolute-value result; not a general radical simplifier |
| `school solve` | Selected repeated-power substitution and school polynomial factorizations over Q/R/C, with legacy dispatch otherwise | Factor or substitution certificate, root completeness or unresolved factor |
| `school inequality` | Rational inequalities whose critical factors are certified linear/quadratic factors | Ordered critical points and exact sign chart |
| `school differentiate` | Source-bound rational derivatives | Quotient-rule and simplification certificates |
| `school integrate` | Polynomial times affine sine/cosine, and polynomial substitution into sine/cosine/exponential | Native parts reductions or substitution certificate |
| `school limit` | Selected standard analytic zero limits and rational limits at signed infinity | Exact theorem/leading-term evidence |
| `school periodic` | Real affine sine/cosine/tangent equations with exact supported special-angle targets | Periodic families with integer parameter and pole exclusions |
| `school trig_identity` | Linear combinations of the supported squared-angle identities | Verified identity certificate |
| `school abs_sum`, `school piecewise` | Total real weighted affine absolute sums and disjoint exhaustive rational affine guards | Partition and affine case checks |
| `school methods` | Opt-in checked second method for repeated-root complex quadratics | Primary and distinct verified linear-factor alternative |

`naturalv2` is additive, and `school natural` reuses its validated syntax with bound source admission. Explicit school routes support operations not admitted by natural input. See [CLI guide](crates/cli/USER_GUIDE.md) and [Python guide](crates/python/README.md). Results contain checked mathematical evidence; not every operation emits a full pedagogical trace. General radicals, variable powers, logarithmic primitives, unrestricted periodic and nonlinear piecewise equations, and broad applied-mathematics solving remain outside the delivered scope.

The public W18 breadth corpus has 292 independently keyed valid cases plus four invalid-input checks. The W19 independent suite has 126 held-out cases and adversarial kernel tests. These are fixed regression suites, not representative statistical estimates of all school questions. Formula-only cases do not establish native mechanics, statistics or geometry solvers.
