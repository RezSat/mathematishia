# Coprime Hensel lifting and reconstruction

A monic target f in Z[x] has candidate monic factors g and h with

    f = g*h (mod p^k)

and coprime reductions of g and h modulo p. One Hensel step computes
corrections to obtain factors modulo p^(k+1). The implementation uses
exact integer coefficients; no floating-point reconstruction is involved.

`hensel_lift_pair` stores the target, input factors, lifted factors,
prime and both moduli. `verify_hensel_pair` independently checks:

- Valid prime and prime-power modulus, coprime reductions modulo p
- Unchanged factor degrees and monicity
- Preservation of each factor modulo the old modulus
- Product congruence with f modulo the new modulus

The current pair lift accepts monic integer targets of degree at most
eight, with a bounded number of prime-power steps. It is a coprime
two-factor procedure, not general multifactor lifting.

`modular_reconstruction` attempts selected modular bipartitions,
lifts the two factors and forms centered integer representatives.
The proposed integer factor is accepted only when exact division in
Q[x] has zero remainder. The factorization entry point subsequently
checks the complete product and the irreducibility of its certified
factors.

A failed bounded reconstruction is inconclusive; it does not establish
irreducibility.

Sources: `crates/kernel/src/hensel_lift.rs`,
`modular_reconstruction.rs`.
