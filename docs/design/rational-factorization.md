# Rational factorization over Q

`RationalFactorization` stores an exact rational unit, monic irreducible
factors with multiplicities and unresolved monic cofactors with
multiplicities. `is_complete` is true only when no unresolved factors remain.

## Algorithms

The kernel performs square-free decomposition first. It then applies:

- Rational-root extraction using primitive integer coefficients and the
  rational-root theorem (up to 100000 in primitive coefficient magnitude and
  4096 candidate roots).
- Exact discriminant tests for quadratics; exhaustive rational-root tests
  certify admitted cubics.
- Exhaustive search for monic quadratic divisors of monic integral quartics
  when the Cauchy coefficient bound and search count fit the configured caps.
- Degree-preserving reduction modulo small primes and the finite-field
  Rabin irreducibility test. Irreducibility modulo one prime proves
  irreducibility over Q; failure to find such a prime proves nothing.
- Bounded modular factorization, coprime Hensel pair lifts and centered
  integer reconstruction for selected monic integral inputs through degree
  eight. Reconstructed factors are accepted only after exact long division
  returns zero remainder.

The modular reconstruction step probes primes 2, 3, 5, 7 and 11, with
a maximum of eight prime-power trials for each candidate split. The
current implementation is deliberately bounded and does not implement
general coefficient-bound-driven recombination or complete factorization
of arbitrary rational polynomials.

## Verification

`verify_rational_factorization` checks the leading unit, factor
monicity, positive multiplicities, distinct factors, exact product and
every claimed irreducibility. It rejects claims without a supported
irreducibility proof. Unresolved factors participate in product checking
but carry no irreducibility claim.

The finite-field factorizer verifies its product and irreducibility labels
independently. Each Hensel lift also verifies the original and lifted
congruences, degree/monicity invariants and coprime reductions.

Source files:
`crates/kernel/src/rational_factorization.rs`,
`finite_field.rs`, `finite_field_factorization.rs`,
`quartic_factor.rs`, `modular_irreducibility.rs`,
`hensel_lift.rs`, `modular_reconstruction.rs`.
