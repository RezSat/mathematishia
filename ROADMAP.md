# Roadmap

This file tracks implemented areas and the next domain extensions. See [CAPABILITIES.md](CAPABILITIES.md) for source locations and [the CAS architecture](docs/design/cas-platform.md) for mathematical invariants.

## Implemented

- Exact arbitrary-precision integers and rationals; immutable symbolic expressions with structural canonicalization.
- Unevaluated sine, cosine, tangent, exponential and logarithm nodes; explicit call parsing and direct domain-condition queries.
- Bounded expression/equation parser and Rust CLI.
- Univariate rational-polynomial arithmetic, long division, GCD, differentiation and verified square-free decomposition.
- Canonical bounded sparse multivariate polynomial arithmetic, partial differentiation, substitution and evaluation.
- Exact polynomial antiderivatives and definite integrals at rational endpoints, with independent verification.
- Extended Euclidean algorithm with verified polynomial Bézout certificates and modular inverses in Q[x].
- Canonical quotient-ring arithmetic in Q[x]/(m), including nonunits and signed powers.
- Exact rational matrices, rank, RREF, determinant, inverse and linear systems including affine solution families.
- Certified embedded quadratic fields with exact arithmetic, conjugation, trace, norm and independently checked minimal polynomials.
- Certified rational factors, including bounded quartic splitting and modular irreducibility proofs for higher degrees.
- Exact small-prime polynomial arithmetic, bounded finite-field factorization, coprime Hensel lifting and verified factor reconstruction. Unresolved cofactors remain explicit.
- Abstract irreducibility-certified extensions of degrees two through eight, with exact field arithmetic and checked minimal polynomials for supported elements.
- Domain-aware rational expression parsing, exact normalization over Q(x), differentiation, point evaluation and rational-root solving through degree two.
- Exact rational-linear, selected quadratic and rational-equation solving with domain exclusions.
- Exact multivariable affine equation systems with verified empty, unique and affine solution sets.
- Certified bounded rational real-root bracketing and independent endpoint verification.
- Exact representation and verification of quadratic real irrational and complex roots.
- Formal symbolic derivatives for sums, products and positive integer powers.
- Typed scalar conditions, guarded reciprocal cancellation and composite definedness obligations.
- Proof graph for expression-equality steps, multiple condition dependencies and independent rule checks.
- Checked structured derivation rendering for supported proof graphs, linear equations and symbolic derivatives.
- Experimental Python bindings, platform wheel tests and source-distribution rebuild.

## In progress / next

1. Extend multivariate polynomials with division, reduction and Expr conversion; connect them to a general algebra simplifier.
2. Add verified analytic differentiation rules and retain nested function-domain conditions through parsing and rewriting.
3. Extend nonlinear equation and inequality solving, polynomial roots and non-affine systems.
4. Extend verified derivation rendering to more algebra and calculus rules and stable frontend-oriented serialization.
5. Extend polynomial integration to elementary functions; implement limits and series with explicit supported domains and unresolved results.
6. Expand rational factorization beyond the bounded reconstruction path; improve algebraic embeddings and general arithmetic.
7. Add exact linear-algebra extensions, comprehensive real-root isolation and targeted symbolic matrix operations.
8. Stabilize the computation API, benchmark coverage and package releases for multiple frontends.

The implemented proof graph checks dependencies and supported rule evidence; it is not a general theorem prover.
