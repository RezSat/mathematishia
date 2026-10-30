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

Equation and assumption representations, proof or verification obligations,
introduced restrictions, and optional sub-derivations remain future work.
`substitute_symbol` is a single-symbol replacement operation, not an
equality-preserving derivation: without an assignment assumption, substitution
does not prove equality of the original and resulting expressions.

Relation concepts for future rules include `Equal`, `Equivalent`, `Implies`,
`Approximate`, and `DefinedAs`; only `Equal` is implemented. Phrases such as
“By the zero-product property” belong to rendering, not mathematical semantics.
Mathematishia is not claiming to be a formal proof assistant.
