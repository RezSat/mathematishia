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
assert [factor.multiplicity for factor in m.square_free("(x-1)^2*(x+2)^3", "x").factors] == [2, 3]
print(answer.steps)
```

This is **not yet a general-purpose CAS or stable public Python API**. The
supported Mathema grammar is deliberately small, all computations use explicit
checked kernel contexts, and the limits are not hard memory/CPU bounds.
