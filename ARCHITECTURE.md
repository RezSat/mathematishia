# Architecture

The intended computation path is:

```text
Mathema Surface Language
        ↓
Frontend Parsing
        ↓
Semantic Lowering
        ↓
Mathema Semantic IR
        ↓
Evaluation Context
        ↓
Mathematishia Kernel
        ↓
Domain-specific representations
        ↓
Algorithms
        ↓
Result + MathematicalDerivation
```

Mathema Surface is the eventual canonical textual language. Mathema Semantic IR
is a syntax-independent mathematical representation; parsing belongs in a
future frontend, not in the semantic crate.

`mathema` will own semantic values and their structural behavior.
`mathematishia-kernel` will own mathematical operations, domain conversions,
algorithms, verification, and derivation generation. Generic symbolic
expressions provide a common language, while specialized domains provide the
representations algorithms need.

Construction creates values. Normalization establishes cheap, deterministic
structural form. Evaluation applies an explicit context. Rewriting and
simplification are deliberate transformation operations. Domain algorithms
operate on suitable domain representations. Derivation records mathematical
steps; verification checks the relevant claims. Formatting renders results and
derivations and does not define their semantics.

Python, API, MCP, and visualization systems are future consumers of kernel
semantics; they must not reimplement mathematical behavior.

Detailed contracts: [expressions](docs/design/expressions.md),
[evaluation](docs/design/evaluation.md), [domains](docs/design/domains.md),
[derivations](docs/design/derivations.md), and
[determinism](docs/design/determinism.md).
