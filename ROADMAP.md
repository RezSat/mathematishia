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
   are implemented. Multivariate and advanced algorithms remain future work.
4. First equation solver: an exact, univariate linear solver now classifies
   unique, empty, and universal rational solution sets. It records typed
   equation-equivalence steps, with an exactly checked nonzero division
   condition. Nonlinear, multivariate and assumption-dependent solving remain
   future work.
5. Mathema surface frontend: the first bounded expression/equation grammar
   and explicit lowering are implemented in a separate crate. The initial
   CLI can parse, expand, solve rational-linear equations, and perform exact
   polynomial operations. This is not yet a general Mathema language, REPL,
   general evaluator or user-facing application.
6. Python bindings: experimental PyO3/maturin package exposes exact parsing,
   distribution, rational-linear solving and polynomial operations using the
   checked Rust kernel. A CI job builds, installs and tests a wheel as the
   publication gate. A stable public API, platform matrix and automated
   releases remain future work.
7. Arbitrary-precision numerical work: distinguish approximation from
   certified numerical results.
8. Calculus and additional domains: add capabilities as their semantics and
   representations are designed.
