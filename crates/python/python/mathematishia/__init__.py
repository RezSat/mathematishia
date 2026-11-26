"""Exact Mathematishia operations backed by Rust.

This package exposes a narrow rational algebra domain, not a full CAS.
Numeric results use fractions.Fraction, never floating-point approximation.
"""

from __future__ import annotations

from dataclasses import dataclass
from fractions import Fraction
from typing import Literal

from . import _native
from ._native import (MathSyntaxError, ResourceLimitError,
                      UnsupportedMathematicsError, UndefinedAtPointError,
                      VerificationFailure)


@dataclass(frozen=True)
class Step:
    before: str
    after: str
    rule: str
    verified_condition: str | None = None


@dataclass(frozen=True)
class Expansion:
    expression: str
    complete: bool
    steps: tuple[Step, ...]


@dataclass(frozen=True)
class LinearResult:
    kind: Literal["unique", "none", "all"]
    value: Fraction | None
    steps: tuple[Step, ...]


@dataclass(frozen=True)
class QuadraticResult:
    kind: Literal["finite", "all_rationals"]
    roots: tuple[Fraction, ...]

    @property
    def has_rational_roots(self) -> bool:
        return self.kind == "all_rationals" or bool(self.roots)


@dataclass(frozen=True)
class Polynomial:
    expression: str
    coefficients: tuple[Fraction, ...]

    @property
    def degree(self) -> int | None:
        """Degree is undefined for the zero polynomial."""
        return len(self.coefficients) - 1 if self.coefficients else None


@dataclass(frozen=True)
class ExactQuadraticRoot:
    kind: Literal["rational", "real_quadratic", "complex_quadratic"]
    rational: Fraction | None
    minimal_polynomial: Polynomial | None
    branch: Literal[
        "lower_real", "upper_real", "negative_imaginary", "positive_imaginary"
    ] | None


@dataclass(frozen=True)
class AlgebraicRootStep:
    rule: str
    monic_polynomial: Polynomial | None
    discriminant: Fraction | None


@dataclass(frozen=True)
class AlgebraicRootsResult:
    kind: Literal["finite", "all_complex"]
    roots: tuple[ExactQuadraticRoot, ...]
    steps: tuple[AlgebraicRootStep, ...]


@dataclass(frozen=True)
class SymbolicDifferentiationStep:
    before: str
    derivative: str
    rule: str
    relation: Literal["derivative_of"]


@dataclass(frozen=True)
class SymbolicDerivative:
    original: str
    variable: str
    derivative: str
    steps: tuple[SymbolicDifferentiationStep, ...]
    conditions: tuple[str, ...] = ()


@dataclass(frozen=True)
class SquareFreeFactor:
    polynomial: Polynomial
    multiplicity: int


@dataclass(frozen=True)
class SquareFreeDecomposition:
    unit: Fraction
    factors: tuple[SquareFreeFactor, ...]


def _poly(data: tuple[str, list[str]]) -> Polynomial:
    expression_text, coefficients = data
    return Polynomial(expression_text, tuple(Fraction(c) for c in coefficients))


@dataclass(frozen=True)
class RationalFunction:
    numerator: Polynomial
    denominator: Polynomial
    excluded_zero_sets: tuple[Polynomial, ...]


@dataclass(frozen=True)
class RationalQuadraticResult:
    kind: Literal["finite", "all_admissible"]
    roots: tuple[Fraction, ...]
    excluded_zero_sets: tuple[Polynomial, ...]


def _rational(data: tuple[tuple[str, list[str]], tuple[str, list[str]],
                          list[tuple[str, list[str]]]]) -> RationalFunction:
    n, d, excluded = data
    return RationalFunction(_poly(n), _poly(d), tuple(_poly(p) for p in excluded))


@dataclass(frozen=True)
class DomainExpression:
    expression: str
    conditions: tuple[str, ...]


@dataclass(frozen=True)
class DomainProofNode:
    id: int
    kind: str
    value: str
    parents: tuple[int, ...]
    rule: str | None


@dataclass(frozen=True)
class DomainRewrite:
    status: Literal["applied", "blocked", "refuted", "unchanged"]
    expression: str
    conditions: tuple[str, ...]
    outstanding: tuple[str, ...]
    rule: str | None
    proof: tuple[DomainProofNode, ...]


def domain_expression(source: str) -> DomainExpression:
    """Parse an integer-power expression with its original domain restrictions."""
    value, conditions = _native.domain_expression(source)
    return DomainExpression(value, tuple(conditions))


@dataclass(frozen=True)
class DomainEquation:
    left: DomainExpression
    right: DomainExpression


def domain_equation(source: str) -> DomainEquation:
    """Parse an equation with domain restrictions retained on each side."""
    left, right = _native.domain_equation(source)
    return DomainEquation(
        DomainExpression(left[0], tuple(left[1])),
        DomainExpression(right[0], tuple(right[1])),
    )


def substitute_domain(
    source: str, variable: str, replacement: str
) -> DomainExpression:
    """Substitute a symbolic value while retaining all original restrictions."""
    expression_text, conditions = _native.substitute_domain(
        source, variable, replacement
    )
    return DomainExpression(expression_text, tuple(conditions))


def rewrite_domain(source: str, nonzero: tuple[str, ...] = ()) -> DomainRewrite:
    """Attempt one verified inverse cancellation under explicit nonzero premises."""
    status, value, conditions, outstanding, rule, nodes = _native.rewrite_domain(
        source, list(nonzero)
    )
    return DomainRewrite(
        status,
        value,
        tuple(conditions),
        tuple(outstanding),
        rule,
        tuple(
            DomainProofNode(node_id, kind, text, tuple(parents), node_rule)
            for node_id, kind, text, parents, node_rule in nodes
        ),
    )


