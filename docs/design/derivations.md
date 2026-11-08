# Derivations and verification

The kernel stores mathematical evidence as typed values. Human-readable explanations are produced separately and have no authority over a proof result.

## Relations

- `Relation::Equal`: two scalar expressions have the same value on the rule's domain.
- `Relation::Equivalent`: two equations have the same solution set in the selected domain.
- `Relation::DerivativeOf`: the recorded result is the formal derivative of an expression with respect to a specified variable.

These relations are not interchangeable. In particular, a derivative is not an equality rewrite.

## Existing step records

`DerivationStep` records an expression before and after the distributive law, with the stable rule ID `algebra.distribute_product_over_sum`. The verifier enumerates valid applications in supported additive/multiplicative contexts.

`EquationStep` carries exact equivalence evidence for rational-linear solving, including the nonzero divisor used to isolate a variable.

`DifferentiationStep` records formal derivative rules for constants, symbols, sums, products and positive integer powers. The derivative verifier reconstructs each claimed result.

Quadratic-root evidence records the normalized polynomial, exact discriminant and classification. These are typed facts rather than invented expression-equality steps.

`GuardedRewriteStep` records `algebra.cancel_symbol_inverse`, its nonzero condition and whether that condition is established by exact arithmetic or by a supplied premise.

## Proof graph

`ProofGraph` stores a sequentially numbered list of nodes; references may point only to earlier nodes. The supported node types are:

| Node | Value | Dependencies |
| --- | --- | --- |
| Input | Scalar expression | None |
| Condition | Typed condition with claimed status | External assumption context |
| Distribution | Result expression plus distributive evidence | Prior expression node |
| Guarded cancellation | Result expression plus cancellation evidence | Prior expression and condition nodes |

`verify_proof_graph` checks the input, references, condition statuses, exact step records and selected expression conclusion. All nodes must be reachable as dependencies of the conclusion; unused branches and orphaned premises are rejected.

An established condition is marked `Proven` for an exact rational fact or `Assumed` when it follows from the supplied premises. Unknown conditions cannot justify a graph edge. The graph has a configurable node limit (128 by default).

The current graph covers expression equality using the two registered rewrite rules. Equation proofs, condition discharge inside the graph, general composition, and student-facing rendering require further work.
