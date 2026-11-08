# Roadmap

These phases describe architectural goals, not dates.

0. Repository and architectural foundation: establish boundaries, policies, and
   repeatable checks.
1. Exact scalar semantic core: define Integer, Rational, Symbol, and the first
   scalar expression forms with tested semantics. Integer and canonical Rational
   values, basic exact arithmetic, and example/property tests are complete.
   Exact-name Symbol and opaque Expr values with Integer, Rational, Symbol, Add,
   Mul, and Pow forms are complete, including conservative constructor
   normalization, structural equality/hashing, deterministic display, and
   example/property tests. Kernel inspection uses a read-only semantic view
   and exact single-symbol substitution. A bounded parser and an initial
   rational-linear solver now exist in other crates; general symbolic
   rewriting, simplification and a mature CAS API remain future work.
2. Transformation and derivation infrastructure: represent transformations
   and their mathematical justification. A minimal `DerivationStep`, stable
   `RuleId`, `Equal` relation, and one distributive-law transformation are
   implemented. Bounded deterministic distribution and a rule-specific
   independent verifier are also implemented. A general proof calculus,
   assumption system, conditional rewrite registry and complete derivation
   framework remain future work.
3. Exact polynomial domain: a canonical univariate QQ polynomial value now
   provides exact addition, subtraction, multiplication, and evaluation.
   Bounded fallible expression conversion, exact long division and monic GCD
   and exact formal rational-polynomial differentiation are implemented.
   Verified square-free decomposition now separates multiplicity classes.
   Irreducible/multivariate factorization and general calculus remain future work.
4. First equation solver: an exact, univariate linear solver now classifies
   unique, empty, and universal rational solution sets. It records typed
   equation-equivalence steps, with an exactly checked nonzero division
   condition. Nonlinear, multivariate and assumption-dependent solving remain
   future work.
5. Mathema surface frontend: the first bounded expression/equation grammar
   and explicit lowering are implemented in a separate crate. The initial
   CLI can parse, expand, solve rational-linear equations, and perform exact
   polynomial operations, including formal differentiation and square-free
   decomposition. This is not yet
   a general Mathema language, REPL,
   general evaluator or user-facing application.
6. Python bindings: experimental PyO3/maturin package exposes exact parsing,
   distribution, rational-linear solving and polynomial operations using the
   checked Rust kernel. A CI job builds, installs and tests a wheel as the
   publication gate. A stable public API, platform matrix and automated
   releases remain future work.
7. Arbitrary-precision numerical work: distinguish approximation from
   certified numerical results.
8. Calculus and additional domains: exact rational-polynomial differentiation
   is implemented. A first independently verified symbolic derivative supports
   sums, products and positive integer powers over immutable Expr values.
   Negative/fractional powers, general functions, domains/assumptions and
   integration remain separate planned milestones.

### Next CAS foundation

An exact rational-root solver for univariate degree-at-most-two polynomials
now builds on arbitrary-precision perfect-square testing and an independent
mathematical verifier. General algebraic-number arithmetic and ordering,
higher-degree factorization, domain-aware higher-degree rational equations,
partial fractions and symbolic integration remain planned.

A domain-aware quadratic rational-equation solver now combines the exact
rational-root domain with inherited denominator exclusions. The existing
linear rational-equation interface remains unchanged. Unrestricted rational
function syntax and higher-degree nonlinear solving remain future tasks.

### Algebraic root domain and the next platform milestones

Canonical degree-two algebraic values now represent irrational real roots and
nonreal complex conjugates exactly, by a monic irreducible minimal polynomial
and a branch selector. The checked kernel emits typed normalization,
discriminant, and classification evidence, checked independently. The roots2
CLI and Python algebraic_roots API expose the same verified domain.

This is NOT yet general algebraic-number arithmetic, polynomial factorization
over number fields, a symbolic-conditions engine or a user-facing step renderer.

The next integrated engineering phases are tracked in
docs/design/cas-platform.md. Priorities: definedness/assumptions and a
verifiable proof graph; rational expression parsing and conditional rewrites;
full univariate factorization; exact polynomial/algebraic solving; analytic
functions and calculus; certified numerics; a human-friendly renderer; and
then hardened release/distribution. Progress is measured by passing semantic
contracts and capability coverage, not commit count.

### Initial scalar assumptions and guarded rewrite

Typed scalar conditions and a conservative assumptions collection now
distinguish exact facts, premises, unresolved obligations and refuted literal
conditions. The first independently verified conditional transformation
cancels an atomic symbolic inverse only with a nonzero justification.
Composite definedness, general proof graphs and a conditional rewrite
framework remain planned; see [docs/design/assumptions.md](docs/design/assumptions.md).

For a module-level implementation map, see [CAPABILITIES.md](CAPABILITIES.md).