def expression(source: str) -> str:
    """Parse and return the conservatively canonicalized expression display."""
    return _native.expression(source)


def expand(source: str, max_steps: int = 8) -> Expansion:
    """Return bounded, independently verified distributive steps."""
    text, complete, steps = _native.expand(source, max_steps)
    return Expansion(text, complete, tuple(Step(*step) for step in steps))


def solve(equation: str, variable: str) -> LinearResult:
    """Solve an exact univariate rational-linear equation."""
    kind, value, steps = _native.solve(equation, variable)
    return LinearResult(
        kind,
        Fraction(value) if value is not None else None,
        tuple(Step(*step) for step in steps),
    )


def solve_quadratic(source: str, variable: str) -> QuadraticResult:
    """Exact rational roots of polynomials of degree at most two.

    A finite empty result means no roots in Q, not no real/complex roots.
    """
    kind, roots = _native.solve_quadratic(source, variable)
    return QuadraticResult(kind, tuple(Fraction(root) for root in roots))


@dataclass(frozen=True)
class QuadraticFieldValue:
    field_polynomial: Polynomial
    embedding: Literal[
        "lower_real", "upper_real", "negative_imaginary", "positive_imaginary"
    ]
    constant: Fraction
    coefficient: Fraction
    minimal_polynomial: Polynomial
    root_branch: str | None
    trace: Fraction
    norm: Fraction


def _quad(data) -> QuadraticFieldValue:
    field, branch, coefficients, minimal, root_branch, invariants = data
    a, b = coefficients
    trace, norm = invariants
    return QuadraticFieldValue(
        _poly(field), branch, Fraction(a), Fraction(b),
        _poly(minimal), root_branch, Fraction(trace), Fraction(norm),
    )


@dataclass(frozen=True)
class PolynomialResidue:
    representative: Polynomial
    modulus: Polynomial


def _residue(data) -> PolynomialResidue:
    value, modulus = data
    return PolynomialResidue(_poly(value), _poly(modulus))


def modular_reduce(source: str, modulus: str, variable: str) -> PolynomialResidue:
    """Canonical polynomial remainder in Q[x]/(m)."""
    return _residue(_native.modular_reduce(source, modulus, variable))


def modular_operation(
    operation: Literal["add", "sub", "mul"],
    left: str, right: str, modulus: str, variable: str,
) -> PolynomialResidue:
    """Perform arithmetic in a polynomial quotient ring exactly."""
    return _residue(_native.modular_operation(
        operation, left, right, modulus, variable
    ))


def modular_power(
    source: str, modulus: str, variable: str, exponent: int,
) -> PolynomialResidue | None:
    """Signed powers, returning None when a required inverse does not exist."""
    result = _native.modular_power(source, modulus, variable, exponent)
    return _residue(result) if result is not None else None


@dataclass(frozen=True)
class Matrix:
    rows: tuple[tuple[Fraction, ...], ...]

    @property
    def shape(self) -> tuple[int, int]:
        return len(self.rows), len(self.rows[0])


@dataclass(frozen=True)
class MatrixRref:
    matrix: Matrix
    pivot_columns: tuple[int, ...]

    @property
    def rank(self) -> int:
        return len(self.pivot_columns)


@dataclass(frozen=True)
class MatrixSolution:
    kind: Literal["unique", "infinite", "inconsistent"]
    particular: tuple[Fraction, ...]
    nullspace_basis: tuple[tuple[Fraction, ...], ...]


def _matrix_input(rows) -> list[list[str]]:
    # Reject binary floating-point input rather than silently treating its
    # decimal display as an exact mathematical value.
    result = []
    for row in rows:
        converted = []
        for value in row:
            if isinstance(value, bool) or not isinstance(value, (str, int, Fraction)):
                raise TypeError("matrix entries must be exact integers, fractions or strings")
            converted.append(str(value))
        result.append(converted)
    return result


def _matrix_output(rows) -> Matrix:
    return Matrix(tuple(tuple(Fraction(value) for value in row) for row in rows))


def matrix_determinant(rows) -> Fraction:
    """Exact determinant of a square rational matrix."""
    return Fraction(_native.matrix_determinant(_matrix_input(rows)))


def matrix_inverse(rows) -> Matrix | None:
    """Exact inverse, or None when the square matrix is singular."""
    result = _native.matrix_inverse(_matrix_input(rows))
    return _matrix_output(result) if result is not None else None


def matrix_rref(rows) -> MatrixRref:
    """Reduced row echelon form and exact pivot columns."""
    result, pivots = _native.matrix_rref(_matrix_input(rows))
    return MatrixRref(_matrix_output(result), tuple(pivots))


def matrix_multiply(left, right) -> Matrix:
    """Multiply two rational matrices exactly."""
    return _matrix_output(_native.matrix_multiply(
        _matrix_input(left), _matrix_input(right)
    ))


def matrix_solve(rows, rhs) -> MatrixSolution:
    """Solve Ax=b, including inconsistent and affine solution families."""
    converted = _matrix_input(rows)
    values = _matrix_input([rhs])[0]
    kind, particular, basis = _native.matrix_solve(converted, values)
    return MatrixSolution(
        kind,
        tuple(Fraction(value) for value in particular),
        tuple(tuple(Fraction(value) for value in row) for row in basis),
    )


