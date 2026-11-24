import unittest
from dataclasses import FrozenInstanceError
from fractions import Fraction

import mathematishia as m


class RealAnalysisTests(unittest.TestCase):
    def test_rational_limits_and_holes(self):
        result = m.limit("(x^2-1)/(x-1)", "x", "1")
        self.assertEqual(result.kind, "finite")
        self.assertEqual(result.value, Fraction(2))
        self.assertTrue(result.excluded_at_point)
        self.assertEqual(result.source.excluded_zero_sets[0].coefficients,
                         (Fraction(-1), Fraction(1)))
        self.assertEqual(result.exclusion_values, (Fraction(0),))
        with self.assertRaises(FrozenInstanceError):
            result.value = Fraction(3)
        self.assertEqual(m.limit("(2*x+1)/(x+2)", "x", "0.5").value, Fraction(4, 5))

    def test_one_sided_and_two_sided_poles(self):
        for direction, kind in [
            ("left", "negative_infinity"),
            ("right", "positive_infinity"),
            ("two-sided", "does_not_exist"),
        ]:
            result = m.limit("1/x", "x", "0", direction)
            self.assertEqual(result.kind, kind)
            self.assertIsNone(result.value)
            self.assertEqual(result.denominator_factor.order, 1)
            self.assertTrue(result.excluded_at_point)
        self.assertEqual(m.limit("1/x^2", "x", "0").kind, "positive_infinity")

    def test_inequality_interval_boundaries_and_quadratic_roots(self):
        result = m.solve_inequality("x^2-4>0")
        self.assertEqual(len(result.intervals), 2)
        left, right = result.intervals
        self.assertEqual(left.lower.kind, "negative_infinity")
        self.assertEqual(left.upper.root.rational, Fraction(-2))
        self.assertEqual(right.lower.root.rational, Fraction(2))
        self.assertEqual(right.upper.kind, "positive_infinity")
        self.assertFalse(left.upper_closed)
        self.assertFalse(right.lower_closed)
        bounded = m.solve_inequality("x^2≤4").intervals[0]
        self.assertTrue(bounded.lower_closed and bounded.upper_closed)
        irrational = m.solve_inequality("x^2<2").intervals[0]
        self.assertEqual(irrational.lower.root.branch, "lower_real")
        self.assertEqual(irrational.upper.root.branch, "upper_real")
        self.assertEqual(irrational.lower.root.minimal_polynomial.coefficients,
                         (Fraction(-2), Fraction(0), Fraction(1)))
        self.assertEqual(m.solve_inequality("x^2+1<0").intervals, ())

    def test_both_operand_exclusions_survive_cancellation(self):
        result = m.solve_inequality("0/(x-1)<=0/(x+2)")
        self.assertEqual(len(result.intervals), 3)
        self.assertEqual({p.coefficients for p in result.residual.excluded_zero_sets},
                         {(Fraction(-1), Fraction(1)), (Fraction(2), Fraction(1))})
        self.assertTrue(all(point.excluded for point in result.critical_points))
        self.assertTrue(all(not interval.lower_closed and not interval.upper_closed
                            for interval in result.intervals))
        rational = m.solve_inequality("(x-1)/(x+2)<=0").intervals[0]
        self.assertEqual(rational.lower.root.rational, Fraction(-2))
        self.assertEqual(rational.upper.root.rational, Fraction(1))
        self.assertFalse(rational.lower_closed)
        self.assertTrue(rational.upper_closed)

    def test_refusals_and_existing_apis(self):
        for source in ["x^3>0", "sin(x)>0", "x+y>0"]:
            with self.assertRaises(m.UnsupportedMathematicsError):
                m.solve_inequality(source)
        with self.assertRaises(m.MathSyntaxError):
            m.solve_inequality("0<x<1")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.limit("sin(x)/x", "x", "0")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.limit("1/x", "x", "infinity")
        with self.assertRaises(m.MathSyntaxError):
            m.limit("x^2", "x", "0", "above")
        with self.assertRaises(m.UndefinedAtPointError):
            m.limit("1/0", "x", "0")
        self.assertTrue(m.expandall("(x+1)^2", ["x"]).complete)


if __name__ == "__main__":
    unittest.main()
