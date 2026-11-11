# Roadmap

This file tracks implemented areas and the next domain extensions. See [CAPABILITIES.md](CAPABILITIES.md) for source locations and [the CAS architecture](docs/design/cas-platform.md) for mathematical invariants.

## Implemented

- Exact arbitrary-precision integers and rationals; immutable symbolic expressions with structural canonicalization.
- Bounded expression/equation parser and Rust CLI.
- Univariate rational-polynomial arithmetic, long division, GCD, differentiation and verified square-free decomposition.
- Extended Euclidean algorithm with verified polynomial Bézout certificates and modular inverses in Q[x].
- Canonical quotient-ring arithmetic in Q[x]/(m), including nonunits and signed powers.
- Exact rational matrices, rank, RREF, determinant, inverse and linear systems including affine solution families.
- Certified embedded quadratic fields with exact arithmetic, conjugation, trace, norm and independently checked minimal polynomials.
- Certified rational factors, including bounded quartic splitting and modular irreducibility proofs for higher degrees.
- Exact small-prime polynomial arithmetic, bounded finite-field factorization, coprime Hensel lifting and verified factor reconstruction. Unresolved cofactors remain explicit.
- Abstract irreducibility-certified extensions of degrees two through eight, with exact field arithmetic and checked minimal polynomials for supported elements.
- Domain-aware rational expression parsing, exact normalization over Q(x), differentiation, point evaluation and rational-root solving through degree two.
- Exact rational-linear, selected quadratic and rational-equation solving with domain exclusions.
- Exact representation and verification of quadratic real irrational and complex roots.
- Formal symbolic derivatives for sums, products and positive integer powers.
- Typed scalar conditions, guarded reciprocal cancellation and composite definedness obligations.
- Proof graph for expression-equality steps, multiple condition dependencies and independent rule checks.
- Experimental Python bindings, platform wheel tests and source-distribution rebuild.

## In progress / next

1. Expand multivariate polynomial and rational-expression representations and connect them to a general algebra simplifier.
2. Introduce trigonometric, exponential and logarithmic expression nodes with domain conditions and verified differentiation rules.
3. Extend equation and inequality solving, including systems and selected polynomial roots beyond quadratic degree.
4. Implement rule-verified student-facing derivations and stable structured outputs for algebra and calculus.
5. Add elementary integration, limits and series with explicit supported domains and unresolved results.
6. Expand rational factorization beyond the bounded reconstruction path; improve algebraic embeddings and general arithmetic.
7. Add exact linear-algebra extensions, certified numerics and targeted symbolic matrix operations.
8. Stabilize the computation API, benchmark coverage and package releases for multiple frontends.

The implemented proof graph checks dependencies and supported rule evidence; it is not a general theorem prover.