def quadratic_value(
    polynomial: str, variable: str, branch: str,
    constant: str, coefficient: str,
) -> QuadraticFieldValue:
    """Represent a+b*alpha in a certified quadratic field with a selected embedding."""
    return _quad(_native.quadratic_value(
        polynomial, variable, branch, constant, coefficient
    ))


def quadratic_operation(
    operation: Literal["add", "sub", "mul", "div"],
    polynomial: str, variable: str, branch: str,
    a: str, b: str, c: str, d: str,
) -> QuadraticFieldValue:
    """Combine two elements in the same embedded quadratic field exactly."""
    return _quad(_native.quadratic_operation(
        operation, polynomial, variable, branch, (a, b), (c, d)
    ))


def quadratic_inverse(
    polynomial: str, variable: str, branch: str,
    constant: str, coefficient: str,
) -> QuadraticFieldValue:
    """Return the inverse of a nonzero quadratic algebraic value."""
    return _quad(_native.quadratic_inverse(
        polynomial, variable, branch, constant, coefficient
    ))


def algebraic_roots(source: str, variable: str) -> AlgebraicRootsResult:
    """Return complete exact complex roots through degree two with proof steps.

    Irrational values are represented by an irreducible monic minimal
    polynomial plus a selected real or complex root branch. No floats.
    """
    kind, values, evidence = _native.algebraic_roots(source, variable)
    roots = tuple(
        ExactQuadraticRoot(
            root_kind,
            Fraction(rational) if rational is not None else None,
            _poly(minimal) if minimal is not None else None,
            branch,
        )
        for root_kind, rational, minimal, branch in values
    )
    steps = tuple(
        AlgebraicRootStep(
            rule,
            _poly(monic) if monic is not None else None,
            Fraction(discriminant) if discriminant is not None else None,
        )
        for rule, monic, discriminant in evidence
    )
    return AlgebraicRootsResult(kind, roots, steps)


@dataclass(frozen=True)
class RationalFactor:
    polynomial: Polynomial
    multiplicity: int


@dataclass(frozen=True)
class RationalFactorization:
    unit: Fraction
    factors: tuple[RationalFactor, ...]
    unresolved: tuple[RationalFactor, ...]

    @property
    def complete(self) -> bool:
        return not self.unresolved


def factor_polynomial(source: str, variable: str) -> RationalFactorization:
    """Return certified rational factors and any explicitly unresolved cofactors."""
    unit, factors, unresolved = _native.factor_polynomial(source, variable)
    return RationalFactorization(
        Fraction(unit),
        tuple(RationalFactor(_poly(poly), multiplicity) for poly, multiplicity in factors),
        tuple(RationalFactor(_poly(poly), multiplicity) for poly, multiplicity in unresolved),
    )


@dataclass(frozen=True)
class AlgebraicExtensionValue:
    field_polynomial: Polynomial
    representative: Polynomial


def _extension(data) -> AlgebraicExtensionValue:
    field, value = data
    return AlgebraicExtensionValue(_poly(field), _poly(value))


def extension_value(
    modulus: str, value: str, variable: str
) -> AlgebraicExtensionValue:
    """Exact coordinates in a certified abstract quadratic or cubic extension."""
    return _extension(_native.extension_value(modulus, value, variable))


def extension_operation(
    operation: Literal["add", "sub", "mul", "div"],
    modulus: str, left: str, right: str, variable: str
) -> AlgebraicExtensionValue:
    """Operate within one certified field presentation over Q."""
    return _extension(_native.extension_operation(
        operation, modulus, left, right, variable
    ))


def extension_power(
    modulus: str, value: str, variable: str, exponent: int
) -> AlgebraicExtensionValue:
    """Exact positive or negative powers of a field element."""
    return _extension(_native.extension_power(
        modulus, value, variable, exponent
    ))


def extension_minimal_polynomial(
    modulus: str, value: str, variable: str
) -> Polynomial:
    """Find and verify the monic minimal polynomial over Q."""
    return _poly(_native.extension_minimal_polynomial(modulus, value, variable))


def polynomial(source: str, variable: str) -> Polynomial:
    """Convert a supported expression into the exact Q[x] domain."""
    return _poly(_native.polynomial(source, variable))



def differentiate(source: str, variable: str) -> Polynomial:
    """Return the formal derivative of a supported exact rational polynomial."""
    return _poly(_native.differentiate(source, variable))

def differentiate_expression(source: str, variable: str) -> SymbolicDerivative:
    """Differentiate supported symbolic expressions with verified rule steps.

    Zero, negative, fractional and symbolic powers are rejected rather than
    assigning undocumented domain assumptions.
    """
    derivative, steps = _native.differentiate_expression(source, variable)
    return SymbolicDerivative(
        source,
        variable,
        derivative,
        tuple(SymbolicDifferentiationStep(*step) for step in steps),
    )


def differentiate_expression_real(source: str, variable: str) -> SymbolicDerivative:
    """Differentiate a real function with exact checked steps and prerequisites.

    Conditions are symbolic requirements, not claims that they hold. Complex
    logarithm differentiation is outside this real-domain interface.
    """
    derivative, steps, conditions = _native.differentiate_expression_real(source, variable)
    return SymbolicDerivative(
        source,
        variable,
        derivative,
        tuple(SymbolicDifferentiationStep(*step) for step in steps),
        tuple(conditions),
    )


