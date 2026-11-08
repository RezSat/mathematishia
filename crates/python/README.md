# Mathematishia Python bindings (experimental)

Install from source with Python 3.10+ and a Rust toolchain:

```sh
python -m pip install ./crates/python
```

The package uses maturin and PyO3 to compile the same Rust kernel used by the
CLI. It offers exact rational results and typed immutable Python result values.

```python
from fractions import Fraction
import mathematishia as m

answer = m.solve("2*x+4=10", "x")
assert answer.kind == "unique" and answer.value == Fraction(3)
assert m.evaluate("x^2+1/2", "x", "1/2") == Fraction(3, 4)
assert m.differentiate("x^3", "x").coefficients == (Fraction(0), Fraction(0), Fraction(3))
assert m.differentiate_expression("x^2", "x").derivative == "2 * x"
assert [factor.multiplicity for factor in m.square_free("(x-1)^2*(x+2)^3", "x").factors] == [2, 3]
print(answer.steps)
```

This is **not yet a general-purpose CAS or stable public Python API**. The
supported Mathema grammar is deliberately small, all computations use explicit
checked kernel contexts, and the limits are not hard memory/CPU bounds.

Supported symbolic derivatives are available via
`m.differentiate_expression("x*y+(x+1)^3", "x")`. The immutable result includes
its derivative and independently checked rule steps. It is deliberately not
a general transcendental-function calculus; negative, zero, fractional and
symbolic exponents are unsupported. Work metering is deterministic and
context-local, not a hard memory/time guarantee.

### Exact rational functions

`rational_function("x^2-1", "x-1", "x")` returns an immutable normalized
numerator, monic denominator and excluded-zero-set polynomials. The functions
`rational_evaluate`, `rational_differentiate` and `rational_operation` share
the same exact Rust domain rules and preserve canceled poles.

Solve exact quadratic roots with
`m.solve_quadratic("x^2-5*x+6", "x")`. The immutable
`QuadraticResult` contains sorted, distinct `Fraction` roots, or an
`all_rationals` classification for the zero polynomial. No rational roots
is not the same as no real or complex solutions.
