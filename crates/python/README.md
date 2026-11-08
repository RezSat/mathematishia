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
