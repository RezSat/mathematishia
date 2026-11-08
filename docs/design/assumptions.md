# Scalar assumptions and guarded rewriting

A `Condition` is a typed mathematical proposition over an immutable `Expr`.
The initial vocabulary covers `NonZero`, `Positive`, `Real`, `Integer`
and membership in the integer, rational, real, or complex scalar domains.

The kernel's `Assumptions` collection stores explicitly supplied premises.
`assess` distinguishes four states:

| Status | Meaning |
| --- | --- |
| Proven | Exact rational-literal arithmetic establishes the proposition. |
| Assumed | The proposition follows from a recorded premise under the supported implication rules. |
| Unknown | The proposition has neither supporting evidence nor an exact counterexample. |
| Refuted | Exact rational-literal arithmetic contradicts the proposition. |

Premises contradicted by exact scalar values are rejected. Direct repetition,
logical implication from positive to nonzero/real, and number-domain inclusions
are supported. These implications are intentionally one-way: being real does not
prove that a number is positive, and being rational does not prove that it is an
integer. A premise is not silently promoted into an unconditional proof.

The first guarded rule, `algebra.cancel_symbol_inverse`, removes one
`x * x^-1` pair from a commutative scalar product, where `x` is an atomic
symbol. The transformation requires `NonZero(x)`. Without that condition it
returns a `Blocked` outcome containing the unresolved obligation. An applied
step records its exact input/output, stable rule identifier, equality relation,
condition, and whether the justification is proven or assumed.

`verify_guarded_rewrite_step` independently checks the pair, resulting
expression, rule, relation, and premise classification. It never calls the
rewrite generator. Verified steps with assumed premises are conditional
equalities under those premises, not universal expression identities.

This is not yet a global domain/definedness engine. In particular, arbitrary
composite reciprocal bases are excluded because the expressions inside them
might themselves be undefined. There is no general rewriting framework,
symbolic sign solver, contradiction solver for symbolic premises, or proof DAG.
These are separate planned extensions.