def square_free(source: str, variable: str) -> SquareFreeDecomposition:
    """Separate exact polynomial multiplicity classes, not irreducible factors."""
    unit, factors = _native.square_free(source, variable)
    return SquareFreeDecomposition(
        Fraction(unit),
        tuple(SquareFreeFactor(_poly(data), multiplicity)
              for data, multiplicity in factors),
    )


def rational_simplify(source: str, variable: str) -> RationalFunction:
    """Reduce a univariate rational expression over Q with original domain holes."""
    return _rational(_native.rational_simplify(source, variable))


def rational_derivative(source: str, variable: str) -> RationalFunction:
    """Differentiate a rational expression without filling removed poles."""
    return _rational(_native.rational_derivative(source, variable))


def rational_value(source: str, variable: str, at: str) -> Fraction:
    """Evaluate a rational expression exactly on its original domain."""
    return Fraction(_native.rational_value(source, variable, at))


def rational_solve(source: str, variable: str) -> RationalQuadraticResult:
    """Return admissible rational roots for equations of residual degree <= 2."""
    kind, roots, excluded = _native.rational_solve(source, variable)
    return RationalQuadraticResult(
        kind,
        tuple(Fraction(root) for root in roots),
        tuple(_poly(condition) for condition in excluded),
    )


def rational_function(numerator: str, denominator: str, variable: str) -> RationalFunction:
    """Reduce Q(x) exactly while retaining the input denominator's zero set."""
    return _rational(_native.rational_function(numerator, denominator, variable))


def rational_solve_quadratic(left_numerator: str, left_denominator: str,
                             right_numerator: str, right_denominator: str,
                             variable: str) -> RationalQuadraticResult:
    """Solve an exact degree-two rational equation over its original domain."""
    kind, roots, excluded = _native.rational_solve_quadratic(
        left_numerator, left_denominator,
        right_numerator, right_denominator, variable,
    )
    return RationalQuadraticResult(
        kind,
        tuple(Fraction(root) for root in roots),
        tuple(_poly(condition) for condition in excluded),
    )


def rational_evaluate(numerator: str, denominator: str,
                      variable: str, at: str) -> Fraction:
    """Evaluate exactly; canceled poles remain undefined."""
    return Fraction(_native.rational_evaluate(numerator, denominator, variable, at))


def rational_differentiate(numerator: str, denominator: str,
                           variable: str) -> RationalFunction:
    """Apply the exact quotient rule without altering the input domain."""
    return _rational(_native.rational_differentiate(numerator, denominator, variable))


def rational_operation(operation: Literal["add", "sub", "mul", "div"],
                       left_numerator: str, left_denominator: str,
                       right_numerator: str, right_denominator: str,
                       variable: str) -> RationalFunction:
    """Add, subtract, multiply or divide Q(x) partial functions exactly."""
    return _rational(_native.rational_operation(
        operation, left_numerator, left_denominator,
        right_numerator, right_denominator, variable,
    ))


def evaluate(source: str, variable: str, at: str) -> Fraction:
    """Evaluate a rational polynomial at an exact rational input string."""
    return Fraction(_native.evaluate(source, variable, at))


def divide(dividend: str, divisor: str, variable: str) -> tuple[Polynomial, Polynomial]:
    """Return exact quotient and remainder in Q[x]."""
    quotient, remainder = _native.divide(dividend, divisor, variable)
    return _poly(quotient), _poly(remainder)


@dataclass(frozen=True)
class EuclideanDivision:
    dividend: Polynomial
    divisor: Polynomial
    quotient: Polynomial
    remainder: Polynomial


@dataclass(frozen=True)
class ExtendedGcd:
    gcd: Polynomial
    left_coefficient: Polynomial
    right_coefficient: Polynomial
    divisions: tuple[EuclideanDivision, ...]


def extended_gcd(left: str, right: str, variable: str) -> ExtendedGcd:
    """Return the monic polynomial gcd, Bézout coefficients and division evidence."""
    g, s, t, steps = _native.extended_gcd(left, right, variable)
    return ExtendedGcd(
        _poly(g),
        _poly(s),
        _poly(t),
        tuple(EuclideanDivision(*(_poly(item) for item in step)) for step in steps),
    )


def modular_inverse(value: str, modulus: str, variable: str) -> Polynomial | None:
    """Return a polynomial inverse modulo the modulus when gcd(value, modulus)=1."""
    result = _native.modular_inverse(value, modulus, variable)
    return _poly(result) if result is not None else None


def gcd(left: str, right: str, variable: str) -> Polynomial:
    """Return the monic greatest common divisor in Q[x]."""
    return _poly(_native.gcd(left, right, variable))



@dataclass(frozen=True)
class CertifiedPolynomialRoot:
    kind: Literal["rational", "real_quadratic", "complex_quadratic"]
    rational: Fraction | None
    minimal_polynomial: Polynomial | None
    branch: str | None
    multiplicity: int


@dataclass(frozen=True)
class PolynomialRootSet:
    polynomial: Polynomial
    kind: Literal["finite", "all_complex"]
    roots: tuple[CertifiedPolynomialRoot, ...]
    unresolved_cofactor: Polynomial | None
    complete: bool


def _partial_roots(data) -> PolynomialRootSet:
    polynomial_data, kind, roots, unresolved, complete = data
    return PolynomialRootSet(
        _poly(polynomial_data),
        kind,
        tuple(
            CertifiedPolynomialRoot(
                root_kind, Fraction(rational) if rational is not None else None,
                _poly(minimal) if minimal is not None else None,
                branch, multiplicity,
            )
            for root_kind, rational, minimal, branch, multiplicity in roots
        ),
        _poly(unresolved) if unresolved is not None else None,
        complete,
    )


