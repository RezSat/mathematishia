# Contributing

Keep changes narrow and reviewable. Preserve mathematical semantics and
deterministic behavior. Behavioral changes need tests; use property or invariant
tests where they fit. Document non-obvious mathematical assumptions and keep
hidden global state out of the mathematical kernel.

Measure optimization claims with benchmarks before adding complexity. Maintain
source and dependency provenance, and do not port implementation code whose
license is incompatible with this repository. Use the repository checks listed
in the README before proposing a change.

## Changes and reviews

Use concise imperative commit subjects that describe the change, without
Conventional Commit prefixes. Keep comments focused on mathematical invariants,
non-obvious decisions and contracts rather than restating the code.

Open one focused pull request per issue. Keep working branches current with
main, and delete them after integration. Do not rewrite published history.

User-facing mathematical operations must include independently checked kernel
results, domain-aware input handling, CLI and Python access, positive and
refusal-path regression tests, and corresponding documentation. Internal
implementation details do not need separate commands. A result must distinguish
complete, partial, conditional and unsupported outcomes without guessing.

Run formatting, lints and relevant tests before submitting changes. Merge only
after review and all required checks pass.
