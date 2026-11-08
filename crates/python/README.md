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
- `evaluate`, `differentiate`, `square_free`: exact rational-polynomial operations.
- `differentiate_expression`: formal differentiation of sums, products and positive integer powers, with verified derivation steps.
- `rational_function`, `rational_evaluate`, `rational_differentiate`, `rational_operation`: partial rational functions with recorded domain restrictions.
- `algebraic_roots`: exact quadratic roots, represented as rationals or as a monic minimal polynomial with a branch selector.

The parser covers a bounded grammar. General function calculus, arbitrary algebraic-number arithmetic, and a stable Python compatibility contract have not yet been implemented. See [the syntax specification](../../docs/design/mathema-surface.md).
