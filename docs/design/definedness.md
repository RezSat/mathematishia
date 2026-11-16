# Definedness of scalar expressions

The current inspection domain is exact scalars, free symbols, sums, products and integer powers over the complex scalars. A negative or zero integer exponent requires a nonzero base. In particular, `0^0` is undefined under this contract. Symbolic and non-integer exponents are rejected rather than assigned implicit branch semantics.

`definedness_obligations` traverses expression nodes with a caller-provided node limit and returns ordered, distinct `Condition::NonZero` obligations. `Assumptions::assess` handles exact rational nonzero facts and derives nonzero products and integer powers from established nonzero factors. A nonzero sum is not inferred from nonzero summands.

`DomainExpression` pairs the normalized `Expr` with restrictions accumulated during `add`, `multiply`, `divide`, and integer powers. Constructing it from existing `Expr` inspects that expression. Using its composition methods retains conditions even when the normalized value loses a factor, such as `0 * x^-1` reducing to `0` on the domain `x != 0`.

The `algebra.cancel_composite_inverse` rule removes a matching `f * f^-1` pair when `f` is a composite expression, recording the definedness obligations of `f` and `NonZero(f)`. Unknown conditions block the rewrite. Refuted literal conditions reject the candidate. The verifier checks the factors, exact resulting expression, rule and complete condition evidence independently of the generator. `ProofGraph` records each condition as an explicit predecessor node.

The ordinary `Expr` constructors still perform unconditional structural normalization. When they fold a product containing an undefined subexpression, they may discard domain restrictions before a later inspection can observe them. Clients that need preservation through construction must use `DomainExpression`. The domain-aware frontend uses this representation; legacy raw expression APIs do not preserve eliminated restrictions.

## Analytic functions

`definedness_obligations_in_domain` inspects built-in sine, cosine, tangent,
exponential and logarithm calls with explicit `ScalarDomain::Real` or
`ScalarDomain::Complex`. Tangent requires a nonzero cosine. Real logarithm
requires a positive argument; complex logarithm requires a nonzero argument.
Children are traversed for nested obligations. Unknown function names are
rejected rather than silently treated as total functions.

`DomainExpression::function` preserves restrictions supplied by the argument
during construction. `DomainExpression::from_expr_in_domain` can inspect an
existing expression but cannot recover restrictions removed by an earlier raw
constructor. Conditions are symbolic obligations, not assertions that they hold.

## Frontend lowering

`parse_domain_expression` and `parse_domain_statement` lower syntax through
`DomainExpression` constructors before scalar folding. They preserve restrictions
of integer powers and nested supported analytic functions, using complex-domain
conditions by default. `parse_domain_expression_in_domain` and
`parse_domain_statement_in_domain` accept explicit real or complex domains.
Symbolic and noninteger powers remain unsupported in the domain-aware parser.

The legacy parser returns plain `Expr` and can lose domain restrictions when
a constructor eliminates an undefined subexpression.

## User interfaces

The domain-aware parser exports expression and equation entrypoints. The CLI
commands `domain`, `subdomain`, and `rewrite` expose inspection,
substitution and independently verified one-step cancellation. Python
provides corresponding immutable results through `domain_expression`,
`domain_equation`, `substitute_domain` and `rewrite_domain`.

The legacy parsing, evaluation and solving entrypoints still accept plain
`Expr` values. They should not be used to reconstruct conditions lost by
constructor normalization. The domain-aware APIs are the supported path
for calculations requiring explicit original-domain provenance.
