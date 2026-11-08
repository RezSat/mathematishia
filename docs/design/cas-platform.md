# Mathematishia: toward a full, explainable CAS

## North star

Build an open-source exact symbolic mathematics engine with structured
mathematical provenance. The target is broad SymPy-like functionality
together with the quality of guided steps users associate with education
products. Mathematishia is not claiming existing parity with SymPy,
Symbolab or Mathematica. Those are demanding product-level goals.

A count of commits, lines, or generated features is not a measure of
mathematical correctness or usefulness. The unit of progress is a verified
vertical slice: a documented domain contract, typed results, one or more
mathematical algorithms, invalid-domain behavior, independent checkers
where feasible, Rust/CLI/Python integration, adversarial and regression
tests, and passing CI.

## Existing components

- mathema: arbitrary-precision integer/rational values, symbols, immutable
  expression IR, semantic views and canonical scalar constructors.
- kernel: exact polynomial arithmetic, division/GCD, square-free
  multiplicities, rational partial functions with explicit holes, selected
  linear and quadratic solvers, derivation records and independent checks.
- Algebraic roots: exact degree-two real irrational and complex conjugate
  values through a monic minimal polynomial and a branch selector; ordered
  typed normalization/discriminant evidence. Not a general number field.
- frontend and CLI: bounded parser, polynomial/solver commands and proof
  data presentation. Full rational-expression syntax and broad functions
  are not implemented.
- Python: experimental maturin/PyO3 bindings, immutable results and
  installed-wheel smoke tests.

## Immutable engineering contracts

1. Domain semantics before algorithms. Never divide by a symbolic
   denominator without retaining the necessary nonzero condition. Reduced
   rational functions remain partial functions with original exclusions.
2. Typed evidence before narratives. Every mathematical step needs a stable
   identifier, a relation, and all necessary exact side conditions.
   Classification and differentiation are not automatically equality steps.
3. Independent verification where practical. A verifier must reconstruct
   results without trusting the generator or sampling floating-point values.
4. Determinism. Exact inputs produce the same canonical values, proof order,
   error categories and stable rule IDs across runs.
5. No silent scope escalation. If a domain is unsupported, return an
   explicit error or partial/conditional result, never a guessed conclusion.
6. Metering is not sandboxing. Exact arithmetic can allocate costly large
   integers. Hard isolation needs separate process/time/memory controls.
7. Rust owns the mathematics; Python, CLI and future web UI are adapters.
8. No AI in the verified mathematical kernel. AI may help create code and
   user explanations, but may not change the mathematical verdict.

## Next engineering phases, in dependency order

### A. Exact domains and mathematical conditions

- Finish a minimal algebraic-number arithmetic domain, with canonical
  embeddings, exact equality, field operations and comparisons where
  mathematically defined. Degree-two roots are only the starting point.
- Introduce explicit assumptions: nonzero, real, positive, integer, and
  domain membership. Separate proven conditions from unknown obligations.
- Add safe expression division/powers on top of those conditions and
  rational partial-function semantics.
- Gate: exact special cases and adversarial cancellation tests agree
  with independent exact-domain oracles.

### B. Structured proof graph and step checker

- Model proof nodes, parent references, input/output mathematical types,
  relation kind, rule ID, assumptions consumed, new obligations, and exact
  witnesses. Store semantic values, never prose as a proof.
- Support deterministic linearization into concise or detailed explanations.
  A user should be able to ask why a step is legal and see its conditions.
- Add conditional rewriting, repeatability controls, proof budget and
  completeness status. Reject disconnected or forged DAG edges.
- Gate: independent checkers validate every emitted step, including
  nonzero obligations; formatting is not required for proof verification.

### C. Polynomial and equation platform

- Build exact factorization over Q[x], including rational-root factoring and
  irreducible factor extraction; then multivariate polynomial domains.
- Implement elimination, resultants, rational functions, partial fractions
  and equations beyond quadratic with explicit root domains.
- Reuse the canonical algebraic value domain instead of duplicating root
  representations for each solver.
- Gate: identities recomposed exactly, all roots checked when completeness
  is claimed, reproducible tests against established math systems.

### D. Analytic functions and calculus

- Add typed function nodes (sin, cos, exp, log, abs, etc.) with proper
  real/complex domains and branch cuts, not ad-hoc Symbol names.
- Define differentiation, limits and integration with guarded rules;
  represent unevaluated integrals when unsupported.
- General simplification must be assumption-sensitive (for example
  sqrt(x^2) is not x without appropriate assumptions).
- Gate: verified rewrites, branch/domain regression corpus and
  independently checked transformations within supported scopes.

### E. Numeric computation with honest accuracy

- Arbitrary-precision approximations need explicit precision, rounding
  semantics, and error estimates; separate approximations from exact values.
- Eventually support certified intervals, root isolation, matrix/tensor
  numerical operations and proven error bounds.
- Gate: reference comparisons, error-bound tests, reproducible performance
  measurements and resource isolation.

### F. Explanations, UX and distribution

- Render the proof graph as a pedagogical explanation with selectable detail,
  math notation, LaTeX/MathML, domain warnings and accessibility support.
- Create an explainable problem corpus; score both mathematical correctness
  and pedagogical clarity separately. Cover gradeschool-to-university
  progressions without conflating a plausible explanation with a proof.
- Stabilize a versioned JSON/structured client contract and Python API.
- Publish signed, reproducible platform wheels and source distributions
  to PyPI only after release gates and trusted-publisher setup.

## Evidence and evaluation plan

Every new feature gets a coverage matrix: valid input, invalid input,
boundary/limit behavior, domain restrictions, canonicalization,
determinism, and forged-evidence rejection. Maintain a growing golden corpus
of steps and solution classifications. Cross-check selected outputs against
SymPy or other mature systems in a separate test harness, with exact rational
or symbolic comparisons when possible; a third-party system is a test
oracle, not a runtime dependency or a license to suppress disagreements.

Measure: supported operation/domain coverage; passing evidence checks;
counterexample count; reproducible error classifications; test-suite
completion on Linux, Windows and macOS; Python installed-wheel health;
and benchmark regressions on recorded workloads. Track capability milestones
on the roadmap, not vanity commit totals.

## Release gates

- All supported platform checks, strict lint and tests pass for the tag.
- User-facing examples match the released version; unsupported domains
  have explicit errors or precise partial-result semantics.
- Stable packaged API, changelog and compatibility policy are documented.
- Built wheel and source distribution install cleanly without the repository.
- PyPI account, name availability, GitHub environment protection and
  Trusted Publishing are configured by the maintainer.
- Release upload is manual/approved, never performed on an ordinary push.

See the separate PyPI checklist for the release process.
