# Embedded quadratic fields

A field presentation consists of an irreducible monic polynomial

    f(x) = x^2 + b*x + c

over Q, together with a selected root branch. For a quadratic, irreducibility
over Q is equivalent to the discriminant `b^2-4c` not being a square in Q.
The branch is `lower_real` or `upper_real` for positive discriminant, and
`negative_imaginary` or `positive_imaginary` for negative discriminant.

An element has the exact coordinate form `a + d*alpha`, where
`alpha^2 = -b*alpha-c`. Addition and multiplication operate on these rational
coordinates. The inverse of a nonzero element uses its conjugate and norm.

For `z=a+d*alpha`:

    trace(z) = 2*a-b*d
    norm(z) = a^2-a*b*d+c*d^2

If `d=0`, the minimal polynomial is `x-a`. Otherwise it is

    x^2 - trace(z)*x + norm(z).

The returned minimal polynomial is checked independently by substitution into
Q[x]/(f). Its degree is checked against whether the element is rational. The
resulting exact root identity carries a normalized minimal polynomial and the
branch implied by the original embedding; multiplying an irrational generator
by a negative rational reverses the branch.

Different branches or different base polynomials are different field
presentations in this API. Operations between them are rejected rather than
silently identifying field isomorphisms.

Sources: `crates/kernel/src/quadratic_field.rs`,
`crates/kernel/src/algebraic.rs`, `crates/kernel/src/polynomial_residue.rs`.
