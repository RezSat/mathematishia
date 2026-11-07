import unittest
from dataclasses import FrozenInstanceError
from fractions import Fraction

import mathematishia as m


class ExactMathTests(unittest.TestCase):
    def test_module_imports_installed_extension(self):
        self.assertTrue(hasattr(m, "solve"))
        self.assertEqual(m.expression("1/2 + 1/3"), "5/6")
        self.assertEqual(m.expression("340282366920938463463374607431768211456"),
                         "340282366920938463463374607431768211456")

    def test_linear_solver_and_verified_conditions(self):
        result = m.solve("2*x+4=10", "x")
        self.assertEqual(result.kind, "unique")
        self.assertEqual(result.value, Fraction(3))
        self.assertGreaterEqual(len(result.steps), 1)
        self.assertTrue(result.steps[0].before.endswith(" = 10"))
        self.assertIn("x", result.steps[0].before)
        self.assertEqual(result.steps[-1].rule, "equation.divide_by_nonzero_coefficient")
        self.assertEqual(result.steps[-1].verified_condition, "2 != 0")
        with self.assertRaises(FrozenInstanceError):
            result.value = Fraction(4)

    def test_no_and_all_rational_solutions(self):
        self.assertEqual(m.solve("x+1=x+2", "x").kind, "none")
        self.assertEqual(m.solve("x=x", "x").kind, "all")

    def test_partial_and_complete_expansion(self):
        partial = m.expand("(x+1)*(y+2)", 0)
        self.assertFalse(partial.complete)
        self.assertEqual(len(partial.steps), 0)
        result = m.expand("2*(x+3)", 4)
        self.assertTrue(result.complete)
        self.assertEqual(len(result.steps), 1)
        self.assertEqual(result.steps[0].rule, "algebra.distribute_product_over_sum")

    def test_exact_polynomial_arithmetic(self):
        p = m.polynomial("(x+1)^2", "x")
        self.assertEqual(p.coefficients, (Fraction(1), Fraction(2), Fraction(1)))
        self.assertEqual(p.degree, 2)
        self.assertEqual(m.polynomial("0", "x").degree, None)
        self.assertEqual(m.evaluate("x^2+1/2", "x", "1/2"), Fraction(3, 4))
        q, r = m.divide("x^2-1", "x-1", "x")
        self.assertEqual(q.coefficients, (Fraction(1), Fraction(1)))
        self.assertEqual(r.coefficients, ())
        self.assertEqual(m.gcd("x^2-1", "x-1", "x").coefficients,
                         (Fraction(-1), Fraction(1)))

    def test_formal_differentiation_is_exact(self):
        derivative = m.differentiate("x^3+1/2*x", "x")
        self.assertEqual(
            derivative.coefficients,
            (Fraction(1, 2), Fraction(0), Fraction(3)),
        )
        self.assertEqual(derivative.degree, 2)
        self.assertEqual(m.differentiate("5", "x").coefficients, ())
        self.assertEqual(m.differentiate("0", "x").coefficients, ())
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.differentiate("x^-1", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.differentiate("x^33", "x")

    def test_square_free_decomposition_and_immutable_typed_results(self):
        result = m.square_free("2*(x-1)^2*(x+2)^3", "x")
        self.assertEqual(result.unit, Fraction(2))
        self.assertEqual(
            [(factor.polynomial.coefficients, factor.multiplicity)
             for factor in result.factors],
            [
                ((Fraction(-1), Fraction(1)), 2),
                ((Fraction(2), Fraction(1)), 3),
            ],
        )
        with self.assertRaises(FrozenInstanceError):
            result.unit = Fraction(3)
        with self.assertRaises(FrozenInstanceError):
            result.factors[0].multiplicity = 4

    def test_square_free_constants_and_unsupported_polynomials(self):
        constant = m.square_free("-3/4", "x")
        self.assertEqual(constant.unit, Fraction(-3, 4))
        self.assertEqual(constant.factors, ())
        sample = m.square_free("x^2+1", "x")
        self.assertEqual(len(sample.factors), 1)
        self.assertEqual(sample.factors[0].multiplicity, 1)
        self.assertEqual(
            sample.factors[0].polynomial.coefficients,
            (Fraction(1), Fraction(0), Fraction(1)),
        )
        for expr in ["0", "x^-1", "x^33", "y+1"]:
            with self.assertRaises(m.UnsupportedMathematicsError):
                m.square_free(expr, "x")

    def test_square_free_matches_published_reference_coefficients(self):
        result = m.square_free(
            "2*x^5+16*x^4+50*x^3+76*x^2+56*x+16",
            "x",
        )
        self.assertEqual(result.unit, Fraction(2))
        self.assertEqual(
            [(f.polynomial.coefficients, f.multiplicity) for f in result.factors],
            [
                ((Fraction(1), Fraction(1)), 2),
                ((Fraction(2), Fraction(1)), 3),
            ],
        )

    def test_exact_symbolic_derivative_with_checked_rules(self):
        result = m.differentiate_expression("x*y+x^3", "x")
        self.assertEqual(result.original, "x*y+x^3")
        self.assertEqual(result.variable, "x")
        self.assertIn("y", result.derivative)
        self.assertIn("3", result.derivative)
        self.assertTrue(result.steps)
        self.assertTrue(all(s.relation == "derivative_of" for s in result.steps))
        self.assertTrue(all(s.rule.startswith("calculus.") for s in result.steps))
        self.assertEqual(m.differentiate_expression("x*y", "y").derivative, "x")
        self.assertEqual(m.differentiate_expression("x^2", "x").derivative, "2 * x")
        with self.assertRaises(FrozenInstanceError):
            result.derivative = "42"

    def test_symbolic_derivative_domains_are_explicit(self):
        for source in ["x^-1", "x^0", "x^(1/2)", "x^y"]:
            with self.assertRaises(m.UnsupportedMathematicsError):
                m.differentiate_expression(source, "x")
        with self.assertRaises(m.ResourceLimitError):
            m.differentiate_expression("x^33", "x")
        with self.assertRaises(m.ResourceLimitError):
            m.differentiate_expression("x^999999999999999999999999999999999999999", "x")

    def test_typed_errors(self):
        with self.assertRaises(m.MathSyntaxError):
            m.expression("x / 2")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.solve("x^2=4", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.divide("x", "0", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.polynomial("y+1", "x")
        with self.assertRaises(m.ResourceLimitError):
            m.expand("x*(y+1)", 13)

    def test_untrusted_input_cannot_bypass_type_validation(self):
        with self.assertRaises(m.MathSyntaxError):
            m.solve("x+2", "x")
        with self.assertRaises(m.MathSyntaxError):
            m.solve("x=1", "2")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.evaluate("x+1", "x", "y")
        with self.assertRaises(m.MathSyntaxError):
            m.expression("5/0")


if __name__ == "__main__":
    unittest.main()
