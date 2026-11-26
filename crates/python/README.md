# Python bindings

Install from the repository checkout with Python 3.10+ and a Rust toolchain:

```sh
python -m pip install ./crates/python
```

The package uses maturin and PyO3. Results are immutable Python objects backed by the same checked Rust kernel used by the CLI.

```python
from fractions import Fraction
import mathematishia as m

answer = m.solve("2*x+4=10", "x")
assert answer.kind == "unique"
assert answer.value == Fraction(3)
assert m.evaluate("x^2+1/2", "x", "1/2") == Fraction(3, 4)

roots = m.algebraic_roots("x^2-2", "x")
for root in roots.roots:
    print(root.kind, root.branch, root.minimal_polynomial)
```

## Supported operations

- `solve`, `solve_quadratic` and `rational_solve_quadratic`: selected exact equation domains. The quadratic rational solver retains excluded denominator zeros.
- `evaluate`, `differentiate`, `square_free`, `extended_gcd`, `modular_inverse`: exact rational-polynomial operations and independently checked Bézout coefficients.
- `differentiate_expression`: formal differentiation of sums, products, positive integer powers and entire analytic sine, cosine and exponential calls, with checked rule steps.
- `differentiate_expression_real`: checked real-domain derivatives, including tangent and logarithm, with explicit symbolic prerequisites in `result.conditions`.
- `rational_function`, `rational_evaluate`, `rational_differentiate`, `rational_operation`: partial rational functions with recorded domain restrictions.
- `rational_simplify`, `rational_derivative`, `rational_value`, `rational_solve`: full univariate rational expressions and equations over Q, preserving source-domain exclusions.
- `algebraic_roots`: exact quadratic roots, represented as rationals or as a monic minimal polynomial with a branch selector.
- `factor_polynomial`: certified rational linear, quadratic and bounded cubic factors, plus unresolved cofactors.
- `extension_value`, `extension_operation`, `extension_power`, `extension_minimal_polynomial`: arithmetic and minimal polynomials in supported certified abstract algebraic fields.

The parser covers a bounded grammar. Analytic numerical evaluation, general integration, cross-field arithmetic, complete polynomial factorization and a stable Python compatibility contract are not implemented. See [the syntax specification](../../docs/design/mathema-surface.md).

## Domain-aware expressions

```python
import mathematishia as m

value = m.domain_expression("0*x^-1")
assert value.expression == "0"
assert value.conditions == ("x != 0",)

equation = m.domain_equation("0*x^-1=0*y^-1")
substituted = m.substitute_domain("0*x^-1", "x", "0")
assert substituted.conditions == ("0 != 0",)

rewrite = m.rewrite_domain("(x+1)*(x+1)^-1", ("x+1",))
assert rewrite.status == "applied"
assert rewrite.expression == "1"
```

The `domain_expression`, `domain_equation`, `substitute_domain`, and
`rewrite_domain` interfaces use integer-power definedness rules.
Noninteger and symbolic powers are rejected. The original `expression`
function continues to return a plain normalized string without domain
provenance.

## Rational expressions

```python
from fractions import Fraction
import mathematishia as m

simplified = m.rational_simplify("(x^2-1)/(x-1)", "x")
assert simplified.numerator.coefficients == (Fraction(1), Fraction(1))
assert simplified.excluded_zero_sets[0].coefficients == (
    Fraction(-1), Fraction(1)
)
assert m.rational_value("(x^2-1)/(x-1)", "x", "2") == Fraction(3)

roots = m.rational_solve("(x^2-1)/(x-1)=3", "x")
assert roots.roots == (Fraction(2),)
```

The rational solver currently returns rational roots only and requires a
residual numerator of degree at most two. An empty finite root list does
not rule out irrational or complex roots. See
[the rational-expression design](../../docs/design/rational-expressions.md).

## Polynomial Bézout identity

```python
import mathematishia as m

result = m.extended_gcd("x^3-1", "x^2-1", "x")
assert result.gcd.expression == "-1 + x"

inverse = m.modular_inverse("x", "x^2+1", "x")
assert inverse is not None
```

The result records a complete Euclidean division chain and coefficients
`s` and `t` satisfying `s*a+t*b=gcd(a,b)`. The Rust verifier checks
each division identity, the terminating remainder, the monic gcd, and
the Bézout identity. A modular inverse is returned only when the gcd is 1.

## Polynomial quotient rings and quadratic fields

