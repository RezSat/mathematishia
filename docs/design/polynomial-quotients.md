# Polynomial quotient rings

`PolynomialResidue` represents an element of Q[x]/(m), where m has positive
degree. The constructor makes the modulus monic and reduces the representative
to degree less than deg(m). Equality compares both canonical parts. Multiplying
the modulus by a nonzero rational therefore does not change the residue.

Addition, subtraction and multiplication reduce the result modulo m. Inversion
uses the extended Euclidean algorithm and exists exactly when gcd(a,m)=1.
`pow_signed` returns no result for a negative exponent of a nonunit.

The exact congruence check `verify_residue_product` verifies that the proposed
product differs from the direct polynomial product by a multiple of m. It is
independent of the product constructor, though both use the exact Euclidean
division implementation.

## Ring versus field

An arbitrary nonconstant m need not be irreducible. For example, in
Q[x]/(x^2-1), the nonzero residue classes of x-1 and x+1 multiply to zero.
The quotient-ring API makes no field claim and returns `None` for the inverse
of a nonunit.

The implementation uses `EvaluationContext` checks before and after admitted
operations. Intermediate polynomial degree and scalar size remain subject to
the context limits; those checks do not impose a wall-clock or memory cap.

Source: `crates/kernel/src/polynomial_residue.rs`.
