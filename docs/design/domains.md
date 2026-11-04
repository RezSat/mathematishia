# Mathematical domains

Generic expressions are not universal algorithm representations. Specialized
domains will own the data structures and algorithms suited to their
mathematics. Candidate domains include ZZ, QQ, polynomial rings, matrix
domains, formal series, and numerical domains.

Conversions between semantic expressions and specialized domains are explicit
and fallible. Promotion and coercion should follow concrete requirements; this
project is not introducing a general category or coercion system now.

The initial implementation direction is pure Rust. Mature native libraries
such as FLINT may be evaluated behind suitable boundaries if measurements
justify them. No backend trait is warranted solely because multiple backends
might exist later.

The first specialized domain is RationalPolynomial in the kernel: an exact
univariate polynomial over QQ with an implicit indeterminate. Dense
coefficients are stored in ascending degree order, with all trailing zero
coefficients removed (the zero polynomial stores no coefficients). Addition,
subtraction, quadratic-time coefficient convolution, and Horner evaluation are
exact. This domain neither parses nor owns Expr; conversion from expressions
will be explicit and fallible when implemented.

`RationalPolynomial::try_from_expr` converts supported exact scalar expressions
in one selected `Symbol` with an explicit maximum degree/power budget. It rejects
other symbols, non-positive or non-integer powers (including ambiguous `0^0`),
and values beyond the bound. `to_expr` constructs a semantic expression using
the selected indeterminate; the round trip guarantees polynomial value, not
identical expression structure. Conversion does not modify `Expr` normalization.

Polynomial long division over QQ returns an exact quotient and remainder,
rejecting zero divisors; the remainder is zero or has strictly lower degree.
Euclidean GCD repeatedly uses that division, then normalizes the result to
monic form. GCD(0,0) is defined as zero, and other constant GCDs equal one.
These operations are specialized algorithms, not generic expression rewrites.

Formal differentiation of the rational-polynomial domain applies the exact
coefficient power rule: the coefficient of x^(i-1) in p'(x) equals i times
the coefficient of x^i in p(x). Constants and zero differentiate to canonical
zero. `RationalPolynomial::differentiate` does not construct generic symbolic
steps, and the context-checked operation enforces input/output resource
admission. General symbolic calculus and a justified differentiation-step
format remain separate future work.
