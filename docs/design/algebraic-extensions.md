# Abstract algebraic extensions

`CertifiedAlgebraicField` constructs Q[alpha] from a monic defining
polynomial whose irreducibility over Q has been certified. The current
degree range is 2 through 8. Its certificate is independently checked
before the field is admitted.

An element is represented by a polynomial of degree less than the
defining polynomial. Arithmetic uses exact polynomial quotient
operations; inverses use the extended Euclidean algorithm. An
operation between elements with different defining polynomials fails
rather than silently identifying field isomorphisms.

The type does not select a real or complex embedding. The
`QuadraticField` representation handles the embedded quadratic case
separately.

## Minimal polynomials

For an element a in a degree-n extension, 1, a, ..., a^n are
linearly dependent over Q. The kernel constructs the first monic
annihilator found by exact rational elimination, then checks it
independently: substitution into the quotient ring must give zero,
and the candidate must be certified irreducible over Q.

For larger degrees, certification can fail because the rational
factorizer is bounded. In that case the operation returns an error,
not an unproved minimal polynomial.

## Interfaces

Rust: `CertifiedAlgebraicField`, `AlgebraicExtensionElement`,
`verify_certified_algebraic_field`, `verify_extension_minimal_polynomial`.

CLI: `fieldmin`, `fieldpow`, `fieldop`.
Python: `extension_value`, `extension_operation`,
`extension_power`, `extension_minimal_polynomial`.

Cross-field composita, compatible embeddings and arbitrary higher-degree
field construction are not yet implemented.
