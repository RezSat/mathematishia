# Mathema textual expression grammar

`mathema-frontend` parses one expression or one equation per input.
An equation contains one top-level `=`. The parser produces a private
syntax tree before lowering into immutable semantic values.

- Arbitrary-precision decimal integer literals and exact rational division,
  with denominator-zero rejection. Floating-point literals are unsupported.
- Unicode identifiers beginning with a letter or underscore, followed by
  letters, digits or underscores.
- Explicit `+`, `-`, `*`, `/`, `^`, unary signs and parentheses. There is
  no implicit multiplication.
- Division and multiplication share precedence. Powers associate to the right;
  unary minus binds below exponentiation, so `-x^2` and `(-x)^2` differ.
- Built-in unary calls `sin(expr)`, `cos(expr)`, `tan(expr)`, `exp(expr)`
  and `log(expr)`. Unknown function calls are rejected.
- No assignments, floating-point literals, function definitions, comments,
  semicolon-separated statements or multi-argument calls.

The parser limits input bytes, tokens and nesting depth. Syntax errors retain
UTF-8 byte offsets. `Expr::Display` is not a stable serialization or a
guaranteed round-trip source encoding.

## Domain-preserving lowering

`parse_domain_expression` and `parse_domain_statement` retain conditions
removed by structural normalization. They use complex-domain integer-power
and built-in function restrictions. Their `_in_domain` variants select
`ScalarDomain::Real` or `ScalarDomain::Complex`.

Real logarithm requires a positive argument; complex logarithm requires a
nonzero argument. Tangent requires a nonzero cosine argument. Nested calls are
traversed. Conditions are prerequisites, not proven claims.

The CLI exposes `diffexpr` for supported unrestricted formal derivatives
and `diffreal` for checked real derivatives with explicit conditions.
Both delegate mathematics to the kernel. The Python binding provides
`differentiate_expression` and `differentiate_expression_real`.
