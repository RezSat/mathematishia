# Evaluation

Expression construction, normalization, substitution, rewriting and domain evaluation are separate operations.

`Expr` constructors perform structural normalization only. `EvaluationContext` applies per-call admission limits and invokes supported checked algorithms. Existing low-level APIs remain available to trusted Rust callers.

## Resource limits

The context checks expression size and depth, scalar bit length, polynomial degree, intermediate arithmetic bounds, distribution steps, symbolic powers, rational conditions and metered work units. Checked operations admit and validate inputs and outputs at supported boundaries. The budgets are deterministic but do not impose process-level memory or wall-clock limits.

Parsing and low-level expression construction occur before the evaluation context is applied. Workload isolation must therefore be provided by an external execution boundary for untrusted inputs.

## Assumptions

`Assumptions` stores premises and classifies supported scalar conditions as proven, assumed, unknown or refuted. The initial guarded rewrite requires an established `NonZero` condition. Proof graphs attach condition nodes to the rewrites that consume them.

There is no global mutable mathematical state. Precision controls, general definedness inference and evaluation of analytic functions remain separate work.
