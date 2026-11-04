# Derivations

`AlgorithmTrace` is low-level developer diagnostics. It is unstable and is not
a user-facing mathematical explanation.

`MathematicalDerivation` is semantic, structured, deterministic, and built as
mathematical computation proceeds. It can be rendered independently of the
algorithm. The kernel now provides a minimal `DerivationStep`: ordinary typed
data recording the `before` and `after` expressions, a stable `RuleId`, and the
`Relation` between them. It is not a general proof framework, and no formal
proof checker exists.

The first rule is `algebra.distribute_product_over_sum` with `Relation::Equal`.
It applies the unconditional distributive identity
`R * (t1 + ... + tn) -> R*t1 + ... + R*tn` once to a top-level product,
selecting the first sum factor in canonical order. It operates on commutative
scalar-expression semantics and does not recurse or expand to a fixed point.
Because the rule is an unconditional formal identity, it has no side
conditions. Future conditional rules must represent their assumptions and
obligations explicitly in typed data before they are implemented.

Bounded distribution can now search Add/Mul children in canonical depth-first order
without entering powers. Each step records the full expression before and after;
`expand_distributively` returns a connected sequence, a result, and an explicit
completion flag. Completion means no further eligible distribution within this
restricted traversal, not complete symbolic simplification. The step budget
limits steps only, not expression size or memory consumption.

Equation and assumption representations, proof or verification obligations,
introduced restrictions, and optional sub-derivations remain future work.
`substitute_symbol` is a single-symbol replacement operation, not an
equality-preserving derivation: without an assignment assumption, substitution
does not prove equality of the original and resulting expressions.

Relation concepts for future rules include `Equal`, `Equivalent`, `Implies`,
`Approximate`, and `DefinedAs`; only `Equal` is implemented. Phrases such as
“By the zero-product property” belong to rendering, not mathematical semantics.
Mathematishia is not claiming to be a formal proof assistant.

The initial linear equation solver works with a separate semantic `Equation`
value (two Expr sides). It converts both sides to bounded univariate rational
polynomials and classifies their difference as a unique root, no solution, or
all rational values. Recorded `EquationStep`s use `Relation::Equivalent`, not
expression equality, and the coefficient-division step carries the exact
nonzero scalar verified by arithmetic. Constant-equation classification is a
typed result, not an invented equality step. This is not a general equation
solver, conditional rewrite system, or independently checked proof certificate.

## Independent checking (Task 14)

The kernel now exposes explicit, rule-specific verification operations for
recorded distributive and rational-linear derivations. Distribution verification
reconstructs *all* legal one-step rewrites in supported Add/Mul contexts without
calling the generator. Equation verification checks rule shapes, exact rational
solution-set equivalence, and the recorded nonzero coefficient at division.
Chain verification checks the supplied source, intermediate continuity, result,
solution classification and narrow completion semantics. An unrecognized or
unsupported claim is rejected; this is not a general proof assistant, proof
certificate format, or independent verification of arbitrary symbolic functions.
