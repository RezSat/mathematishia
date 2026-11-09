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
- `differentiate_expression`: formal differentiation of sums, products and positive integer powers, with verified derivation steps.
- `rational_function`, `rational_evaluate`, `rational_differentiate`, `rational_operation`: partial rational functions with recorded domain restrictions.
- `rational_simplify`, `rational_derivative`, `rational_value`, `rational_solve`: full univariate rational expressions and equations over Q, preserving source-domain exclusions.
- `algebraic_roots`: exact quadratic roots, represented as rationals or as a monic minimal polynomial with a branch selector.
- `factor_polynomial`: certified rational linear, quadratic and bounded cubic factors, plus unresolved cofactors.
- `extension_value`, `extension_operation`, `extension_power`, `extension_minimal_polynomial`: arithmetic and minimal polynomials in certified abstract quadratic and cubic fields.

The parser covers a bounded grammar. General function calculus, cross-field arithmetic, higher-degree factorization and a stable Python compatibility contract have not yet been implemented. See [the syntax specification](../../docs/design/mathema-surface.md).

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
