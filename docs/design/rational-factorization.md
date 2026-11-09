# Rational polynomial factorization

The factorization result stores an exact rational unit, a list of certified
irreducible monic factors with positive multiplicities, and a list of monic
unresolved cofactors with multiplicities. A result is complete exactly when
the unresolved list is empty.

The algorithm first performs verified square-free decomposition. It then
extracts rational linear factors, uses the discriminant test for quadratic
irreducibility, and uses the rational root theorem for cubics. A cubic is
irreducible over Q exactly when it has no rational root.

## Bounded root enumeration

For a polynomial with rational coefficients, denominators are cleared using
an exact integer LCM. The coefficients are divided by their integer content
to form the primitive integer polynomial. A rational root p/q in lowest terms
must have p dividing the constant term and q dividing the leading term.

The current exhaustive enumeration runs only when every primitive coefficient
has absolute value at most 100000 and the distinct candidate set contains at
most 4096 values. If those bounds cannot be met, the component remains
unresolved. Rational roots at zero can be extracted without enumeration.

Quadratic irreducibility is decided by an exact rational discriminant square
test and is not subject to the enumeration bound. Irreducible factors of degree
four or higher are not certified by this algorithm. A quartic without rational
roots is not automatically irreducible.

## Verification

`verify_rational_factorization` checks the unit against the leading
coefficient, monicity, positive multiplicities, factor uniqueness, exact
reconstruction and each claimed irreducibility. Unsupported irreducibility
claims are rejected. Unresolved factors are included in reconstruction but
are not represented as certified irreducibles.

The evaluation context checks admitted polynomials and divides. Candidate
enumeration has explicit bounds; these limits do not constitute a wall-clock
or memory cap.

Sources: `crates/kernel/src/rational_factorization.rs`,
`square_free.rs`, `crates/mathema/src/number/integer.rs`.
