# CLI and Python capability parity

This table describes the checked mathematical scope, not equivalence of every
input grammar or presentation. The CLI uses text results; Python returns typed
values. Neither frontend substitutes numerical guesses for exact answers.

| Mathematical capability | CLI | Python | Checked scope / remaining gap |
| --- | --- | --- | --- |
| Expression and domain parsing, substitution, guarded rewrite | `expr`, `domain`, `subdomain`, `rewrite` | `expression`, `domain_expression`, `domain_equation`, `substitute_domain`, `rewrite_domain` | Exact rational scalars, supported integer powers and analytic calls, retained obligations in domain-aware paths; not general simplification or theorem proving. |
| Distributive steps and polynomial expansion | `expand`, `expandall` | `expand`, `expandall`, `expand_polynomial` | Bounded distributive steps versus complete checked Q[variables] expansion; explicit or inferred ambient ring, positive integer powers and source conditions retained. |
| Linear and quadratic equations | `solve`, `solveq`, `roots2` | `solve`, `solve_quadratic`, `algebraic_roots` | Linear rational solutions; rational quadratic classification or exact quadratic algebraic roots over C. No general symbolic equation solving. |
| Automatic equation dispatch and partial roots | `solveauto`, `roots` | `solve_polynomial_equation`, `polynomial_roots`, `rational_solve` | Unrestricted Q[x] complex roots with multiplicity and unresolved cofactor; rational-function roots over Q with exclusions. The versioned result is shared through CLI `--json` and Python `solveauto` / `problem`; legacy text preserves its existing scope. |
| Sturm root counts | `sturm` | `sturm_root_count` | Nonzero square-free unrestricted Q[x], distinct real roots strictly between finite ordered rational non-root endpoints; degree limit 32. Source conditions refused, including in scalar endpoints. Both interfaces check endpoint and identifier admission. |
| Complete real-root isolation | `isolate` | `isolate_real_roots` | Nonzero square-free unrestricted Q[x]; disjoint open dyadic brackets, exactly one distinct root per bracket and proven global coverage; positive rational target width and exact midpoint error bound. |
| Rational root-bracket refinement | `rootbracket` | `refine_real_root_bracket` | Nonzero unrestricted Q[x], degree limit 32; exact rational root, sign-change existence bracket, or unresolved equal signs. Width and bisection count are explicit; no uniqueness or all-root coverage. |
| Explicit affine equation systems | `linsystem` | `solve_linear_system` | Complete empty, unique or affine-family solutions over Q, up to 16 equations and 16 distinct variables, explicit variable order; source restrictions refused. |
| Ordered multivariate polynomial division | `mvdivide` | `divide_multivariate` | Q[variables], sorted-name lex order, caller divisor order, max 16 variables/32 divisors, degree 128/2048 distinct terms under default limits; independently checked recomposition and reduced remainder, not a unique normal form. |
| Univariate polynomial arithmetic | `poly`, `eval`, `divide`, `gcd`, `egcd`, `diff` | `polynomial`, `evaluate`, `divide`, `gcd`, `extended_gcd`, `differentiate` | Exact Q[x] arithmetic and degree/work limits; legacy plain-Expr operations assume polynomial inputs, not arbitrary rational expressions with canceled restrictions. |
| Polynomial square-free and rational factorization | `sqfree`, `factor` | `square_free`, `factor_polynomial` | Exact rational factors and multiplicities, explicit unresolved cofactors; not complete arbitrary factorization. |
| Polynomial residues and modular inverses | `modreduce`, `modop`, `modpow`, `modinv` | `modular_reduce`, `modular_operation`, `modular_power`, `modular_inverse` | Exact Q[x]/(m); modulus need not be irreducible. Inverse only when checked invertibility holds. |
| Embedded quadratic fields | `qvalue`, `qop`, `qinv` | `quadratic_value`, `quadratic_operation`, `quadratic_inverse` | Explicit defining quadratic, selected branch, exact rational field arithmetic. |
| Certified abstract extensions | `fieldop`, `fieldpow`, `fieldmin` | `extension_value`, `extension_operation`, `extension_power`, `extension_minimal_polynomial` | Certified irreducible defining polynomials degree 2 to 8; exact field arithmetic and minimal polynomial. Value construction is available as addition of zero; no separate CLI constructor needed. No embedding or cross-field arithmetic. |
| Exact matrix operations | `matdet`, `matinv`, `matrref`, `matmul`, `matsolve` | `matrix_determinant`, `matrix_inverse`, `matrix_rref`, `matrix_multiply`, `matrix_solve` | Rational matrices up to 16 by 16, exact rank, inverse and affine solutions. No eigenvalue or symbolic matrix support. |
| Rational functions | `rat`, `rateval`, `ratdiff`, `ratop`, `ratsolveq`, `rsimplify`, `rdiff`, `reval`, `rsolve` | `rational_function`, `rational_evaluate`, `rational_differentiate`, `rational_operation`, `rational_solve_quadratic`, `rational_simplify`, `rational_derivative`, `rational_value`, `rational_solve` | Bounded Q(x), original denominator exclusions in domain-aware source paths; quadratic residual rational roots only. |
| Symbolic differentiation | `diffexpr`, `diffreal` | `differentiate_expression`, `differentiate_expression_real` | Verified polynomial, product and supported analytic chain rules; real log/tan obligations; complex principal log branch cut unsupported. |
| Integration | `integrate` | `integrate`, `integrate_expression_dispatch`, `integrate_polynomial`, `integrate_polynomial_between`, `integrate_elementary` | Zero-constant primitive for bounded polynomial, rational-affine sin/cos/exp sums and polynomial times affine exponential; definite polynomial integral at exact rational bounds, without inherited restrictions. No general analytic integration. |
| Analytic constants | `evalexact` | `analytic_constant` | Five checked identities sin(0), cos(0), tan(0), exp(0), log(1); CLI applies up to 16 rewrites and reports partial when unresolved. Python also exposes `evalexact` through the shared dispatcher. |
| Real finite-point limits | `limit` | `limit` | Exact Q(x) at finite rational point, left/right/two-sided, original exclusions; finite values, signed infinities, nonexistence. No analytic limits or limits at infinity. |
| Real inequalities | `inequality`, `solve`, `solveauto` | `solve_inequality` | Single comparison of Q(x) expressions, numerator/denominator/restriction polynomials degree at most 2, exact rational or quadratic-algebraic interval endpoints; poles and holes excluded. |

