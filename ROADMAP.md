# Roadmap

These phases describe architectural goals, not dates.

0. Repository and architectural foundation: establish boundaries, policies, and
   repeatable checks.
1. Exact scalar semantic core: define Integer, Rational, Symbol, and the first
   scalar expression forms with tested semantics. Integer and canonical Rational
   values, basic exact arithmetic, and example/property tests are complete.
   Symbol and scalar expression forms remain unimplemented.
2. Transformation and derivation infrastructure: represent transformations
   and their mathematical justification.
3. Exact polynomial domain: add a specialized polynomial representation and
   algorithms.
4. First equation solver: solve a deliberately bounded class of equations and
   report conditions and derivations.
5. Mathema surface frontend: parse text and lower it into semantic values.
6. Python bindings: expose established kernel behavior.
7. Arbitrary-precision numerical work: distinguish approximation from
   certified numerical results.
8. Calculus and additional domains: add capabilities as their semantics and
   representations are designed.
