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

## Exact Sturm root counts

```sh
mathematishia sturm 'x^2-2' x -2 2
mathematishia sturm 'x^2-2' x 0 2
```

Counts distinct real roots strictly inside a finite rational interval using the
same independently verified Sturm certificate as Python's `sturm_root_count`.
The polynomial must be nonzero, square-free and unrestricted over Q[x],
with ordered non-root endpoints. Nonzero constants have count zero. Exact
decimal or rational endpoints are accepted. The output includes the count,
endpoint sign variations, the Sturm sequence and its endpoint signs.
A repeated factor, even outside the interval, is unsupported. Original
source-domain restrictions (including canceled denominator zeros), root
endpoints and undefined scalars are refused. Degree and work budgets apply.
This is a root count, not root isolation or a list of roots.

## Explicit affine systems, ordered division and root brackets

```sh
mathematishia linsystem x,y 'x+y=3' 'x-y=1'
mathematishia mvdivide x,y 'x*y+1' x y
mathematishia rootbracket 'x^2-2' x 1 2 '1/16' 8
```

- `linsystem` accepts 1 to 16 equations in 1 to 16 explicitly named distinct
  variables. It returns the complete empty, unique or affine-family
  solution set over Q, with vectors in the input variable order.
  For a family, every solution is the particular vector plus a rational
  linear combination of the displayed basis vectors. Non-affine inputs
  and undeclared variables are refused.
- `mvdivide` accepts an explicit ring of up to 16 variables and up to
  32 nonzero polynomial divisors over Q. The kernel uses lexicographic
  monomials in sorted variable-name order, but tries divisors in supplied
  order. Each quotient corresponds to its divisor. The remainder is
  reduced with respect to their leading terms, not necessarily a unique
  normal form. There is no Gröbner basis claim. The default bounded
  multivariate limits include total degree 128 and 2048 distinct terms.
- `rootbracket` accepts a nonzero unrestricted Q[x] polynomial, increasing
  finite rational endpoints, positive rational target width and a nonnegative
  bisection count. An exact endpoint or midpoint root is reported as one
  rational root. Opposite signs certify at least one root in the returned
  open interval; equal nonzero signs remain unresolved. A false
  `precision_met` does not invalidate a sign-change existence certificate.
  No uniqueness or all-root isolation is claimed.

All three commands refuse inherited domain restrictions rather than discard
them, enforce checked kernel resource bounds, and independently verify their
results. They do not use a separate mathematical algorithm or wire schema.

See [CLI and Python capability parity](PARITY.md) for the operation-by-operation scope.
