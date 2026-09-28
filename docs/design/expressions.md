# Expressions

Semantic expressions are immutable. Callers will use an opaque `Expr`
abstraction; its storage is not semantic API. Structural equality and
mathematical equivalence are distinct operations.

The first semantic scope is Integer, Rational, Symbol, Add, Mul, and Pow. This
is not a commitment to a large universal enum for future features. Equations,
inequalities, and logical statements need not be forced into a scalar
expression type.

Normalization is conservative and deterministic. Structural ordering must be
deterministic. Future serialization must not expose memory addresses, arena or
interner IDs, or cached hashes. Free symbol identity begins deterministic and
name-based; scoped or bound-variable identity remains a future design problem
and must not be simulated with random IDs.

`mathema` now implements `Integer` and `Rational` as standalone scalar values,
not expression nodes. `Integer` privately stores an arbitrary-precision
`num-bigint` integer. `Rational` privately stores an `Integer` numerator and
denominator, reduced to coprime terms with a positive denominator; zero is
always `0/1`. Its fallible constructor rejects zero denominators with the
Mathema-owned `ZeroDenominator` error. Accessors borrow Mathema scalar types.

Both values support exact negation, addition, subtraction, multiplication,
value equality, numerical total ordering, and equality-consistent hashing.
An `Integer` converts directly to a `Rational` with denominator one. Division
and general coercion are not implemented. Basic `Display` prints integers in
decimal and rationals as `n/d`, omitting `/1`; this is human formatting, not a
stable serialization contract. Symbolic expression types remain unimplemented.
