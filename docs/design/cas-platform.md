# Computer algebra architecture

Mathematishia separates syntax, symbolic values, domain-specific algorithms, and verification. The kernel computes exact results and structured derivations; the frontend and language bindings do not implement their own algebra.

## Core representations

- `Expr` is an immutable scalar-expression tree with conservative constructor normalization. Structural equality is distinct from mathematical equivalence.
- `RationalPolynomial` represents a univariate polynomial over Q.
- `RationalFunction` retains the domain exclusions introduced by its original denominators.
- Quadratic algebraic roots use a monic minimal polynomial and a branch selector. General algebraic-number field arithmetic has not been implemented.
- `Condition` and `Assumptions` represent nonzero, sign, integral, real and domain-membership facts. A premise is recorded separately from an exact arithmetic fact.
- `ProofGraph` joins existing expression-rewrite evidence to its source and condition dependencies. Each edge is checked by the corresponding independent rule verifier.

See [CAPABILITIES.md](../../CAPABILITIES.md) for implementation paths.

## Mathematical invariants

1. Every division must preserve its definedness conditions. Algebraically equal rational fractions need not define the same partial function.
2. A transformation records exact input and output values, a stable rule ID, a relation kind, and any required conditions. Differentiation and root classification are not expression equalities.
3. Verification reconstructs the permitted rule application. A rule identifier alone does not establish correctness.
4. Canonical values, error categories and proof ordering are deterministic for fixed inputs and options.
5. Unsupported domains produce explicit errors or bounded partial results.
6. Work budgets meter selected operations; process-level CPU and memory isolation are separate concerns.
7. Mathematical semantics belong to Rust. Python and command-line adapters expose the kernel's results.

## Implementation order

### 1. Exact domains

Extend the quadratic algebraic-root representation to exact arithmetic with field embeddings and equality. Add controlled arithmetic and ordering only for domains with a defined comparison. Extend definedness beyond integer powers and propagate restrictions through parsing.

### 2. Proof and rewriting

The proof graph checks expression inputs, condition dependencies, distributive and guarded cancellation rules, conclusion reachability and node budgets. [Definedness rules](definedness.md) cover integer powers and domain-preserving expression composition. Next:

- Record intermediate obligations without treating unresolved propositions as established facts.
- Extend conditional rewriting to rational-function and analytic-function domains.
- Add proof composition, equation relations, and typed witnesses for domain algorithms.
- Define deterministic step linearization and rendering without making formatting part of verification.

### 3. Polynomial and equation algorithms

Factor univariate polynomials over Q, then introduce higher-degree algebraic solving, elimination, resultants and multivariate polynomial representations. Verify factor recomposition and completeness whenever a result is presented as exhaustive.

### 4. Functions and calculus

Add typed function nodes and domain information for logarithms, trigonometric functions, absolute value and complex branches. Extend differentiation and implement integration with explicit unevaluated results when no supported method applies.

### 5. Numeric domains

Separate exact values from approximations. Introduce precision and rounding policies before certified intervals, root isolation or numerical linear algebra.

### 6. Interfaces and packaging

Stabilize machine-readable result types, preserve proof dependencies in API output, and implement readable explanations. Python releases require a supported platform matrix and reproducible installed-package tests; see [the release checklist](../release/pypi.md).

## Verification

Changes to an algorithm require tests for normal inputs, zero/boundary cases, invalid domains and deterministic output. Evidence-producing operations also require forged-record rejection tests. Cross-check polynomial or algebraic results against independent exact arithmetic when possible.

CI runs Rust formatting, compilation, strict Clippy checks and tests on Linux and Windows; wheel installation and source-distribution reconstruction are tested separately. A passing CI run checks the implementation but does not establish completeness outside documented domains.
