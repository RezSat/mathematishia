# Abstract algebraic extensions

`CertifiedAlgebraicField` presents Q[alpha] using a monic irreducible defining
polynomial of degree two or three. Its constructor checks irreducibility using
the exact rational factorizer. If the factorizer cannot certify the defining
polynomial, construction fails. There is no default root selection or complex
embedding.

Elements are canonical coefficient polynomials of degree less than the
defining polynomial. Their arithmetic uses exact quotient-ring operations.
A nonzero element in a certified field has an inverse; the inverse is computed
using the extended polynomial GCD. Operations between different defining
polynomials are rejected, even when the corresponding number fields are
abstractly isomorphic.

## Minimal polynomials

For an element a of an extension of degree n, the powers
1, a, ..., a^n are linearly dependent over Q. The kernel solves the exact
rational linear systems for the least degree dependency and constructs a
monic annihilating polynomial.

The verifier separately evaluates the proposed polynomial at the element in
the quotient ring, checks its degree and monicity, and certifies
irreducibility over Q using the rational factorizer. If the proof cannot
be completed under the factorization bounds, the minimal-polynomial
operation returns an explicit error rather than claiming success.

## Interfaces

Rust: `CertifiedAlgebraicField`, `AlgebraicExtensionElement`,
`verify_certified_algebraic_field`, `verify_extension_minimal_polynomial`.

CLI: `fieldmin`, `fieldpow`, `fieldop`.
Python: `extension_value`, `extension_operation`,
`extension_power`, `extension_minimal_polynomial`.

The embedded quadratic API is separate. Cross-field composita, compatible
complex embeddings, general root isolation and higher-degree extension
certification are not implemented.

Sources: `crates/kernel/src/algebraic_extension.rs`,
`extension_minimal.rs`, `polynomial_residue.rs`,
`rational_factorization.rs`.