```python
from fractions import Fraction
import mathematishia as m

ring_value = m.modular_operation("mul", "x", "x", "x^2+1", "x")
assert ring_value.representative.coefficients == (Fraction(-1),)
assert m.modular_power("x-1", "x^2-1", "x", -1) is None

value = m.quadratic_value("x^2-2", "x", "upper_real", "1", "2")
assert value.minimal_polynomial.coefficients == (
    Fraction(-7), Fraction(-2), Fraction(1)
)
assert value.norm == Fraction(-7)

square_i = m.quadratic_operation(
    "mul", "x^2+1", "x", "positive_imaginary",
    "0", "1", "0", "1",
)
assert square_i.constant == Fraction(-1)
assert square_i.coefficient == Fraction(0)
```

`modular_reduce`, `modular_operation` and `modular_power` operate in
quotient rings. Nonunits have no negative powers. `quadratic_value`,
`quadratic_operation` and `quadratic_inverse` require a certified irreducible
quadratic with one selected embedding. These APIs do not combine elements from
different quadratic fields.

## Rational factors and abstract cubic fields

```python
from fractions import Fraction
import mathematishia as m

factors = m.factor_polynomial("(x-1)*(x^2+1)", "x")
assert factors.complete
assert len(factors.factors) == 2

partial = m.factor_polynomial("x^4+1", "x")
assert not partial.complete

cube = m.extension_power("x^3-2", "x", "x", 3)
assert cube.representative.coefficients == (Fraction(2),)

minimal = m.extension_minimal_polynomial("x^3-2", "x^2", "x")
assert minimal.coefficients == (
    Fraction(-4), Fraction(0), Fraction(0), Fraction(1)
)
```

The cubic field API defines an abstract generator `alpha` satisfying the
given irreducible polynomial. It does not select a real or complex root.
A reducible or uncertified defining polynomial is rejected.

## Exact rational matrices

Matrix entries accept integers, exact fraction strings or `fractions.Fraction`.
Binary floating-point entries are rejected rather than implicitly interpreted
as exact decimal values.

```python
from fractions import Fraction
import mathematishia as m

a = [[2, 1], [5, 3]]
assert m.matrix_determinant(a) == Fraction(1)

inverse = m.matrix_inverse(a)
assert inverse.rows == (
    (Fraction(3), Fraction(-1)),
    (Fraction(-5), Fraction(2)),
)

solution = m.matrix_solve(a, [5, 13])
assert solution.kind == "unique"
assert solution.particular == (Fraction(2), Fraction(1))

affine = m.matrix_solve([[1, 1, 1], [2, 2, 2]], [3, 6])
assert affine.kind == "infinite"
assert len(affine.nullspace_basis) == 2
```

Other operations: `matrix_rref` returns the row-reduced matrix, pivot
columns and rank; `matrix_multiply` multiplies two matrices. The exact
kernel currently accepts dimensions up to 16.

## Domain-checked real differentiation

```python
import mathematishia as m

result = m.differentiate_expression_real("tan(log(x))", "x")
assert result.steps[-1].rule == "calculus.tan_chain"
assert "x > 0" in result.conditions
```

Each condition is a required hypothesis, not a theorem proved by differentiation.
The unrestricted `differentiate_expression` supports sine, cosine and
exponential chain rules. Tangent and logarithm require the domain-aware interface.
Complex principal-log differentiation is not included.

## Certified root sets, real root counts, and exact integrals

These entrypoints call the Rust kernel and preserve exact rational values and
verification outcomes. They return frozen Python dataclasses, not human-formatted
success messages.

```python
from fractions import Fraction
import mathematishia as m

solutions = m.solve_polynomial_equation("x^2-5*x+6=0", "x")
assert solutions.complete
assert [root.rational for root in solutions.roots] == [Fraction(2), Fraction(3)]

irrational = m.polynomial_roots("x^2-2", "x")
assert [root.branch for root in irrational.roots] == ["lower_real", "upper_real"]

count = m.sturm_root_count("x^2-2", "x", "-2", "2")
assert count.count == 2

primitive = m.integrate_polynomial("x^2", "x")
assert primitive.primitive.coefficients == (
    Fraction(0), Fraction(0), Fraction(0), Fraction(1, 3)
)
assert m.integrate_polynomial_between("x^2", "x", "0", "3").value == 9

identity = m.analytic_constant("sin(0)")
assert (identity.status, identity.expression, identity.rule) == (
    "exact", "0", "analytic.sin_zero"
)

elementary = m.integrate_elementary("cos(x)", "x")
assert elementary.rule == "calculus.integrate_affine_cosine"
```