def polynomial_roots(source: str, variable: str) -> PolynomialRootSet:
    """Certified complex roots of Q[x], retaining any unresolved cofactor.

    An incomplete result is not a statement that all roots have been found.
    Original domain restrictions must be absent.
    """
    return _partial_roots(_native.polynomial_roots(source, variable))


def solve_polynomial_equation(equation: str, variable: str) -> PolynomialRootSet:
    """Solve a polynomial equality over C with exact, possibly partial evidence.

    Rational expressions with excluded inputs must use a domain-aware solver.
    """
    return _partial_roots(_native.solve_polynomial_equation(equation, variable))


@dataclass(frozen=True)
class SturmRootCount:
    polynomial: Polynomial
    lower: Fraction
    upper: Fraction
    count: int
    lower_variations: int
    upper_variations: int
    sequence: tuple[Polynomial, ...]
    lower_signs: tuple[Literal["negative", "zero", "positive"], ...]
    upper_signs: tuple[Literal["negative", "zero", "positive"], ...]


def sturm_root_count(
    source: str, variable: str, lower: str, upper: str
) -> SturmRootCount:
    """Count distinct real roots in an open rational interval with a certificate.

    Requires a nonzero square-free polynomial and non-root interval endpoints.
    """
    poly, endpoints, counts, chain, signs = _native.sturm_root_count(
        source, variable, lower, upper
    )
    return SturmRootCount(
        _poly(poly), Fraction(endpoints[0]), Fraction(endpoints[1]),
        counts[0], counts[1], counts[2],
        tuple(_poly(term) for term in chain),
        tuple(signs[0]), tuple(signs[1]),
    )


@dataclass(frozen=True)
class PolynomialIntegral:
    integrand: Polynomial
    primitive: Polynomial


def integrate_polynomial(source: str, variable: str) -> PolynomialIntegral:
    """Exact verified antiderivative of unrestricted Q[x], constant set to zero."""
    integrand, primitive = _native.integrate_polynomial(source, variable)
    return PolynomialIntegral(_poly(integrand), _poly(primitive))


@dataclass(frozen=True)
class DefinitePolynomialIntegral:
    integrand: Polynomial
    primitive: Polynomial
    lower: Fraction
    upper: Fraction
    value: Fraction


def integrate_polynomial_between(
    source: str, variable: str, lower: str, upper: str
) -> DefinitePolynomialIntegral:
    """Evaluate a directed integral at rational endpoints with checked evidence."""
    integrand, primitive, a, b, value = _native.integrate_polynomial_between(
        source, variable, lower, upper
    )
    return DefinitePolynomialIntegral(
        _poly(integrand), _poly(primitive),
        Fraction(a), Fraction(b), Fraction(value),
    )


@dataclass(frozen=True)
class AnalyticConstant:
    status: Literal["exact", "unchanged"]
    expression: str
    conditions: tuple[str, ...]
    rule: str | None
    path: tuple[int, ...]


def analytic_constant(
    source: str, domain: Literal["real", "complex"] = "real"
) -> AnalyticConstant:
    """Apply one independently verified zero/one analytic identity, if available.

    An unchanged expression is explicitly not an exact numeric evaluation.
    Conditions are inherited, not automatically discharged.
    """
    status, value, conditions, rule, path = _native.analytic_constant(source, domain)
    return AnalyticConstant(status, value, tuple(conditions), rule, tuple(path))


@dataclass(frozen=True)
class ElementaryIntegral:
    integrand: str
    primitive: str
    conditions: tuple[str, ...]
    rule: str
    input_scale: Fraction
    slope: Fraction
    intercept: Fraction


def integrate_elementary(
    source: str, variable: str, domain: Literal["real", "complex"] = "real"
) -> ElementaryIntegral:
    """Verify one supported rational multiple of sin, cos or exp of affine input."""
    original, primitive, conditions, rule, scale, slope, intercept = (
        _native.integrate_elementary(source, variable, domain)
    )
    return ElementaryIntegral(
        original, primitive, tuple(conditions), rule,
        Fraction(scale), Fraction(slope), Fraction(intercept),
    )



@dataclass(frozen=True)
class MultivariateTerm:
    exponents: tuple[int, ...]
    coefficient: Fraction


@dataclass(frozen=True)
class PolynomialExpansion:
    status: Literal["exact", "conditional"]
    source: DomainExpression
    expanded: DomainExpression
    variables: tuple[str, ...]
    terms: tuple[MultivariateTerm, ...]
    complete: bool
    syntactically_unchanged: bool


def expand_polynomial(
    source: str, variables: tuple[str, ...] | list[str]
) -> PolynomialExpansion:
    """Expand completely in an explicit Q[variables] ring with checked proof.

    The ring is canonicalized into variable-name order. Result conditions are
    original, unproved domain obligations. Unsupported expressions and budget
    refusals raise distinct existing Mathematishia exceptions.
    """
    if isinstance(variables, (str, bytes)):
        raise TypeError("variables must be a sequence of identifiers")
    status, original, expanded, original_conditions, output_conditions, ring, terms, complete, unchanged = (
        _native.expand_polynomial(source, list(variables))
    )
    return PolynomialExpansion(
        status,
        DomainExpression(original, tuple(original_conditions)),
        DomainExpression(expanded, tuple(output_conditions)),
        tuple(ring),
        tuple(MultivariateTerm(tuple(exponents), Fraction(coefficient))
              for exponents, coefficient in terms),
        complete,
        unchanged,
    )


