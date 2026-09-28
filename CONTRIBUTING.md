# Contributing

Keep changes narrow and reviewable. Preserve mathematical semantics and
deterministic behavior. Behavioral changes need tests; use property or invariant
tests where they fit. Document non-obvious mathematical assumptions and keep
hidden global state out of the mathematical kernel.

Measure optimization claims with benchmarks before adding complexity. Maintain
source and dependency provenance, and do not port implementation code whose
license is incompatible with this repository. Use the repository checks listed
in the README before proposing a change.