`polynomial_roots` and `solve_polynomial_equation` return **certified
complex roots**, with multiplicities and explicit `complete` and
`unresolved_cofactor` fields. A partial result must not be interpreted as a
complete root set. `solve` remains the existing rational-linear solver;
`solve_polynomial_equation` is the new separate polynomial-equality entrypoint
until the Python adapter consumes the now-public shared kernel result contract. Source-domain exclusions
cannot be silently discarded; use `rational_solve` for supported rational
equations.

`sturm_root_count` counts distinct real roots strictly between two rational
endpoints and includes the exact Sturm sequence, endpoint signs and variation
counts. It rejects zero or nonsquare-free polynomials, reversed intervals, and
root endpoints. It does not isolate roots or approximate their positions.

`integrate_polynomial` returns the canonical antiderivative with zero integration
constant; `integrate_polynomial_between` returns an exact directed value.
Both reject source-domain exclusions. `integrate_elementary` supports only
one rational multiple of `sin`, `cos`, or `exp` of a nonconstant rational-affine
argument; the primitive has no represented additive constant, and retained
conditions are not automatically proven. General integration and generic math-question dispatch remain limited.
Checked finite-point rational-function limits and real inequalities are exposed through typed Python adapters. The API does not infer unsupported mathematical results.

`analytic_constant` applies at most one deterministic, verified rewrite:
`sin(0)`, `cos(0)`, `tan(0)`, `exp(0)` or `log(1)`. It returns
`status="unchanged"` for nonmatching input and raises a stable exception for
unsupported domain or undefined inputs; it is not a general analytic evaluator.

All these operations use the kernel's default fixed resource budgets.
`MathSyntaxError`, `UnsupportedMathematicsError`, `ResourceLimitError`,
`UndefinedAtPointError` and `VerificationFailure` distinguish the principal
error categories. Numeric inputs are exact rational **strings** rather than
binary floats. API object fields are immutable.


## Polynomial expansion and combined integration

These Python interfaces call the same certified Rust kernel operations as the
corresponding CLI commands. They return frozen, typed results with exact
`fractions.Fraction` coefficients and do not implement arithmetic in Python.

```python
from fractions import Fraction
import mathematishia as m

square = m.expandall("(x+1)^2", ["x"])
assert square.status == "exact" and square.complete
assert square.variables == ("x",)
assert {(term.exponents, term.coefficient) for term in square.terms} == {
    ((0,), Fraction(1)), ((1,), Fraction(2)), ((2,), Fraction(1))
}

cube = m.expand_polynomial("(x-y)^3", ["y", "x"])
assert cube.variables == ("x", "y")
assert {(t.exponents, t.coefficient) for t in cube.terms} == {
    ((3, 0), Fraction(1)), ((2, 1), Fraction(-3)),
    ((1, 2), Fraction(3)), ((0, 3), Fraction(-1))
}

answer = m.integrate("x^2+sin(2*x)", "x")
assert answer.status == "exact"
assert len(answer.terms) == 2
assert answer.primitive.conditions == answer.source.conditions
```

- `expandall(source, variables)` and `expand_polynomial(source, variables)`
  are the same fully checked polynomial expansion. The explicit ambient
  `Q[variables]` ring is sorted canonically in the result; each `MultivariateTerm`
  has an exponent tuple aligned with that ring and an exact rational coefficient.
  `source` and `expanded` are domain-aware objects with original conditions.
  `complete=True` means *all* supported polynomial terms were expanded,
  not merely one distributive step. `status="conditional"` retains unproved
  obligations; `status="exact"` has no retained conditions. The older
  `expand(source, max_steps)` remains the stepwise distribution API.
- `integrate(source, variable, domain="real")` and
  `integrate_expression_dispatch(...)` return a `DispatchedIntegral`
  with an exact zero-additive-constant primitive, retained unproved
  conditions, and checked `IntegrationTerm` evidence. The status is
  `"conditional"` when obligations remain and `"exact"` otherwise. A polynomial term
  exposes an exact polynomial-primitive certificate; a sine/cosine/exponential
  term exposes its affine scale/slope/intercept, rule ID and derivative
  verification rules. Domains `"rational"`, `"real"`, and `"complex"` are
  explicit; analytic terms require real or complex domain. The older
  `integrate_polynomial`, `integrate_polynomial_between` and
  `integrate_elementary` APIs remain available.

