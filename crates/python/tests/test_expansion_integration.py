import unittest
from dataclasses import FrozenInstanceError, replace
from fractions import Fraction

import mathematishia as m


class WaveTwoPythonParityTests(unittest.TestCase):
    def coefficients(self, expansion):
        return {term.exponents: term.coefficient for term in expansion.terms}

    def test_square_expands_completely_with_exact_coefficients(self):
        result = m.expandall("(x+1)^2", ["x"])
        self.assertEqual(result.status, "exact")
        self.assertTrue(result.complete)
        self.assertFalse(result.syntactically_unchanged)
        self.assertEqual(result.variables, ("x",))
        self.assertEqual(self.coefficients(result), {
            (0,): Fraction(1),
            (1,): Fraction(2),
            (2,): Fraction(1),
        })
        self.assertEqual(result.source.conditions, ())
        self.assertEqual(result.expanded.conditions, ())
        self.assertEqual(result, m.expand_polynomial("(x+1)^2", ("x",)))
        self.assertEqual(result, m.expandall("(x+1)^2", ["x"]))
        with self.assertRaises(FrozenInstanceError):
            result.complete = False

    def test_multivariate_cube_uses_sorted_ring_and_exponents(self):
        result = m.expandall("(x-y)^3", ["y", "x"])
        self.assertEqual(result.variables, ("x", "y"))
        self.assertEqual(self.coefficients(result), {
            (3, 0): Fraction(1),
            (2, 1): Fraction(-3),
            (1, 2): Fraction(3),
            (0, 3): Fraction(-1),
        })
        self.assertEqual(sum((term.coefficient for term in result.terms), Fraction()), Fraction())
        self.assertTrue(result.complete)

    def test_zero_ring_already_expanded_and_rational_coefficients(self):
        self.assertEqual(
            self.coefficients(m.expandall("3/4", [])),
            {(): Fraction(3, 4)},
        )
        zero = m.expandall("0", [])
        self.assertEqual(zero.terms, ())
        self.assertTrue(zero.complete)
        plain = m.expandall("x+1", ["x"])
        self.assertTrue(plain.complete)
        self.assertEqual(self.coefficients(plain), {
            (0,): Fraction(1),
            (1,): Fraction(1),
        })
        decimal = m.expandall("(x+0.5)^2", ["x"])
        self.assertEqual(self.coefficients(decimal), {
            (0,): Fraction(1, 4),
            (1,): Fraction(1),
            (2,): Fraction(1),
        })

    def test_expansion_preserves_original_undefined_domain_even_when_zero(self):
        result = m.expandall("0*x^-1", ["x"])
        self.assertEqual(result.status, "conditional")
        self.assertEqual(result.source.expression, "0")
        self.assertEqual(result.expanded.expression, "0")
        self.assertEqual(result.source.conditions, ("x != 0",))
        self.assertEqual(result.expanded.conditions, result.source.conditions)
        self.assertTrue(result.complete)
        self.assertEqual(result.terms, ())

    def test_expansion_rejects_inadmissible_rings_and_unsupported_forms(self):
        for expression, variables in [
            ("x+y", ["x"]),
            ("sin(x)", ["x"]),
            ("x^-1", ["x"]),
            ("x^(1/2)", ["x"]),
            ("x+1", ["x", "x"]),
            ("x^y", ["x", "y"]),
        ]:
            with self.subTest(expression=expression, variables=variables):
                with self.assertRaises(m.UnsupportedMathematicsError):
                    m.expandall(expression, variables)
        with self.assertRaises(TypeError):
            m.expandall("x", "x")
        with self.assertRaises(m.MathSyntaxError):
            m.expandall("x+*1", ["x"])
        with self.assertRaises(m.MathSyntaxError):
            m.expandall("x+y", ["x*y"])

    def test_expansion_resource_refusal_is_distinct_from_unsupported(self):
        with self.assertRaises(m.ResourceLimitError):
            m.expandall("x^129", ["x"])
        with self.assertRaises(m.ResourceLimitError):
            m.expandall("1", [f"x{i}" for i in range(17)])
        with self.assertRaises(m.ResourceLimitError):
            m.expandall("x" * 32769, ["x"])

    def test_python_display_objects_are_not_kernel_proof_certificates(self):
        result = m.expandall("(x+1)^2", ["x"])
        forged = replace(result, expanded=m.DomainExpression("0", ()))
        self.assertNotEqual(forged, result)
        self.assertEqual(m.expandall("(x+1)^2", ["x"]), result)
        with self.assertRaises(TypeError):
            m.expandall("(x+1)^2", ["x"], forged)

    def test_combined_polynomial_and_affine_analytic_integral(self):
        result = m.integrate("x^2+sin(2*x)", "x")
        self.assertEqual(result.status, "exact")
        self.assertEqual(result.domain, "real")
        self.assertEqual(result.variable, "x")
        self.assertEqual(result.source.conditions, ())
        self.assertEqual(result.primitive.conditions, ())
        self.assertEqual({term.kind for term in result.terms},
                         {"polynomial", "elementary"})
        polynomial = next(term for term in result.terms if term.kind == "polynomial")
        analytic = next(term for term in result.terms if term.kind == "elementary")
        self.assertIsNone(polynomial.rule)
        self.assertIsNotNone(polynomial.polynomial)
        self.assertEqual(
            polynomial.polynomial.primitive.coefficients,
            (Fraction(0), Fraction(0), Fraction(0), Fraction(1, 3)),
        )
        self.assertIsNone(polynomial.affine)
        self.assertEqual(analytic.rule, "calculus.integrate_affine_sine")
        self.assertEqual(analytic.affine.input_scale, Fraction(1))
        self.assertEqual(analytic.affine.slope, Fraction(2))
        self.assertEqual(analytic.affine.intercept, Fraction(0))
        self.assertIn("cos(", analytic.primitive)
        self.assertTrue(analytic.derivative_rules)
        self.assertEqual(result, m.integrate_expression_dispatch("x^2+sin(2*x)", "x"))
        with self.assertRaises(FrozenInstanceError):
            analytic.kind = "polynomial"

    def test_dispatch_exact_polynomials_and_multiple_analytic_terms(self):
        polynomial = m.integrate("x^2+1", "x", "rational")
        self.assertEqual(polynomial.status, "exact")
        self.assertEqual(len(polynomial.terms), 2)
        self.assertTrue(all(term.kind == "polynomial" for term in polynomial.terms))
        self.assertEqual(polynomial.primitive.conditions, ())
        analytic = m.integrate("sin(x)+cos(x)+exp(x)", "x", "complex")
        self.assertEqual(len(analytic.terms), 3)
        self.assertTrue(all(term.kind == "elementary" for term in analytic.terms))
        self.assertEqual(analytic.domain, "complex")

    def test_dispatch_preserves_history_restrictions(self):
        result = m.integrate("0*x^-1+sin(x)", "x")
        self.assertEqual(result.status, "conditional")
        self.assertEqual(result.source.conditions, ("x != 0",))
        self.assertEqual(result.primitive.conditions, ("x != 0",))
        self.assertEqual(len(result.terms), 1)

    def test_dispatch_rejects_analytic_products_and_non_affine_terms(self):
        for source in [
            "x*sin(x)",
            "sin(x)*cos(x)",
            "sin(x^2)",
            "log(x)",
            "tan(x)",
            "2*(sin(x)+cos(x))",
            "x^-1+x^2",
            "y+x",
        ]:
            with self.subTest(source=source):
                with self.assertRaises(m.UnsupportedMathematicsError):
                    m.integrate(source, "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate("sin(x)", "x", "rational")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate("cos(x)", "x", "rational")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate("sin(x)", "x", "integer")
        with self.assertRaises(m.MathSyntaxError):
            m.integrate("x^2+*", "x")
        with self.assertRaises(m.ResourceLimitError):
            m.integrate("x" * 32769, "x")

    def test_existing_interfaces_still_return_distinct_typed_results(self):
        self.assertTrue(m.expand("2*(x+1)", 8).complete)
        self.assertEqual(m.integrate_polynomial("x^2", "x").primitive.coefficients,
                         (Fraction(0), Fraction(0), Fraction(0), Fraction(1, 3)))
        self.assertEqual(m.integrate_elementary("cos(x)", "x").rule,
                         "calculus.integrate_affine_cosine")


if __name__ == "__main__":
    unittest.main()
