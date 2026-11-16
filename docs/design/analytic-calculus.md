# Analytic differentiation

The exact symbolic derivative engine uses immutable `Expr` nodes and
postorder `DifferentiationStep` evidence. The generator and
`verify_symbolic_derivative` implement the supported rules independently.

## Supported chain rules

For an inner expression `u` and its checked derivative `u'`:

| Function | Derivative rule | Required domain |
| --- | --- | --- |
| `sin(u)` | `cos(u) * u'` | entire; unrestricted |
| `cos(u)` | `-sin(u) * u'` | entire; unrestricted |
| `exp(u)` | `exp(u) * u'` | entire; unrestricted |
| `tan(u)` | `cos(u)^(-2) * u'` | real or complex, `cos(u) != 0` |
| `log(u)` | `u^(-1) * u'` | real, `u > 0` |

Each rule has a stable `DerivativeRuleId`. Sums, products, constants,
symbols and strictly positive integer powers retain their original rules.
Unrecognized functions and unsupported powers return typed errors.

`EvaluationContext::differentiate_expression` provides unrestricted
formal derivatives and does not accept tangent or logarithm.
`differentiate_domain_expression` requires a source `DomainExpression`
and the selected scalar domain. Every required condition must be retained
in the source. Domain conditions are **assumed prerequisites**, not
mathematical facts proved by this operation.

The complex principal logarithm has a branch cut. A mere `u != 0`
condition does not establish complex differentiability on that cut, so
the complex `log` derivative is explicitly unsupported.

## Proof records and interfaces

The verifier reconstructs the derivative using the original expression,
selected variable, rule IDs and source obligations; it rejects missing,
modified, reordered or unjustified steps. The derivation renderer preserves
condition dependencies as assumed conditions in the output.

CLI `diffreal` and Python `differentiate_expression_real` return the
derivative, checked steps and required real-domain conditions.
There is no general analytic evaluation, branch tracking or elementary
integration in this module.

Sources: `crates/kernel/src/differentiate_expr.rs`,
`crates/kernel/src/context.rs`, `crates/kernel/src/derivation_render.rs`,
`crates/mathema/src/definedness.rs`, `crates/frontend/src/lib.rs`.