**Mathematical boundaries:** Expansion supports finite bounded rational
polynomials only. Undeclared symbols, negative/fractional/symbolic powers and
analytic functions are unsupported. Complex exponentials and general
analytic simplification are not polynomial expansion. A zero expression with
history such as `0*x^-1` remains subject to `x != 0`.

Combined integration supports bounded *top-level sums* of rational-polynomial
terms and rational-affine sin/cos/exp terms. Products such as `x*sin(x)`,
`sin(x)*cos(x)`, unexpanded products of sums, logarithms, tangents,
nonlinear analytic arguments, general substitutions and symbolic additive
constants are not claimed. Unresolved domain conditions are retained,
not discharged. Exact analytic definite endpoint evaluation is not provided.

**Refusal and proof handling:** No partially expanded expression is falsely
reported as complete. Unsupported input raises `UnsupportedMathematicsError`,
admission/term/work budgets raise `ResourceLimitError`, invalid syntax
raises `MathSyntaxError`, and a failed independent Rust certificate check
raises `VerificationFailure`. Mathematical evidence comes from the
independently verified kernel, not user-modifiable Python display fields.
Scalar inputs are exact strings rather than binary floating-point estimates.

## Real limits and inequalities

```python
from fractions import Fraction
import mathematishia as m

answer = m.limit("(x^2-1)/(x-1)", "x", "1")
assert answer.value == Fraction(2) and answer.excluded_at_point
assert m.limit("1/x", "x", "0", "left").kind == "negative_infinity"
intervals = m.solve_inequality("x^2-4>0").intervals
assert len(intervals) == 2
assert intervals[0].upper.root.rational == Fraction(-2)
```

`limit(source, variable, at, direction="two-sided")` returns a frozen `LimitResult` with exact finite value or explicit signed infinity/nonexistence, source rational function, factor evidence and retained exclusions. The target is an exact rational string; infinite targets and transcendental limits are unsupported.

`solve_inequality(source, variable="x")` returns a frozen `InequalityResult` with exact real interval endpoints (rational or certified quadratic roots), open/closed flags, original-domain exclusions in the residual, and critical point multiplicities. Only one comparison and rational factors of degree at most two are supported. Both operands' exclusions survive cancellation. Existing equation APIs are unchanged.

## Affine systems, polynomial division and root refinement

The following APIs use the existing bounded Rust kernel, with no numerical
approximation or independent Python solver. Existing API signatures are unchanged; Sturm admission now rejects source-restricted endpoints and normalized non-identifiers.

| Python API | Supported domain | Explicit limitations |
| --- | --- | --- |
| solve_linear_system(equations, variables) | Rational affine equalities, at most 16 equations and 16 explicitly ordered distinct variables | No nonlinear terms, unknown symbols or unresolved source restrictions. Exact unique, inconsistent or affine family. |
| divide_multivariate(dividend, divisors, variables) | Ordered lexicographic division over Q in a declared ring of at most 16 variables, 32 divisors, degree 128, bounded terms/scalars/work | No symbolic denominators, source-domain restrictions, zero divisors or undeclared symbols. Quotient order is significant; exponent vectors use the returned sorted-symbol ring. Reduced remainder is not a Groebner normal form. |
| refine_real_root_bracket(source, variable, lower, upper, target_width, max_bisections) | Nonzero Q[x], unrestricted polynomial source, exact rational endpoints and positive width, bounded work | Same-sign intervals are unresolved; a sign change certifies existence only, never uniqueness. Exhausting the bisection count can leave precision_met false. |

Results are frozen dataclasses with tuples and Fraction coefficients, not
floats. For example, solve_linear_system(("x+y=1",), ("x", "y")) returns an
affine family indexed by the declared variable order. Root brackets are
independently replayed; division recomposition and reduced remainder are
checked by the kernel. Original source restrictions are rejected instead of
being silently removed.

The automatic equation dispatcher and bounded exact evaluation are available
as `solveauto` and `evalexact` through the shared versioned result. The existing
individual equation solvers and one-step `analytic_constant` API remain
available. The matched CLI commands are `linsystem`, `mvdivide`,
`rootbracket` and `sturm`.

