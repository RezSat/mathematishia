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
mathematishia integrate 'x^2+sin(2*x)' x
mathematishia expandall '(x+1)^2'
mathematishia expandall '(x-y)^3' x y
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
- `integrate` now delegates to the checked integration dispatcher. It supports
  full polynomials over Q and top-level sums of polynomial and rational-multiple
  `sin`, `cos`, `exp` terms with nonzero rational-affine arguments. Every
  term has its own independently checked primitive certificate. The displayed
  primitive fixes the additive constant to zero and retains original domain
  conditions as **unproved requirements**, labeled `conditional`. Exact
  directed definite integration accepts rational bounds and polynomial
  integrands without historical restrictions. Analytic definite integrals,
  mixed-function products, unexpanded multiplied sums, `tan`, `log`,
  nonlinear analytic arguments and general rational integrals remain unsupported.
  Resource exhaustion and unsupported mathematical forms have distinct errors.
- `evalexact` applies up to 16 independently verified analytic
  constant rewrites: `sin(0)`, `cos(0)`, `tan(0)`, `exp(0)`, `log(1)`.
  Rational arithmetic is folded exactly by the expression representation.
  Unresolved expressions return `status: partial`, never an invented value.
- `expandall` uses the checked exact sparse Q[variables] polynomial kernel.
  It handles nested sums, products, and positive integer powers such as
  `(x+1)^2` and `(x-y)^3`. Omit variables to infer them from the
  parsed expression, or provide an explicit ambient ring with
  `expandall '<EXPRESSION>' x y`. The ring is deterministically sorted.
  The command displays the canonical expanded expression, any inherited
  domain conditions, whether its syntax changed, and `complete: true`
  **only after independent polynomial equality verification**. Nonpolynomial
  powers, analytic functions, undeclared symbols and exhausted degree/term/work
  budgets produce typed refusals. Expansion is over exact rational coefficients.
- Legacy `expand` remains distributive-step-only and continues to report
  `distribution_complete` and `polynomial_expansion: not certified`;
  its behavior is intentionally unchanged.

The parser accepts exact integers, rational numbers, finite decimals, and supported implicit multiplication such as `2x`, `2(x+1)` and `(x+1)(x-1)`. Single real comparisons (`<`, `<=`, `>`, `>=`, `≤`, `≥`) are supported by `inequality`, `solve` and `solveauto`; chained comparisons, `sqrt`, `abs`, and ambiguous identifier juxtaposition remain unsupported. The kernel enforces checked expression, scalar,
degree and work-unit admission limits. Verification records are checked
before the CLI presents results. Conditions are unproved obligations,
not facts established by this tool.

The Rust kernel exposes a typed `ProblemResult` contract for currently
supported solver operations. It does **not** yet have a checked expansion
outcome, and its `Integrate` adapter is deliberately unsupported. Accordingly,
`expandall` and combined `integrate` report verified human-readable
kernel certificates directly; they do not invent `ProblemResult` evidence or
ad hoc JSON. `--json` remains unavailable until a separately agreed,
deterministic schema and adapter support exist. The CLI does not infer
assumptions or invent algebraic proof steps.

## Real limits and inequalities

```sh
mathematishia limit '(x^2-1)/(x-1)' x 1
mathematishia limit '1/x' x 0 left
mathematishia inequality '(x-1)/(x+2)<=0' x
mathematishia solveauto 'x^2-4>0'
```

Limits take an exact finite rational point and optional direction `left`, `right` or `two-sided` (default). Finite values, signed infinities and two-sided nonexistence are explicit; `excluded_at_point` records whether the original function is undefined there. Transcendental expressions and limits at infinity remain unsupported.

Inequalities return an exact ordered real interval union with open/closed endpoints and original denominator exclusions. Numerator, denominator and each inherited restriction must have degree at most two after checked rational conversion. Rational endpoints are exact; irrational quadratic endpoints are represented by a minimal polynomial and root branch. Both original operands are checked before subtraction. Comparisons are not silently interpreted as equations; chained comparisons and higher-degree factors are refused.
