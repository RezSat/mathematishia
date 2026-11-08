# Scalar conditions

`Condition` describes a proposition about a scalar expression: `NonZero`, `Positive`, `Real`, `Integer`, or membership in Z, Q, R or C.

`Assumptions` stores caller-supplied premises. `assess` returns one of four statuses:

| Status | Definition |
| --- | --- |
| `Proven` | True by exact rational-literal arithmetic |
| `Assumed` | Follows from an explicit premise under supported implication rules |
| `Unknown` | Not established by the available rules |
| `Refuted` | Contradicted by an exact rational literal |

Exact scalar contradictions are rejected at insertion. Supported implications include positive to nonzero/real and standard inclusions between integer, rational, real and complex domains. Implication is not symmetric.

The guarded cancellation rule `algebra.cancel_symbol_inverse` rewrites `x * x^-1` for an atomic symbol `x` only when `NonZero(x)` is established. Without that condition, the generator returns a `Blocked` result and the unresolved condition. Its independent verifier checks the expression pair, rule ID, equality relation and exact condition status.

A `ProofGraph` links such a step to both the preceding expression and an established condition node. Unknown or refuted conditions cannot support an edge. Composite-base cancellation and general definedness inference are not implemented.