## Operations without a standalone public command

The following are not claims of frontend coverage.

- Small-prime field arithmetic, modular irreducibility/factorization and
  Hensel reconstruction support certified rational factorization. A standalone
  finite-field calculator would need explicit field/modulus admission and a
  matched result contract; it is outside historical CLI/Python parity.
- Sparse multivariate arithmetic, substitution, differentiation and reverse
  expression conversion support expansion and ordered division. They remain
  kernel building blocks; a general sparse-ring calculator is not yet a
  matched frontend contract.
- Proof graphs, proof verifiers, assumption primitives, derivation records
  and certificate reconstruction are internal evidence mechanisms. They need
  not each become commands. New structured proof delivery is a separate scope.
- Root isolation of repeated factors, larger Gröbner ideals, general algebraic
  solutions, general analytic calculus, symbolic matrices and numerical
  analytic evaluation are not provided by these commands.

## Shared results

The CLI and Python share a kernel-owned versioned result for selected
operations. Append `--json` to a supported command or use Python
`problem(*args)` with the same arguments (without the flag). Legacy text and
typed APIs remain supported; the versioned schema does not cover every legacy
operation. See [schema v1](../kernel/src/problem_result/README.md).

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
rewrite paths. Legacy unbound expressions cannot attest a lost construction
domain. The ordinary symbol pi is not silently reinterpreted. Opt-in natural input
supports selected special angles and total affine absolute-value equations. General periodic solution families, arbitrary piecewise
or absolute-value expressions, general analytic calculus and universal result
serialization remain unsupported.
