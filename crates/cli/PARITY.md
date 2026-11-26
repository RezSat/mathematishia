# CLI and Python capability parity

This table describes the checked mathematical scope, not equivalence of every
input grammar or presentation. The CLI uses text results; Python returns typed
values. Neither frontend substitutes numerical guesses for exact answers.

| Mathematical capability | CLI | Python | Checked scope / remaining gap |
| --- | --- | --- | --- |
| Expression and domain parsing, substitution, guarded rewrite | `expr`, `domain`, `subdomain`, `rewrite` | `expression`, `domain_expression`, `domain_equation`, `substitute_domain`, `rewrite_domain` | Exact rational scalars, supported integer powers and analytic calls, retained obligations in domain-aware paths; not general simplification or theorem proving. |
| Distributive steps and polynomial expansion | `expand`, `expandall` | `expand`, `expandall`, `expand_polynomial` | Bounded distributive steps versus complete checked Q[variables] expansion; explicit or inferred ambient ring, positive integer powers and source conditions retained. |
| Linear and quadratic equations | `solve`, `solveq`, `roots2` | `solve`, `solve_quadratic`, `algebraic_roots` | Linear rational solutions; rational quadratic classification or exact quadratic algebraic roots over C. No general symbolic equation solving. |
| Automatic equation dispatch and partial roots | `solveauto`, `roots` | `solve_polynomial_equation`, `polynomial_roots`, `rational_solve` | Unrestricted Q[x] complex roots with multiplicity and unresolved cofactor; rational-function roots over Q with exclusions. Python's separate APIs do not yet reproduce automatic dispatch and its partial-over-C status. |
| Sturm root counts | `sturm` | `sturm_root_count` | Nonzero square-free unrestricted Q[x], distinct real roots strictly between finite ordered rational non-root endpoints; degree limit 32. Source conditions refused, including in scalar endpoints. Python endpoint/identifier admission must match the domain-aware check. |
| Rational root-bracket refinement | `rootbracket` | Pending `refine_real_root_bracket` | Nonzero unrestricted Q[x], degree limit 32; exact rational root, sign-change existence bracket, or unresolved equal signs. Width and bisection count are explicit; no uniqueness or all-root coverage. |
| Explicit affine equation systems | `linsystem` | Pending `solve_linear_system` | Complete empty, unique or affine-family solutions over Q, up to 16 equations and 16 distinct variables, explicit variable order; source restrictions refused. |
| Ordered multivariate polynomial division | `mvdivide` | Pending `divide_multivariate` | Q[variables], sorted-name lex order, caller divisor order, max 16 variables/32 divisors, degree 128/2048 distinct terms under default limits; independently checked recomposition and reduced remainder, not a unique normal form. |
| Univariate polynomial arithmetic | `poly`, `eval`, `divide`, `gcd`, `egcd`, `diff` | `polynomial`, `evaluate`, `divide`, `gcd`, `extended_gcd`, `differentiate` | Exact Q[x] arithmetic and degree/work limits; legacy plain-Expr operations assume polynomial inputs, not arbitrary rational expressions with canceled restrictions. |
| Polynomial square-free and rational factorization | `sqfree`, `factor` | `square_free`, `factor_polynomial` | Exact rational factors and multiplicities, explicit unresolved cofactors; not complete arbitrary factorization. |
| Polynomial residues and modular inverses | `modreduce`, `modop`, `modpow`, `modinv` | `modular_reduce`, `modular_operation`, `modular_power`, `modular_inverse` | Exact Q[x]/(m); modulus need not be irreducible. Inverse only when checked invertibility holds. |
| Embedded quadratic fields | `qvalue`, `qop`, `qinv` | `quadratic_value`, `quadratic_operation`, `quadratic_inverse` | Explicit defining quadratic, selected branch, exact rational field arithmetic. |
| Certified abstract extensions | `fieldop`, `fieldpow`, `fieldmin` | `extension_value`, `extension_operation`, `extension_power`, `extension_minimal_polynomial` | Certified irreducible defining polynomials degree 2 to 8; exact field arithmetic and minimal polynomial. Value construction is available as addition of zero; no separate CLI constructor needed. No embedding or cross-field arithmetic. |
| Exact matrix operations | `matdet`, `matinv`, `matrref`, `matmul`, `matsolve` | `matrix_determinant`, `matrix_inverse`, `matrix_rref`, `matrix_multiply`, `matrix_solve` | Rational matrices up to 16 by 16, exact rank, inverse and affine solutions. No eigenvalue or symbolic matrix support. |
| Rational functions | `rat`, `rateval`, `ratdiff`, `ratop`, `ratsolveq`, `rsimplify`, `rdiff`, `reval`, `rsolve` | `rational_function`, `rational_evaluate`, `rational_differentiate`, `rational_operation`, `rational_solve_quadratic`, `rational_simplify`, `rational_derivative`, `rational_value`, `rational_solve` | Bounded Q(x), original denominator exclusions in domain-aware source paths; quadratic residual rational roots only. |
| Symbolic differentiation | `diffexpr`, `diffreal` | `differentiate_expression`, `differentiate_expression_real` | Verified polynomial, product and supported analytic chain rules; real log/tan obligations; complex principal log branch cut unsupported. |
| Integration | `integrate` | `integrate`, `integrate_expression_dispatch`, `integrate_polynomial`, `integrate_polynomial_between`, `integrate_elementary` | Zero-constant primitive for bounded polynomial and supported rational-affine sin/cos/exp sums; definite polynomial integral at exact rational bounds, without inherited restrictions. No general analytic integration. |
| Analytic constants | `evalexact` | `analytic_constant` | Five checked identities sin(0), cos(0), tan(0), exp(0), log(1); CLI applies up to 16 rewrites and reports partial when unresolved. Python one-step interface needs matched bounded dispatch. |
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
- Comprehensive real-root isolation, Gröbner bases, general algebraic
  solutions, general analytic calculus, symbolic matrices and numerical
  analytic evaluation are not provided by these commands.

## Delivery dependencies

The legacy text adapters are direct checked kernel calls. The shared
`ProblemResult` dispatcher and deterministic versioned JSON are a separate
integration dependency; no alternative serializer is supplied here. Matching
Python adapters for the three newly surfaced kernel operations, alignment of
Sturm input admission, and synchronized integration-level documentation and
CI are required before repository-wide delivery parity can be claimed.
