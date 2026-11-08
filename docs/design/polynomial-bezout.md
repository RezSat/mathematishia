# Extended Euclidean algorithm in Q[x]

`EvaluationContext::extended_gcd(a,b)` returns a monic greatest common
divisor `g` and coefficient polynomials `s,t` satisfying

```
s*a + t*b = g
```

The Euclidean record contains `(dividend, divisor, quotient, remainder)`
for every division. The verifier checks that each step satisfies

```
dividend = quotient*divisor + remainder
degree(remainder) < degree(divisor)
```

The chain must terminate at a zero remainder. The final nonzero remainder,
normalized to leading coefficient one, must match `g`. The verifier
checks the Bézout identity independently of the algorithm that generated
the coefficients.

For `a=b=0`, the result is `g=s=t=0` with an empty division record.
When one input is zero, the nonzero input is normalized to monic form.

`EvaluationContext::inverse_mod_polynomial(a,m)` requires a nonzero
modulus. When `gcd(a,m)=1`, the result is `s mod m`; otherwise it is
`None`. The exact division algorithm reduces the inverse to degree
below the modulus. The context checks input and output polynomial limits.

Interfaces:

- Rust: `extended_gcd`, `inverse_mod_polynomial`, `verify_extended_gcd`
- CLI: `egcd`, `modinv`
- Python: `extended_gcd`, `modular_inverse`

Implementation: `crates/kernel/src/extended_gcd.rs`. Tests cover zero
inputs, noncoprime inputs, exact modular inverses, deterministic Bézout
identities, and rejection of modified proof records.
