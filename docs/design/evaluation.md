# Evaluation

Construction creates semantic values. Normalization cheaply establishes a
deterministic structural form. Evaluation applies an operation in an explicit
context. Rewriting applies selected transformations; simplification is an
explicit mathematical operation with choices rather than one universal normal
form. Algorithm execution invokes a domain or mathematical procedure.

Mathematishia does not use a universal repeated-rewrite evaluator as its central
architecture. These operations remain distinct.

An initial immutable `EvaluationContext` now carries explicit admission limits:
expression occurrences, depth, scalar magnitude bits, polynomial degree and
maximum distribution steps. Checked APIs validate inputs and results at
supported operation boundaries and independently verify derivations. These
checks are NOT hard memory, wall-clock or intermediate-allocation limits;
existing low-level APIs remain available for trusted Rust callers. Assumptions,
numeric/precision policy and deterministic seeds are future extensions.
There is no mutable global mathematical state.

Kernel inspection uses the read-only semantic expression view. Single-symbol
substitution is an explicit single-pass transformation rebuilt through the
existing constructors. No general rewrite or derivation system exists yet.

## Metered checked arithmetic (Task 19)

The immutable evaluation context now includes `max_work_units` and
`max_intermediate_scalar_bits`. Checked polynomial conversion, evaluation,
formal differentiation, long division, Euclidean GCD, square-free generation
and square-free verification share a **per-call** metered exact-arithmetic path.
Each scalar arithmetic operation charges deterministic units, and conservative
numerator/denominator bit bounds are inspected *before* large integer
products are allocated. Checked distribution and linear-solving trace
verification use conservative work precharges before invoking their legacy
structural generators/checkers. Work budgets are deliberately not elapsed-time
budgets, and conservative bit bounds may reject results with heavy cancellation.

The parser and the low-level `Expr` constructors run **before** an evaluation
context admits their outputs. Low-level Rust APIs remain unmetered for trusted
callers. General arbitrary-precision operations, allocator behavior, OS process
memory, wall-clock time and third-party cancellation require a separate
execution isolation mechanism. No hard resource safety is claimed here.