def expandall(
    source: str, variables: tuple[str, ...] | list[str]
) -> PolynomialExpansion:
    """Alias for the fully certified polynomial expansion, unlike stepwise expand."""
    return expand_polynomial(source, variables)


@dataclass(frozen=True)
class AffineIntegrationEvidence:
    input_scale: Fraction
    slope: Fraction
    intercept: Fraction


@dataclass(frozen=True)
class PolynomialExponentialEvidence:
    input_polynomial: Polynomial
    primitive_polynomial: Polynomial
    slope: Fraction
    intercept: Fraction


@dataclass(frozen=True)
class IntegrationTerm:
    kind: Literal["polynomial", "elementary", "polynomial_exponential"]
    source: str
    primitive: str
    rule: str | None
    polynomial: PolynomialIntegral | None
    affine: AffineIntegrationEvidence | None
    derivative_rules: tuple[str, ...]
    polynomial_exponential: PolynomialExponentialEvidence | None = None


@dataclass(frozen=True)
class DispatchedIntegral:
    status: Literal["exact", "conditional"]
    source: DomainExpression
    primitive: DomainExpression
    variable: str
    domain: Literal["rational", "real", "complex"]
    terms: tuple[IntegrationTerm, ...]


def integrate_expression_dispatch(
    source: str,
    variable: str,
    domain: Literal["rational", "real", "complex"] = "real",
) -> DispatchedIntegral:
    """Integrate supported bounded polynomial and affine analytic sums exactly.

    Returns the zero-additive-constant primitive with all source conditions.
    Each term has independently checked polynomial, elementary or polynomial-exponential evidence.
    Other generic analytic products, log, tan and nonlinear arguments are unsupported.
    """
    status, original, primitive, selected_domain, source_conditions, result_conditions, terms = (
        _native.integrate_expression_dispatch(source, variable, domain)
    )
    parsed_terms = []
    for kind, term, term_primitive, rule, polynomial, affine, derivative_rules, exponential in terms:
        polynomial_proof = (
            PolynomialIntegral(_poly(polynomial[0]), _poly(polynomial[1]))
            if polynomial is not None else None
        )
        affine_proof = (
            AffineIntegrationEvidence(*(Fraction(value) for value in affine))
            if affine is not None else None
        )
        exponential_proof = (
            PolynomialExponentialEvidence(_poly(exponential[0]), _poly(exponential[1]),
                                          Fraction(exponential[2]), Fraction(exponential[3]))
            if exponential is not None else None
        )
        parsed_terms.append(
            IntegrationTerm(kind, term, term_primitive, rule, polynomial_proof,
                            affine_proof, tuple(derivative_rules), exponential_proof)
        )
    return DispatchedIntegral(
        status,
        DomainExpression(original, tuple(source_conditions)),
        DomainExpression(primitive, tuple(result_conditions)),
        variable,
        selected_domain,
        tuple(parsed_terms),
    )


def integrate(
    source: str,
    variable: str,
    domain: Literal["rational", "real", "complex"] = "real",
) -> DispatchedIntegral:
    """Alias for verified polynomial and supported affine analytic sum integration."""
    return integrate_expression_dispatch(source, variable, domain)


@dataclass(frozen=True)
class LimitFactor:
    order: int
    cofactor: Polynomial


@dataclass(frozen=True)
class LimitResult:
    kind: Literal["finite", "positive_infinity", "negative_infinity", "does_not_exist"]
    value: Fraction | None
    at: Fraction
    direction: Literal["left", "right", "two-sided"]
    variable: str
    source: RationalFunction
    numerator_factor: LimitFactor | None
    denominator_factor: LimitFactor
    exclusion_values: tuple[Fraction, ...]
    excluded_at_point: bool


def limit(
    source: str, variable: str, at: str,
    direction: Literal["left", "right", "two-sided"] = "two-sided",
) -> LimitResult:
    """Checked finite-point rational limit; original excluded points stay excluded."""
    kind, value, point, rational, numerator, denominator, exclusions, excluded = (
        _native.limit(source, variable, at, direction)
    )
    return LimitResult(
        kind, Fraction(value) if value is not None else None, Fraction(point),
        direction, variable, _rational(rational),
        LimitFactor(numerator[0], _poly(numerator[1])) if numerator is not None else None,
        LimitFactor(denominator[0], _poly(denominator[1])),
        tuple(Fraction(v) for v in exclusions), excluded,
    )


@dataclass(frozen=True)
class RealBound:
    kind: Literal["negative_infinity", "finite", "positive_infinity"]
    root: ExactQuadraticRoot | None


@dataclass(frozen=True)
class RealInterval:
    lower: RealBound
    upper: RealBound
    lower_closed: bool
    upper_closed: bool


@dataclass(frozen=True)
class InequalityCriticalPoint:
    root: ExactQuadraticRoot
    numerator_multiplicity: int
    denominator_multiplicity: int
    excluded: bool


@dataclass(frozen=True)
class InequalityResult:
    comparison: Literal["<", "<=", ">", ">="]
    variable: str
    residual: RationalFunction
    intervals: tuple[RealInterval, ...]
    critical_points: tuple[InequalityCriticalPoint, ...]


def _real_root(data) -> ExactQuadraticRoot:
    kind, rational, polynomial, branch = data
    return ExactQuadraticRoot(
        kind, Fraction(rational) if rational is not None else None,
        _poly(polynomial) if polynomial is not None else None, branch,
    )


def _real_bound(data) -> RealBound:
    kind, root = data
    return RealBound(kind, _real_root(root) if root is not None else None)


