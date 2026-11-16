# Multivariate polynomial division over Q

`divide_multivariate` works on `MultivariatePolynomial` values belonging to
the same explicit ambient ring. Variables are ordered lexicographically by
symbol name. Term order is lexicographic on exponent vectors in that order.

## Division procedure

For dividend `f` and nonzero ordered divisors `g[0..n]`, the algorithm
repeatedly examines the leading term of the pending polynomial:

1. Use the first divisor whose leading monomial divides the pending leading
   monomial. Subtract the corresponding exact term multiple and add that
   multiple to the matching quotient.
2. If no leading monomial divides, move the pending leading term into the
   remainder.
3. Continue until the pending polynomial is zero, or return a resource error.

The output contains one quotient per divisor, in caller-supplied order, plus
the remainder and the number of iterations. A reduction step must strictly
decrease the pending leading monomial. Exact rationals are used throughout.

## Verification and limitations

`verify_multivariate_division` independently reconstructs
`f = g[0]*q[0] + ... + g[n-1]*q[n-1] + r` and requires that no monomial
of `r` is divisible by any divisor's leading monomial. It checks divisor
validity, quotient arity, ambient rings and the recorded step bound.

The procedure accepts at most 32 divisors and 8192 reduction steps, within
the existing multivariate coefficient, term, degree and work limits.
No partial output is certified after a resource error. Division priority
affects quotients and remainders. The result is **not** a unique normal form
modulo an arbitrary generating set; Gröbner-basis construction and uniqueness
certificates are separate operations.

Source: `crates/kernel/src/multivariate_division.rs`.
Tests: `crates/kernel/tests/multivariate_division.rs`.
