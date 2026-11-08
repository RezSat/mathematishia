# Derivations

`AlgorithmTrace` is low-level developer diagnostics. It is unstable and is not
a user-facing mathematical explanation.

`MathematicalDerivation` is semantic, structured, deterministic, and built as
mathematical computation proceeds. It can be rendered independently of the
algorithm. The kernel now provides a minimal `DerivationStep`: ordinary typed
data recording the `before` and `after` expressions, a stable `RuleId`, and the
`Relation` between them. A limited, rule-specific independent checker now
exists; it is not a general formal proof framework.

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

A separate `Equation` representation and exact nonzero conditions for the
linear division rule are implemented. General assumptions, proof obligations,
introduced restrictions and optional sub-derivations remain future work.
`substitute_symbol` is a single-symbol replacement operation, not an
equality-preserving derivation: without an assignment assumption, substitution
does not prove equality of the original and resulting expressions.

The implemented relation kinds are `Equal` for scalar identities and
`Equivalent` for equations with identical rational solution sets. `Implies`,
`Approximate` and `DefinedAs` remain future designs. Phrases such as
“By the zero-product property” belong to rendering, not mathematical semantics.
Mathematishia is not claiming to be a formal proof assistant.

The initial linear equation solver works with a separate semantic `Equation`
value (two Expr sides). It converts both sides to bounded univariate rational
polynomials and classifies their difference as a unique root, no solution, or
all rational values. Recorded `EquationStep`s use `Relation::Equivalent`, not
expression equality, and the coefficient-division step carries the exact
nonzero scalar verified by arithmetic. Constant-equation classification is a
typed result, not an invented equality step. This is not a general equation
solver or conditional rewrite system. The supported steps can be checked
independently, but they do not constitute a general proof certificate.

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

## Verified symbolic derivative rules (Task 20)

The checked `differentiate_expression(&Expr, &Symbol)` API implements a
strictly limited exact formal calculus: integer/rational constants, selected
symbols, independent parameter symbols, sums, products and positive integer
powers. It records postorder `DifferentiationStep` values with stable
`calculus.*` rule identifiers. `Relation::DerivativeOf` deliberately does
**not** imply equality of the original expression and its derivative.

The separate `verify_symbolic_derivative` walks the original expression and
independently reconstructs every rule's expected derivative, validates the
claimed relation, enforces complete step order and exact result equality.
The checker does not delegate to the generation routine and does not compare
floating-point samples. The context caps positive exponents at
`max_symbolic_power` and meters traversal/work. Negative, zero, fractional
and variable exponents are rejected explicitly: extending them requires
mathematically defined domains, nonzero obligations and an assumption system.
This does not constitute a general symbolic differentiation/rewrite engine,
proof calculus, transcendental function library, or integration system.


## Evidence for algebraic roots

The quadratic-algebraic API returns typed ordered evidence rather than
pretending every mathematical operation is an expression-equality rewrite.
MonicNormalization records the exact normalized polynomial. Discriminant
records an exact rational value. Classification carries the stable root
rule identifier (rational pair, repeated root, real irrational pair,
complex conjugates, or degenerate degree). The kernel verifies the entire
sequence independently, including classification completeness.

For an eventual Symbolab-style renderer, these are semantic facts: a
renderer may generate user-friendly text or LaTeX, but must not invent a
rule, an equality, a domain condition, or a proof obligation. The next
phase needs a typed proof graph and a formal assumptions/definedness
model to express conditional steps such as division by a possibly-zero
symbol. The current typed evidence is a bounded proof record, not a
general-purpose theorem prover.
