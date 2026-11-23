# User-facing exact solving

The CLI dispatches typed parsed equation statements; it does not infer an
operation from a string search. The default variable is `x`; select another
identifier explicitly. Exact finite decimals and selected implicit multiplication are supported.

```console
mathematishia solveauto '2*x+4=10'
mathematishia solveauto 'x^2-2=0'
mathematishia solveauto '(x^2-1)/(x-1)=3'
mathematishia roots 'x^3-2' x
mathematishia integrate 'x^2' x
mathematishia integrate 'x^2' x 0 2
mathematishia integrate 'sin(2*x+1)' x
mathematishia evalexact 'sin(0)+cos(0)'
```

- `solveauto` classifies rational-linear equations over Q, uses certified
  exact complex polynomial root sets for higher-degree polynomial equations,
  and exposes an unresolved cofactor when remaining roots cannot be
  constructed. Rational-function equations keep canceled denominator
  exclusions but currently classify **only rational roots**. Those answers
  are explicitly labeled partial over C.
- `roots` returns checked complex zeros of unrestricted polynomials over Q,
  with exact quadratic defining polynomials/branches and multiplicities.
  A result is complete only when no unresolved polynomial factor remains.
  Domain-restricted expressions are refused rather than simplified into
  unrestricted polynomials.
- `integrate` returns checked zero-constant primitives of polynomials,
  signed definite polynomial integrals at rational endpoints, and checked
  standalone rational multiples of `sin`, `cos` or `exp` with
  nonzero rational-affine arguments. Mixed analytic products, general
  elementary primitives and analytic definite integrals are unsupported.
  No arbitrary integration constant is encoded.
- `evalexact` applies up to 16 independently verified analytic
  constant rewrites: `sin(0)`, `cos(0)`, `tan(0)`, `exp(0)`, `log(1)`.
  Rational arithmetic is folded exactly by the expression representation.
  Unresolved expressions return `status: partial`, never an invented value.
- Existing `expand` handles distributive steps, not certified
  positive-power polynomial expansion. It distinguishes
  `distribution_complete` from `polynomial_expansion: not certified`.
  The complete polynomial expansion kernel is implemented, but a separate CLI command to expose it is still pending.

The parser accepts exact integers, rational numbers, finite decimals, and supported implicit multiplication such as `2x`, `2(x+1)` and `(x+1)(x-1)`. Inequalities, `sqrt`, `abs`, and ambiguous identifier juxtaposition are not yet supported. The kernel enforces checked expression, scalar,
degree and work-unit admission limits. Verification records are checked
before the CLI presents results. Conditions are unproved obligations,
not facts established by this tool.

The Rust kernel now exposes a typed problem-result contract, but the CLI has not yet wired it into `--json` output. Full polynomial expansion and broader problem categories remain separate frontend increments.
