# Expressions

Semantic expressions are immutable. Callers use an opaque `Expr`
abstraction; its storage is not semantic API. Structural equality and
mathematical equivalence are distinct operations.

The implemented semantic scope is Integer, Rational, Symbol, Add, Mul, and Pow. This
is not a commitment to a large universal enum for future features. Equations,
inequalities, and logical statements need not be forced into a scalar
expression type.

Normalization is conservative and deterministic. Structural ordering must be
deterministic. Future serialization must not expose memory addresses, arena or
interner IDs, or cached hashes. Free symbol identity begins deterministic and
name-based; scoped or bound-variable identity remains a future design problem
and must not be simulated with random IDs.

`mathema` implements `Integer` and `Rational` as standalone scalar values
that can also be lifted into expressions. `Integer` privately stores an arbitrary-precision
`num-bigint` integer. `Rational` privately stores an `Integer` numerator and
denominator, reduced to coprime terms with a positive denominator; zero is
always `0/1`. Its fallible constructor rejects zero denominators with the
Mathema-owned `ZeroDenominator` error. Accessors borrow Mathema scalar types.

Both values support exact negation, addition, subtraction, multiplication,
value equality, numerical total ordering, and equality-consistent hashing.
An `Integer` converts directly to a `Rational` with denominator one. Division
and general coercion are not implemented. Basic `Display` prints integers in
decimal and rationals as `n/d`, omitting `/1`; this is human formatting, not a
stable serialization contract.

`Symbol` stores a private non-empty name; `EmptySymbolName` rejects only the
empty string. Identity, hashing, and lexicographic ordering use the exact name,
without Unicode normalization. Its display returns that name unchanged.

The current private `Expr` implementation uses `Arc<ExprNode>` for shared
immutable nodes. Neither the node enum nor the storage mechanism is public;
this choice remains replaceable. Equality and hashing compare structure and
values, never addresses. No public tree-inspection API exists.

`From` conversions lift Integer, Rational, Symbol, and primitive integers.
Rationals with denominator one become integer expressions, so exact scalars
have one canonical expression representation. Standalone Rational is unchanged.

`Expr::add` and `Expr::mul` flatten their respective nested forms and fold exact
scalar operands. Sums remove zero; products remove one and collapse to zero
when a factor is zero. Empty sums/products are zero/one; singletons become
their element. Remaining operands are sorted by a private structural comparator:
scalar, symbol, power, product, sum. Scalars compare numerically, symbols by exact
name, and composites recursively and lexicographically (power base before
exponent). This does not define a public mathematical order for Expr.

`Expr::pow` only removes an exact integer exponent of one. Zero exponents,
numeric powers, and nested powers are preserved. Constructors do not collect
terms, combine symbolic factors or powers, expand, factor, or cancel. Structural
equality is not a test for general mathematical equivalence.

Expression display uses explicit ASCII `+`, `*`, and `^`, with parentheses for
precedence, nested powers, and fractional or negative power operands. Symbol
names that could resemble numeric or operator syntax are quoted and escaped
within expressions; this display convention defines no future parser grammar.
There is no parser, substitution, rewriting, simplification engine, solver,
serialization, or user-facing CAS API. Generic expressions remain the symbolic
interchange/orchestration layer, not every future algorithm's representation.
