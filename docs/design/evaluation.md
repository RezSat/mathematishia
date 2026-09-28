# Evaluation

Construction creates semantic values. Normalization cheaply establishes a
deterministic structural form. Evaluation applies an operation in an explicit
context. Rewriting applies selected transformations; simplification is an
explicit mathematical operation with choices rather than one universal normal
form. Algorithm execution invokes a domain or mathematical procedure.

Mathematishia does not use a universal repeated-rewrite evaluator as its central
architecture. These operations remain distinct.

The future `EvaluationContext` will carry assumptions, numeric and precision
policy, algorithm options, a deterministic seed, and resource limits. There is
no mutable global mathematical state. The context is not implemented yet.
