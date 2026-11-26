import unittest
from fractions import Fraction

import mathematishia as m


class AdvancedOperationTests(unittest.TestCase):
    def test_algebra(self):
        generators = ("x*y-1", "y^2-1")
        variables = ("x", "y")
        basis = m.groebner_basis(generators, variables)
        self.assertEqual(basis.variables, variables)
        self.assertTrue(basis.basis)
        self.assertEqual(m.ideal_membership("x-y", generators, variables).remainder, "0")
        self.assertTrue(m.ideal_membership("x-y", generators, variables).member)
        self.assertFalse(m.ideal_membership("x*y", ("x^2", "y^2"), variables).member)
        with self.assertRaises(m.UndefinedAtPointError):
            m.groebner_basis(("x+0/y",), variables)
        self.assertEqual(m.solve_polynomial_in_domain("x^2-2", domain="Q").status, "no_solutions")
        result = m.solve_polynomial_in_domain("x^2-2", domain="R")
        self.assertEqual(result.status, "complete")
        self.assertEqual(result.result_domain, "R")
        self.assertEqual(m.solve_polynomial_in_domain("x^3-2").status, "partial")

    def test_angles_and_absolute_intervals(self):
        self.assertEqual(m.special_angle("sin", "1/6").rational, Fraction(1, 2))
        self.assertEqual(m.special_angle("cos", "1/4").branch, "upper_real")
        self.assertIsNone(m.special_angle("tan", "1/2"))
        self.assertFalse(m.trig_zero_examples("sin").complete)
        self.assertEqual(m.trig_zero_examples("cos").pi_coefficients,
                         (Fraction(1, 2), Fraction(3, 2)))
        self.assertEqual(m.solve_absolute("abs(abs(x)-2)=1"),
                         tuple(m.AbsoluteInterval(Fraction(x), Fraction(x)) for x in (-3, -1, 1, 3)))
        self.assertEqual(m.solve_absolute("abs(x)=x"), (m.AbsoluteInterval(Fraction(0), None),))
        self.assertEqual(m.solve_absolute("abs(x)=-1"), ())
        with self.assertRaises(m.UndefinedAtPointError):
            m.solve_absolute("abs(x+0/x)=3")

    def test_calculus_and_records(self):
        self.assertTrue(m.check_calculus("(x^2-1)/(x-1)", "x+1"))
        self.assertEqual(m.check_calculus("x^2", "2*x", relation="derivative"), ())
        with self.assertRaises(m.VerificationFailure):
            m.check_calculus("x^2", "3*x", relation="derivative")
        with self.assertRaises(m.UndefinedAtPointError):
            m.check_calculus("x+1", "(x^2-1)/(x-1)")
        linear = m.derivation_records("linear", "2*x+4=10")
        self.assertEqual(linear.conclusion, "x = 3")
        for record in linear.steps:
            self.assertTrue(all(p < record.index for p in record.premises))
        self.assertTrue(m.derivation_records("derivative", "sin(x^2)").steps)
        self.assertTrue(m.derivation_records("expand", "2*(x+1)").steps)


if __name__ == "__main__":
    unittest.main()
