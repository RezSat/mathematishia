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
   example/property tests. Kernel inspection uses a read-only semantic view;
   exact single-symbol substitution is implemented. Parsing, general
   rewriting, simplification, solving, and a user-facing CAS API remain
   unimplemented.
2. Transformation and derivation infrastructure: represent transformations
   and their mathematical justification. A minimal `DerivationStep`, stable
   `RuleId`, `Equal` relation, and one distributive-law transformation are
   implemented. General transformation infrastructure, assumptions,
   verification obligations, and complete derivations remain future work.
   The distributive rule now supports bounded, deterministic sequences of
   whole-expression steps under a restricted Add/Mul traversal.
3. Exact polynomial domain: a canonical univariate QQ polynomial value now
   provides exact addition, subtraction, multiplication, and evaluation.
   Explicit fallible Expr conversion, division, GCD, and advanced algorithms
   remain future milestones.
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
6. Python bindings: expose established kernel behavior.
7. Arbitrary-precision numerical work: distinguish approximation from
   certified numerical results.
8. Calculus and additional domains: add capabilities as their semantics and
   representations are designed.
