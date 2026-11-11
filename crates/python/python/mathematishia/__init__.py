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


__all__ = [
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
    "expression", "expand", "solve", "solve_quadratic", "algebraic_roots", "polynomial", "differentiate", "differentiate_expression", "square_free",
    "evaluate", "divide", "gcd", "rational_function", "rational_evaluate",
    "rational_differentiate", "rational_operation", "rational_solve_quadratic",
    "rational_simplify", "rational_derivative", "rational_value", "rational_solve",
]
