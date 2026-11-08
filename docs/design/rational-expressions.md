# Univariate rational expressions

A rational expression in Q(x) is interpreted as a partial function, not merely
as an element of the fraction field. Its value is represented by coprime
numerator and denominator polynomials over Q. Its domain is represented by
explicit polynomial nonzero conditions.

## Input and normalization

The frontend accepts `+`, `-`, `*`, `/`, parentheses and integer powers.
Division has the same precedence as multiplication and is left-associative.
The domain-aware frontend builds `DomainExpression` values before structural
simplification. For example, `0/(x-1)` has value zero and a nonzero condition
on `x-1`.

`EvaluationContext::rational_expression` converts a domain-aware expression
into `RationalFunction` for a selected symbol. It interprets the expression
recursively using checked rational-function arithmetic and applies the
original domain restrictions to the result.

- Other free symbols and noninteger powers are rejected.
- Integer powers are bounded by the configured maximum polynomial degree.
- A zero denominator or a contradictory zero restriction is rejected.
- Denominator zeros and holes from original subexpressions remain excluded
  after polynomial GCD cancellation.
- Intermediate and final polynomials are subject to the evaluation context's
  degree, scalar-size, condition-count and selected work checks.

For `(x^2-1)/(x-1)`, the reduced value is `x+1` with the retained condition
`x-1 != 0`. Consequently, the expression cannot be evaluated at `x=1`,
and a rational-equation solver cannot admit `x=1` as a root.

## Operations

The exact rational-function kernel provides arithmetic, differentiation by the
quotient rule, evaluation at rational points, and equations whose reduced
residual numerator has degree at most two. The solver returns rational roots
only and verifies their admissibility against the original domain.

CLI: `rsimplify`, `rdiff`, `reval`, `rsolve`.

Python: `rational_simplify`, `rational_derivative`, `rational_value`,
`rational_solve`. Results use immutable Python records and
`fractions.Fraction` for exact numeric values.

Implementation: `crates/kernel/src/rational_expression.rs`,
`rational_function.rs`, and `crates/frontend/src/lib.rs`.

## Verification boundaries

The polynomial GCD and rational equation solvers have exact checks for their
supported domains. General rational-expression conversion uses the checked
rational-function arithmetic implementation, but does not yet produce a
separately verifiable derivation graph for each arithmetic node. Such a graph
is required before claiming a full explanation or independent proof of an
arbitrary rational simplification.
