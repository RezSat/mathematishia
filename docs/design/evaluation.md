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