def solve_inequality(source: str, variable: str = "x") -> InequalityResult:
    """Exact real interval union for a bounded rational comparison."""
    operator, residual, intervals, points = _native.solve_inequality(source, variable)
    return InequalityResult(
        operator, variable, _rational(residual),
        tuple(RealInterval(_real_bound(lo), _real_bound(hi), lc, hc)
              for lo, hi, lc, hc in intervals),
        tuple(InequalityCriticalPoint(_real_root(root), nm, dm, excluded)
              for root, nm, dm, excluded in points),
    )


@dataclass(frozen=True)
class AffineSystemResult:
    variables: tuple[str, ...]
    solution: MatrixSolution


def solve_linear_system(
    equations: tuple[str, ...], variables: tuple[str, ...],
) -> AffineSystemResult:
    """Solve rational affine equalities in the explicitly ordered variables."""
    variables = tuple(variables)
    kind, particular, basis = _native.solve_linear_system(list(equations), list(variables))
    return AffineSystemResult(
        tuple(variables),
        MatrixSolution(kind, tuple(map(Fraction, particular)),
                       tuple(tuple(map(Fraction, row)) for row in basis)),
    )


@dataclass(frozen=True)
class MultivariateDivisionResult:
    variables: tuple[str, ...]
    quotients: tuple[tuple[MultivariateTerm, ...], ...]
    remainder: tuple[MultivariateTerm, ...]
    steps: int


def divide_multivariate(
    dividend: str, divisors: tuple[str, ...], variables: tuple[str, ...],
) -> MultivariateDivisionResult:
    """Exact ordered lexicographic division, not a Groebner normal form."""
    variables = tuple(variables)
    canonical_variables, quotients, remainder, steps = _native.divide_multivariate(
        dividend, list(divisors), list(variables),
    )

    def terms(data):
        return tuple(MultivariateTerm(tuple(powers), Fraction(value))
                     for powers, value in data)

    return MultivariateDivisionResult(
        tuple(canonical_variables), tuple(terms(q) for q in quotients), terms(remainder), steps,
    )


@dataclass(frozen=True)
class RootBracketResult:
    kind: Literal["exact", "sign_change", "unresolved"]
    root: Fraction | None
    interval: tuple[Fraction, Fraction] | None
    bisections: int
    precision_met: bool


def refine_real_root_bracket(
    source: str, variable: str, lower: str, upper: str, target_width: str,
    max_bisections: int,
) -> RootBracketResult:
    """Certify a rational root or existence in a sign-change interval.

    An unresolved same-sign interval proves neither existence nor absence.
    A sign change does not certify uniqueness or isolation of every root.
    """
    kind, root, interval, count, precision = _native.refine_real_root_bracket(
        source, variable, lower, upper, target_width, max_bisections,
    )
    return RootBracketResult(
        kind, Fraction(root) if root is not None else None,
        (Fraction(interval[0]), Fraction(interval[1])) if interval is not None else None,
        count, precision,
    )


@dataclass(frozen=True)
class ProblemResult:
    """Versioned checked result. The JSON is data, not a reusable certificate."""
    json: str

    def to_json(self) -> str:
        return self.json

    def as_dict(self) -> dict:
        import json
        return json.loads(self.json)

    @property
    def status(self) -> str:
        return self.as_dict()["outcome"]["status"]

    @property
    def result_domain(self) -> str:
        return self.as_dict()["result_domain"]


def problem(*args: str) -> ProblemResult:
    """Use the same checked request and versioned result as CLI --json."""
    return ProblemResult(_native.problem_json(list(args)))


def solveauto(source: str, variable: str = "x") -> ProblemResult:
    """Automatic checked equation dispatch (complex request, explicit result domain)."""
    return problem("solveauto", source, variable)


def evalexact(source: str) -> ProblemResult:
    """Evaluate the supported exact analytic constants."""
    return problem("evalexact", source)


@dataclass(frozen=True)
class IsolatedRealRoot:
    lower: Fraction
    upper: Fraction
    midpoint: Fraction
    error_bound: Fraction


def isolate_real_roots(source: str, variable: str, target_width: str) -> tuple[IsolatedRealRoot, ...]:
    """Complete disjoint root brackets for nonzero square-free unrestricted Q[x]."""
    return tuple(IsolatedRealRoot(*map(Fraction, row)) for row in
                 _native.isolate_real_roots(source, variable, target_width))


@dataclass(frozen=True)
class GroebnerBasis:
    variables: tuple[str, ...]
    basis: tuple[str, ...]


def groebner_basis(generators: tuple[str, ...], variables: tuple[str, ...]) -> GroebnerBasis:
    """Certified formal ideal basis over Q, not necessarily reduced or monic."""
    names, basis = _native.groebner(list(variables), list(generators))
    return GroebnerBasis(tuple(names), tuple(basis))


@dataclass(frozen=True)
class IdealMembership:
    member: bool
    remainder: str


def ideal_membership(target: str, generators: tuple[str, ...], variables: tuple[str, ...]) -> IdealMembership:
    """Checked formal membership or nonmembership in the generated ideal."""
    return IdealMembership(*_native.ideal_membership(list(variables), list(generators), target))


def special_angle(function: str, pi_coefficient: str) -> ExactQuadraticRoot | None:
    """Exact value at q*pi radians; None denotes a verified tangent pole."""
    kind, value, minimal, branch = _native.special_angle(function, pi_coefficient)
    if kind == "undefined":
        return None
    return ExactQuadraticRoot(kind, Fraction(value) if value is not None else None,
                              _poly(minimal) if minimal is not None else None, branch)


