# Exact rational matrices

`ExactMatrix` stores a nonempty rectangular array of canonical rationals.
Both dimensions are capped at 16. Entry sizes are checked against the
evaluation context; no arbitrary floating-point pivots are used.

## Operations

- Matrix multiplication and transposition use exact rational arithmetic.
- Reduced row echelon form uses full Gauss–Jordan elimination with nonzero
  rational pivots. The output stores the pivot columns and rank.
- Determinants use exact forward elimination with row-swap sign tracking.
  A singular square matrix has determinant zero.
- Matrix inversion augments with the identity and applies Gauss–Jordan
  elimination. The returned candidate must independently satisfy both
  A * B = I and B * A = I by exact matrix multiplication.
- Solving Ax=b supports rectangular coefficient matrices. The result
  distinguishes inconsistent, unique and infinite solution sets.
  Infinite solutions contain a particular vector and a basis of the
  homogeneous nullspace, giving x = particular + sum(t_i * basis_i).

`verify_linear_system_vector` checks a proposed vector against every
original equation without using the reduction result. Verification of
the inverse is likewise independent of the inverse elimination path.

The exact row-reduction implementation is not a general proof system for
every row operation. Formal per-row-operation proof certificates and
unbounded dimensions remain separate work.

Source: `crates/kernel/src/exact_matrix.rs`.
Tests: `crates/kernel/tests/exact_matrix.rs`,
`crates/cli/tests/exact_matrix_commands.rs`,
`crates/python/tests/test_package.py`.
