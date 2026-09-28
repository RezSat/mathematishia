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

These are design decisions only; the types are not implemented yet.
