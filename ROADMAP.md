# Roadmap

This file tracks implemented areas and the next domain extensions. See [CAPABILITIES.md](CAPABILITIES.md) for source locations and [the CAS architecture](docs/design/cas-platform.md) for mathematical invariants.

## Implemented

- Exact arbitrary-precision integers and rationals; immutable symbolic expressions with structural canonicalization.
- Unevaluated sine, cosine, tangent, exponential and logarithm nodes; nested real/complex domain-aware parsing and condition propagation.
- Verified analytic chain rules for sine, cosine, exponential, tangent and real logarithm, with explicit prerequisites.
- Five checked exact analytic constant identities with inherited domain restrictions and fixed rule provenance.
- Bounded expression/equation parser and Rust CLI.
- Univariate rational-polynomial arithmetic, long division, GCD, differentiation and verified square-free decomposition.
- Canonical bounded sparse multivariate polynomial arithmetic, partial differentiation, substitution and evaluation.
- Exact bounded conversion between `Expr` / `DomainExpression` and multivariate Q-polynomials in an explicit ambient ring, retaining historical domain conditions.
- Ordered exact multivariate division with independently verified quotient/remainder; no Gröbner normal-form claim.
- Bounded proof-checked final-result explanations for ordered multivariate division, without fabricated intermediate traces.
- Exact polynomial antiderivatives and definite integrals at rational endpoints, with independent verification.
- Verified exact rational-multiple primitives for sine, cosine and exponential of nonconstant rational-affine arguments.
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
- Exact bounded Sturm counts of distinct real roots in rational non-root intervals for square-free polynomials, with independently checked sign/Euclidean evidence.
- Exact partial complex polynomial zero sets, retaining unresolved higher-degree cofactors.
- Exact representation and verification of quadratic real irrational and complex roots.
- Formal symbolic derivatives for sums, products and positive integer powers.
- Typed scalar conditions, guarded reciprocal cancellation and composite definedness obligations.
- Proof graph for expression-equality steps, multiple condition dependencies and independent rule checks.
- Checked structured derivation rendering for supported proof graphs, linear equations and symbolic derivatives.
- Experimental Python bindings, platform wheel tests and source-distribution rebuild.

## In progress / next

1. Extend the bounded multivariate expression bridge to independently verified Buchberger reductions and Gröbner bases; expand exact algebra simplification without assuming arbitrary generators give unique normal forms.
2. Extend analytic domain contracts to safe symbolic rewriting, additional derivatives and explicit complex branch handling.
3. Extend nonlinear equations, inequalities and non-affine systems; proceed from Sturm counting toward certified comprehensive root isolation.
4. Add truthful per-reduction multivariate division traces and additional checked algebra/calculus rendering, then stable frontend-oriented serialization.
5. Extend the current affine sin/cos/exp antiderivative rules to broader elementary integration; implement limits and series with explicit supported domains and unresolved results.
6. Expand rational factorization beyond the bounded reconstruction path; improve algebraic embeddings and general arithmetic.
7. Add exact linear-algebra extensions, comprehensive real-root isolation and targeted symbolic matrix operations.
8. Stabilize the computation API, benchmark coverage and package releases for multiple frontends.

The implemented proof graph checks dependencies and supported rule evidence; it is not a general theorem prover.