### CLI/Python interface audit

The table compares the checked mathematical domains, not identical presentation
or command syntax. Existing narrower APIs are intentionally retained.

| CLI | Python | Domain or delivery gap |
| --- | --- | --- |
| expr, domain, subdomain, rewrite, expand | expression, domain_expression, domain_equation, substitute_domain, rewrite_domain, expand | Parsed expressions, retained integer-power conditions, guarded single rewrites, bounded distribution. Plain expression display is not a domain certificate. |
| expandall | expandall, expand_polynomial | Bounded Q-polynomial ring with inherited restrictions; Python requires the ambient variables explicitly, CLI can infer them. |
| solve, solveq, roots2, roots | solve, solve_quadratic, algebraic_roots, polynomial_roots, solve_polynomial_equation | Rational linear, rational degree-two, exact complex degree-two, partial complex polynomial roots. Higher-degree unresolved cofactors remain explicit. |
| solveauto --json | solveauto, problem | Shared checked dispatch, with explicit result domain and partial scope. |
| evalexact --json | evalexact, analytic_constant | Shared bounded multi-identity traversal; the legacy one-step API remains available. |
| integrate | integrate, integrate_expression_dispatch, integrate_polynomial, integrate_polynomial_between, integrate_elementary | Exact Q-polynomial primitives/definite integrals and affine sin/cos/exp sums; general transcendental integration unsupported. Python uses separate definite-integral API and explicit variable. |
| limit, inequality | limit, solve_inequality | Finite rational point limits of Q(x), real rational comparisons with degree-two factors/exclusions; not arbitrary transcendental limits or inequalities. |
| poly, eval, divide, gcd, egcd, modinv, modreduce, modop, modpow, diff, sqfree, factor | polynomial, evaluate, divide, gcd, extended_gcd, modular_inverse, modular_reduce, modular_operation, modular_power, differentiate, square_free, factor_polynomial | Bounded Q[x] arithmetic, quotient rings, verified derivative and factorization with unresolved factors where required. |
| qvalue, qop, qinv, fieldmin, fieldpow, fieldop | quadratic_value, quadratic_operation, quadratic_inverse, extension_minimal_polynomial, extension_power, extension_operation, extension_value | Exact embedded quadratic fields and certified abstract field presentations; no arbitrary algebraic-number construction or mixed embeddings. Python also exposes direct field-element construction. |
| matdet, matinv, matrref, matmul, matsolve | matrix_determinant, matrix_inverse, matrix_rref, matrix_multiply, matrix_solve | Bounded exact rational matrices and affine solution families. |
| diffexpr, diffreal | differentiate_expression, differentiate_expression_real | Checked symbolic derivatives with explicitly retained real-domain prerequisites; no complex logarithm branch assumptions. |
| rsimplify, rdiff, reval, rsolve, rat, rateval, ratdiff, ratsolveq, ratop | rational_simplify, rational_derivative, rational_value, rational_solve, rational_function, rational_evaluate, rational_differentiate, rational_solve_quadratic, rational_operation | Bounded rational functions in one variable and selected low-degree equations; original zero exclusions are not filled. |
| sturm, linsystem, mvdivide, rootbracket | sturm_root_count, solve_linear_system, divide_multivariate, refine_real_root_bracket | Matched checked mathematical scope. Sturm counts square-free nonzero Q[x] on open rational intervals with nonroot endpoints; the other domains are listed above. |
| isolate | isolate_real_roots | Complete disjoint intervals for unrestricted nonzero square-free Q[x]; exact rational midpoint and error bound. |
| Selected commands with --json | problem | One kernel-owned versioned dispatcher and encoder. Unsupported shapes are refused; not every legacy operation has a schema representation. |

Kernel-only arithmetic building blocks, finite-field factorization and Hensel
reconstruction, proof graphs, derivation rendering, and individual certificate
verification routines are not independent user commands. This audit describes the bounded supported operations rather than full mathematical coverage.

## Shared results and complete real-root isolation

```python
from fractions import Fraction
import mathematishia as m

result = m.solveauto("x^2-2=0")
assert result.status == "complete"
assert result.result_domain == "C"
print(result.to_json())
assert m.evalexact("sin(0)+cos(0)").status == "complete"

roots = m.isolate_real_roots("x^2-2", "x", "1/32")
assert len(roots) == 2
assert roots[0].error_bound <= Fraction(1, 64)
```

