import unittest
from dataclasses import FrozenInstanceError
from fractions import Fraction

import mathematishia as m


class CertifiedDeliveryAPITests(unittest.TestCase):
    def test_polynomial_equation_over_complex_roots(self):
        result = m.solve_polynomial_equation("x^2-5*x+6=0", "x")
        self.assertEqual(result.kind, "finite")
        self.assertTrue(result.complete)
        self.assertIsNone(result.unresolved_cofactor)
        self.assertEqual(
            [(root.rational, root.multiplicity) for root in result.roots],
            [(Fraction(2), 1), (Fraction(3), 1)],
        )
        self.assertEqual(result.polynomial.coefficients,
                         (Fraction(6), Fraction(-5), Fraction(1)))
        self.assertEqual(result, m.solve_polynomial_equation("x^2-5*x+6=0", "x"))
        with self.assertRaises(FrozenInstanceError):
            result.complete = False

    def test_exact_quadratic_algebraic_roots_with_multiplicity(self):
        irrational = m.polynomial_roots("x^2-2", "x")
        self.assertTrue(irrational.complete)
        self.assertEqual([r.kind for r in irrational.roots],
                         ["real_quadratic", "real_quadratic"])
        self.assertEqual([r.branch for r in irrational.roots],
                         ["lower_real", "upper_real"])
        for root in irrational.roots:
            self.assertEqual(root.multiplicity, 1)
            self.assertIsNone(root.rational)
            self.assertEqual(root.minimal_polynomial.coefficients,
                             (Fraction(-2), Fraction(0), Fraction(1)))

        double = m.polynomial_roots("(x-2)^2", "x")
        self.assertEqual([(r.rational, r.multiplicity) for r in double.roots],
                         [(Fraction(2), 2)])

    def test_partial_unresolved_zero_and_nonzero_polynomials(self):
        partial = m.polynomial_roots("x^4+1", "x")
        self.assertEqual(partial.kind, "finite")
        self.assertFalse(partial.complete)
        self.assertIsNotNone(partial.unresolved_cofactor)
        self.assertEqual(partial.roots, ())
        self.assertEqual(m.polynomial_roots("0", "x").kind, "all_complex")
        self.assertTrue(m.polynomial_roots("0", "x").complete)
        self.assertEqual(m.polynomial_roots("7", "x").roots, ())
        self.assertTrue(m.polynomial_roots("7", "x").complete)

    def test_rejects_erased_domains_nonpolynomials_and_parser_failures(self):
        for equation in ["0*x^-1=0", "(x^2-1)/(x-1)=0"]:
            with self.assertRaises(m.UnsupportedMathematicsError):
                m.solve_polynomial_equation(equation, "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.polynomial_roots("x^-1", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.polynomial_roots("sin(x)", "x")
        with self.assertRaises(m.MathSyntaxError):
            m.solve_polynomial_equation("x^2-1", "x")
        with self.assertRaises(m.MathSyntaxError):
            m.polynomial_roots("x^2-", "x")
        with self.assertRaises(m.MathSyntaxError):
            m.polynomial_roots("x^2-1", "2*x")
        with self.assertRaises(m.ResourceLimitError):
            m.polynomial_roots("9" * 1500, "x")
        with self.assertRaises(m.ResourceLimitError):
            m.polynomial_roots("x" * 32769, "x")

    def test_sturm_count_exact_certificate_and_determinism(self):
        result = m.sturm_root_count("x^2-2", "x", "-2", "2")
        self.assertEqual(result.count, 2)
        self.assertEqual(result.lower, Fraction(-2))
        self.assertEqual(result.upper, Fraction(2))
        self.assertEqual(result.lower_variations - result.upper_variations, 2)
        self.assertEqual(len(result.sequence), len(result.lower_signs))
        self.assertEqual(len(result.sequence), len(result.upper_signs))
        self.assertEqual(result, m.sturm_root_count("x^2-2", "x", "-2", "2"))
        self.assertEqual(m.sturm_root_count("x^2+1", "x", "-2", "2").count, 0)
        self.assertEqual(m.sturm_root_count("x^2-2", "x", "0", "2").count, 1)

    def test_sturm_refuses_unsupported_and_invalid_domain(self):
        for source in ["0", "x^2"]:
            with self.assertRaises(m.UnsupportedMathematicsError):
                m.sturm_root_count(source, "x", "-2", "2")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.sturm_root_count("x^2-2", "x", "2", "-2")
        with self.assertRaises(m.UndefinedAtPointError):
            m.sturm_root_count("x^2-1", "x", "1", "2")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.sturm_root_count("x^-1", "x", "-2", "2")
        self.assertEqual(m.sturm_root_count("x^2-2", "x", "0.0", "2").count, 1)
        with self.assertRaises(m.MathSyntaxError):
            m.sturm_root_count("x^2-2", "x", "0..0", "2")

    def test_checked_indefinite_and_directed_polynomial_integrals(self):
        primitive = m.integrate_polynomial("x^2", "x")
        self.assertEqual(primitive.integrand.coefficients,
                         (Fraction(0), Fraction(0), Fraction(1)))
        self.assertEqual(primitive.primitive.coefficients,
                         (Fraction(0), Fraction(0), Fraction(0), Fraction(1, 3)))
        self.assertEqual(m.differentiate(primitive.primitive.expression, "x"),
                         primitive.integrand)
        positive = m.integrate_polynomial_between("x^2", "x", "0", "3")
        negative = m.integrate_polynomial_between("x^2", "x", "3", "0")
        self.assertEqual(positive.value, Fraction(9))
        self.assertEqual(negative.value, Fraction(-9))
        self.assertEqual(positive.lower, Fraction(0))
        self.assertEqual(positive.upper, Fraction(3))
        self.assertEqual(m.integrate_polynomial("7/2", "x").primitive.coefficients,
                         (Fraction(0), Fraction(7, 2)))

    def test_integral_rejects_restricted_or_unsupported_domain(self):
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate_polynomial("x^-1", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate_polynomial("sin(x)", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate_polynomial("y", "x")
        with self.assertRaises(m.MathSyntaxError):
            m.integrate_polynomial_between("x^2", "x", "not a number", "1")

    def test_checked_one_step_analytic_constants(self):
        for source, value, rule in [
            ("sin(0)", "0", "analytic.sin_zero"),
            ("cos(0)", "1", "analytic.cos_zero"),
            ("tan(0)", "0", "analytic.tan_zero"),
            ("exp(0)", "1", "analytic.exp_zero"),
            ("log(1)", "0", "analytic.log_one"),
        ]:
            result = m.analytic_constant(source)
            self.assertEqual(result.status, "exact")
            self.assertEqual(result.expression, value)
            self.assertEqual(result.rule, rule)
            self.assertEqual(result, m.analytic_constant(source))
        untouched = m.analytic_constant("sin(x)")
        self.assertEqual(untouched.status, "unchanged")
        self.assertIsNone(untouched.rule)
        self.assertEqual(untouched.expression, "sin(x)")
        self.assertEqual(m.analytic_constant("sin(0)", "complex").expression, "0")
        with self.assertRaises(FrozenInstanceError):
            untouched.expression = "0"

    def test_analytic_rejects_invalid_domains_and_undefined_points(self):
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.analytic_constant("sin(0)", "rational")
        with self.assertRaises(m.UndefinedAtPointError):
            m.analytic_constant("log(0)")
        with self.assertRaises(m.MathSyntaxError):
            m.analytic_constant("foo(0)")

    def test_elementary_affine_integration_is_verified(self):
        result = m.integrate_elementary("cos(x)", "x")
        self.assertEqual(result.rule, "calculus.integrate_affine_cosine")
        self.assertEqual(result.input_scale, Fraction(1))
        self.assertEqual(result.slope, Fraction(1))
        self.assertEqual(result.intercept, Fraction(0))
        self.assertEqual(result.primitive, "sin(x)")
        self.assertEqual(result.conditions, ())
        self.assertEqual(m.integrate_elementary("cos(x)", "x"), result)
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate_elementary("sin(x^2)", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate_elementary("log(x)", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.integrate_elementary("cos(x)", "x", "rational")


if __name__ == "__main__":
    unittest.main()
