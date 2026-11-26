# Shared problem request and result, schema v1

The kernel owns dispatch and serialization. Frontends parse their input using the
shared parser and construct a typed `ProblemRequest`. They must preserve the
original `DomainExpression` (including conditions), rather than reconstructing it
from a simplified display string. `dispatch_problem` selects the existing checked
algorithm, then publishes its independently verified result. Neither frontend
should reimplement that algorithm.

## Operations

| Operation ID | Statement | Supported domain and scope |
| --- | --- | --- |
| `equation.linear` | Equation | Q, univariate linear |
| `equation.rational_linear` | RationalEquation | Q, restricted rational equation |
| `polynomial.rational_roots_degree_two` | PolynomialZero | Q, degree at most two |
| `polynomial.complex_roots_partial` | PolynomialZero | C, checked roots and unresolved cofactor |
| `equation.auto` | SourceEquation | Q: rational quadratic; C: unrestricted polynomial, otherwise rational-only partial result |
| `polynomial.expand` | Expression | Q/R/C, bounded polynomial in explicit ordered variables |
| `calculus.differentiate` | Expression | Existing verified differentiation scope |
| `calculus.integrate` | Expression | Q/R/C, existing polynomial and supported elementary antiderivatives |
| `calculus.rational_limit` | Limit | R, rational expression at a finite rational point; left, right or two-sided |
| `inequality.rational` | Comparison | R, existing rational sign decomposition and exact algebraic bounds |
| `rational.evaluate` | RationalEvaluation | Q, rational point |
| `analytic.evaluate_exact` | Expression | R, no variables, bounded checked analytic identities and exact literals |

See `ProblemOperation::stable_id` for the authoritative IDs. The original
request and the result domain are separate fields. An empty rational root set
never establishes the absence of complex roots. The two partial forms distinguish
an unresolved polynomial cofactor from a restricted answer scope or unfinished
exact rewrite. Antiderivatives are primitives, not a claim about arbitrary
integration or definite integrals. No general transcendental root solving,
symbolic parameter solving, or numerical approximation is introduced.

## Wire format

`ProblemRequest::to_json` emits a versioned object containing the request.
`ProblemResult::to_json` emits an object with keys in this order:
`schema_version` (1), `request`, `result_domain`, `outcome`, `restrictions`,
`evidence`. The nested request contains `operation`, `statement`, `variables`,
`domain`. Domain IDs are Z, Q, R, C. Statements and expression nodes use
structural `kind` tags. Integers are decimal strings; rationals have decimal
string numerator and denominator, not floating point numbers. Polynomial
coefficients are arrays in ascending degree order. Algebraic roots retain their
minimal polynomial and branch; intervals retain their endpoints and closure
flags. The wire encoder is the authoritative field/tag definition.

Outcome statuses: `complete`, `conditional`, `partial`, `no_solutions`,
`all_admissible`, `unsupported`, `undefined`, `resource_refusal`. Complete
and conditional outcomes have an answer; partial outcomes have an answer and
either an unresolved polynomial or a reason. Unsupported, undefined and resource
refusal have a reason and no proof marker. Evidence is null or an object with
`checked_by` and actual verified step references. A proof marker is not a
serialized certificate and cannot be used to construct a trusted result.

Restrictions include original expression conditions and excluded polynomial zero
sets, even where cancellation removes a denominator or a limit exists at a hole.
Use source-preserving statements for parsed inputs; the lower-level Equation and
PolynomialZero statements cannot recover restrictions lost by the caller.

Serialization is output-only, deterministic for the same typed value, and
bounded (2 MiB output, expression depth 128, scalar bit length 16384).
Serialization failure is separate from mathematical resource refusal. No decoder
or floating-point conversion is supplied. The existing configurable algorithm
budgets remain in force; exact analytic rewrites are capped at 16. Frontends
should present unsupported and partial outcomes explicitly, never as success
without qualification.