@dataclass(frozen=True)
class AbsoluteInterval:
    lower: Fraction | None
    upper: Fraction | None


def solve_absolute(source: str, variable: str = "x") -> tuple[AbsoluteInterval, ...]:
    """Complete closed real interval union; None denotes an infinite bound."""
    return tuple(AbsoluteInterval(Fraction(a) if a is not None else None,
                                  Fraction(b) if b is not None else None)
                 for a, b in _native.solve_absolute(source, variable))


def solve_polynomial_in_domain(source: str, variable: str = "x", domain: str = "C") -> ProblemResult:
    """Checked roots over Q, R or C, with explicit unresolved cofactor."""
    return problem("rootsfield", source, variable, domain)


def check_calculus(source: str, candidate: str, variable: str = "x", relation: str = "identity") -> tuple[Polynomial, ...]:
    """Verify identity or derivative over the original real rational domain.

    Returns retained polynomial nonzero restrictions. Invalid claims raise.
    """
    return tuple(_poly(poly) for poly in _native.check_calculus(source, candidate, variable, relation))


@dataclass(frozen=True)
class DerivationRecord:
    index: int
    rule: str
    claim: str
    premises: tuple[int, ...]
    conditions: tuple[str, ...]


@dataclass(frozen=True)
class DerivationRecords:
    source_conditions: tuple[str, ...]
    steps: tuple[DerivationRecord, ...]
    conclusion: str


def derivation_records(kind: str, source: str, variable: str = "x") -> DerivationRecords:
    """Checked linear, derivative or distribution records; display text is not a certificate."""
    conditions, steps, conclusion = _native.derivation_records(kind, source, variable)
    return DerivationRecords(tuple(conditions),
                             tuple(DerivationRecord(i, rule, claim, tuple(premises), tuple(required))
                                   for i, rule, claim, premises, required in steps),
                             conclusion)


@dataclass(frozen=True)
class PartialTrigZeros:
    pi_coefficients: tuple[Fraction, ...]
    complete: bool = False


def trig_zero_examples(function: str) -> PartialTrigZeros:
    """Two verified examples only; not a complete periodic solution set."""
    return PartialTrigZeros(tuple(map(Fraction, _native.trig_zero_examples(function))))


__all__ = [
    "PartialTrigZeros", "trig_zero_examples",
    "GroebnerBasis", "groebner_basis", "IdealMembership", "ideal_membership",
    "special_angle", "AbsoluteInterval", "solve_absolute", "solve_polynomial_in_domain",
    "check_calculus", "DerivationRecord", "DerivationRecords", "derivation_records",
    "ProblemResult", "problem", "solveauto", "evalexact", "PolynomialExponentialEvidence",
    "IsolatedRealRoot", "isolate_real_roots",
    "AffineSystemResult", "solve_linear_system", "MultivariateDivisionResult",
    "divide_multivariate", "RootBracketResult", "refine_real_root_bracket",
    "LimitFactor", "LimitResult", "limit", "RealBound", "RealInterval",
    "InequalityCriticalPoint", "InequalityResult", "solve_inequality",
    "MultivariateTerm", "PolynomialExpansion", "expand_polynomial", "expandall",
    "AffineIntegrationEvidence", "IntegrationTerm", "DispatchedIntegral",
    "integrate_expression_dispatch", "integrate",
    "CertifiedPolynomialRoot", "PolynomialRootSet", "polynomial_roots",
    "solve_polynomial_equation", "SturmRootCount", "sturm_root_count",
    "PolynomialIntegral", "DefinitePolynomialIntegral",
    "integrate_polynomial", "integrate_polynomial_between",
    "AnalyticConstant", "analytic_constant",
    "ElementaryIntegral", "integrate_elementary",
    "DomainExpression", "DomainEquation", "DomainProofNode", "DomainRewrite",
    "domain_expression", "domain_equation", "substitute_domain", "rewrite_domain",
    "MathSyntaxError", "ResourceLimitError", "UnsupportedMathematicsError",
    "VerificationFailure", "UndefinedAtPointError", "Step", "Expansion", "LinearResult", "QuadraticResult", "Polynomial",
    "RationalFunction", "RationalQuadraticResult",
    "EuclideanDivision", "ExtendedGcd", "extended_gcd", "modular_inverse",
    "ExactQuadraticRoot", "AlgebraicRootStep", "AlgebraicRootsResult",
    "QuadraticFieldValue", "quadratic_value", "quadratic_operation", "quadratic_inverse",
    "Matrix", "MatrixRref", "MatrixSolution", "matrix_determinant", "matrix_inverse",
    "matrix_rref", "matrix_multiply", "matrix_solve",
    "PolynomialResidue", "modular_reduce", "modular_operation", "modular_power",
    "SquareFreeFactor", "SquareFreeDecomposition",
    "RationalFactor", "RationalFactorization", "factor_polynomial",
    "AlgebraicExtensionValue", "extension_value", "extension_operation",
    "extension_power", "extension_minimal_polynomial",
    "SymbolicDifferentiationStep", "SymbolicDerivative",
    "expression", "expand", "solve", "solve_quadratic", "algebraic_roots", "polynomial", "differentiate", "differentiate_expression", "differentiate_expression_real", "square_free",
    "evaluate", "divide", "gcd", "rational_function", "rational_evaluate",
    "rational_differentiate", "rational_operation", "rational_solve_quadratic",
    "rational_simplify", "rational_derivative", "rational_value", "rational_solve",
]
