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
class Polynomial:
    expression: str
    coefficients: tuple[Fraction, ...]

    @property
    def degree(self) -> int | None:
        """Degree is undefined for the zero polynomial."""
        return len(self.coefficients) - 1 if self.coefficients else None


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


def _rational(data: tuple[tuple[str, list[str]], tuple[str, list[str]],
                          list[tuple[str, list[str]]]]) -> RationalFunction:
    n, d, excluded = data
    return RationalFunction(_poly(n), _poly(d), tuple(_poly(p) for p in excluded))


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
    "MathSyntaxError", "ResourceLimitError", "UnsupportedMathematicsError",
    "VerificationFailure", "UndefinedAtPointError", "Step", "Expansion", "LinearResult", "Polynomial",
    "RationalFunction",
    "SquareFreeFactor", "SquareFreeDecomposition",
    "SymbolicDifferentiationStep", "SymbolicDerivative",
    "expression", "expand", "solve", "polynomial", "differentiate", "differentiate_expression", "square_free",
    "evaluate", "divide", "gcd", "rational_function", "rational_evaluate",
    "rational_differentiate", "rational_operation",
]