`problem(*args)` uses the same request grammar and versioned JSON as the CLI
`--json` flag, including roots, expansion, indefinite integration, finite rational
limits, inequalities, symbolic differentiation and exact evaluation. The frozen
wrapper exposes `status`, `result_domain`, `to_json()` and `as_dict()`. JSON
contains exact integer strings and structural algebraic roots, not floats or
trusted deserialized certificates. The legacy typed APIs remain unchanged.

`isolate_real_roots` returns a tuple of frozen brackets with exact Fraction
endpoints, midpoints and half-widths. It certifies complete coverage for nonzero
square-free unrestricted Q[x]. It refuses repeated roots, source restrictions,
and resource exhaustion rather than reporting an incomplete list as complete.

## Checked algebra, angles and derivations

| CLI | Python | Supported mathematical scope |
| --- | --- | --- |
| `rootsfield` | `solve_polynomial_in_domain` | Unrestricted Q[x], explicit Q/R/C solution domain; checked factors and unresolved cofactors. |
| `groebner`, `idealmember` | `groebner_basis`, `ideal_membership` | Formal rational polynomial ideals; at most 3 variables, 8 generators and 12 basis elements. Lexicographic, not necessarily reduced or monic. |
| `trig`, `trigzeros` | `special_angle`, `trig_zero_examples` | Exact rational multiples of pi, supported unit-circle angles; tangent poles undefined, sine/cosine zero examples partial. |
| `abssolve` | `solve_absolute` | One total real affine absolute-value chain per side, at most 8 layers total; complete closed interval unions. |
| `checkcalculus` | `check_calculus` | Verify rational identities or derivatives with retained exclusions; candidate must cover the source domain. |
| `steps` | `derivation_records`, `derivation_records_json` | Checked linear, derivative and distribution records; source restrictions and actual dependencies. |
| `divisiontrace` | `division_trace_json` | Actual ordered polynomial reduction trace, independently verified against the source. |
| `solvefield` | `problem('solvefield', ...)` | Source-bound rational equations over Q/R/C, complete through quadratic residuals; higher-degree unresolved cofactors remain partial. |
| `natural` | `problem('natural', ...)` | Opt-in special angles and affine absolute-value equations; radicals not yet admitted. |

`rootsfield`, `solvefield`, `natural`, `groebner`, `idealmember`,
`trig`, `trigzeros` and `abssolve` expose the shared checked result schema
through CLI JSON and Python `problem(*args)`. Existing text/typed interfaces
remain available. `steps --json` and `divisiontrace` expose separately
verified version-1 trace envelopes; the legacy division result is still final-only.
The ordered trace records actual reductions and remainder moves over unrestricted
Q-polynomials (at most 8 variables/divisors, degree 32, 64 terms and 128 steps).
It does not claim a unique normal form for arbitrary divisors.

Explicit construction-domain provenance is retained on bound values and
checked in the shared dispatcher and selected calculus, substitution and
rewrite paths. Legacy unbound expressions cannot attest a lost construction
domain. The ordinary symbol pi is not silently reinterpreted. Opt-in natural input
supports selected special angles and total affine absolute-value equations. General periodic solution families, arbitrary piecewise
or absolute-value expressions, general analytic calculus and universal result
serialization remain unsupported.

```python
import mathematishia as m

m.solve_polynomial_in_domain('x^2-2', domain='R')
m.groebner_basis(('x*y-1', 'y^2-1'), ('x', 'y'))
m.ideal_membership('x-y', ('x*y-1', 'y^2-1'), ('x', 'y'))
m.special_angle('sin', '1/6')
m.solve_absolute('abs(abs(x)-2)=1')
m.check_calculus('x^2', '2*x', relation='derivative')
m.derivation_records('linear', '2*x+4=10')
```


```python
import json
import mathematishia as m

m.problem('solvefield', '(x^2-1)/(x-1)=3', 'x', 'R')
m.problem('natural', 'sin(pi/6)')
m.problem('natural', '|2x-5|=7')
m.integrate('x*exp(x)', 'x')
json.loads(m.derivation_records_json('linear', '2*x+4=10'))
json.loads(m.division_trace_json('x^2+x*y+y^2+1', ('x+y', 'x'), ('x', 'y')))
```

Polynomial-exponential primitives are checked using the exact polynomial identity Q' + aQ = P. This does not yet emit a complete integration-by-parts teaching trace.
