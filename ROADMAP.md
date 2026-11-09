# Roadmap

This file tracks implemented areas and the next domain extensions. See [CAPABILITIES.md](CAPABILITIES.md) for source locations and [the CAS architecture](docs/design/cas-platform.md) for mathematical invariants.

## Implemented

- Exact arbitrary-precision integers and rationals; immutable symbolic expressions with structural canonicalization.
- Bounded expression/equation parser and Rust CLI.
- Univariate rational-polynomial arithmetic, long division, GCD, differentiation and verified square-free decomposition.
- Extended Euclidean algorithm with verified polynomial Bézout certificates and modular inverses in Q[x].
- Canonical quotient-ring arithmetic in Q[x]/(m), including nonunits and signed powers.
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

1. Provide verified arithmetic traces for general rational-expression conversions and extend proof obligations to equation transformations and domain-specific evidence.
2. Extend conditional rewriting to wider expression classes and analytic branch semantics.
3. Expand rational factorization beyond the current bounded monic reconstruction path, including arbitrary primitive integer polynomials, better modular splitting, general Hensel recombination and coefficient bounds.
4. Extend algebraic fields beyond degree eight and implement compatible embeddings, field composita and cross-field arithmetic.
5. Add analytic functions, broader differentiation and integration.
6. Add numerical precision policies and certified approximations.
7. Build explanatory renderers and stabilize external data formats.
8. Publish Python distributions after versioning and release checks.

The implemented proof graph checks dependencies and supported rule evidence; it is not a general theorem prover.
