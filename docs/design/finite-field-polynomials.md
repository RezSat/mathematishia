# Finite-field polynomials

`FiniteFieldPolynomial` stores canonical ascending coefficients over
F_p. The constructor checks that p is prime and no greater than 251.
Input degree is at most 32; multiplication allows bounded intermediates
of degree 64 to support modular reduction.

Implemented operations are addition, subtraction, scaling, multiplication,
division with remainder, monic normalization, Euclidean GCD, formal
derivative, modular exponentiation and Rabin's irreducibility criterion.

`factor_finite_field` enumerates monic divisors in degree order for
admitted inputs of degree at most eight. The number of divisor candidates
is capped at 20000. A factor is labelled irreducible only when the
independent Rabin test succeeds. Any cofactor not fully resolved under
these limits is retained in `unresolved` rather than marked irreducible.
`verify_finite_field_factorization` checks the exact modular product,
multiplicities and labels.

`modular_irreducibility_witness` searches a fixed set of small primes.
It requires a degree-preserving reduction of the primitive integer
polynomial. The certificate verifier recomputes that reduction and
rechecks irreducibility. No witness means unknown, not reducible.

Sources: `crates/kernel/src/finite_field.rs`,
`finite_field_factorization.rs`, `modular_irreducibility.rs`.
