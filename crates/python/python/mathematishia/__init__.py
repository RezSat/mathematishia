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


def gcd(left: str, right: str, variable: str) -> Polynomial:
    """Return the monic greatest common divisor in Q[x]."""
    return _poly(_native.gcd(left, right, variable))


__all__ = [
    "DomainExpression", "DomainEquation", "DomainProofNode", "DomainRewrite",
    "domain_expression", "domain_equation", "substitute_domain", "rewrite_domain",
    "MathSyntaxError", "ResourceLimitError", "UnsupportedMathematicsError",
    "VerificationFailure", "UndefinedAtPointError", "Step", "Expansion", "LinearResult", "QuadraticResult", "Polynomial",
    "RationalFunction", "RationalQuadraticResult",
    "ExactQuadraticRoot", "AlgebraicRootStep", "AlgebraicRootsResult",
    "SquareFreeFactor", "SquareFreeDecomposition",
    "SymbolicDifferentiationStep", "SymbolicDerivative",
    "expression", "expand", "solve", "solve_quadratic", "algebraic_roots", "polynomial", "differentiate", "differentiate_expression", "square_free",
    "evaluate", "divide", "gcd", "rational_function", "rational_evaluate",
    "rational_differentiate", "rational_operation", "rational_solve_quadratic",
]
