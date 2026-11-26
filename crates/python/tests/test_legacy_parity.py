import unittest
from dataclasses import FrozenInstanceError
from fractions import Fraction

import mathematishia as m


class LegacyParityTests(unittest.TestCase):
    def test_affine_unique_order_and_exactness(self):
        result = m.solve_linear_system(("x+y=1", "x-y=0"), ("y", "x"))
        self.assertEqual(result.variables, ("y", "x"))
        self.assertEqual(result.solution.kind, "unique")
        self.assertEqual(result.solution.particular, (Fraction(1, 2),) * 2)
        with self.assertRaises(FrozenInstanceError):
            result.variables = ()

    def test_affine_family_and_inconsistency(self):
        result = m.solve_linear_system(("x+y=1",), ("x", "y")).solution
        self.assertEqual(result.kind, "infinite")
        self.assertEqual(result.particular, (Fraction(1), Fraction(0)))
        self.assertEqual(result.nullspace_basis, ((Fraction(-1), Fraction(1)),))
        self.assertEqual(m.solve_linear_system(("x=0", "x=1"), ("x",)).solution.kind,
                         "inconsistent")

    def test_affine_refusals(self):
        for equation in ("x*y=1", "z=1", "x/x=1", "0*(1/x)=0"):
            with self.subTest(equation=equation), self.assertRaises(m.UnsupportedMathematicsError):
                m.solve_linear_system((equation,), ("x", "y"))
        with self.assertRaises(m.ResourceLimitError):
            m.solve_linear_system(("x=1",) * 17, ("x",))
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.solve_linear_system(("x=1",), ("x", "x"))
        with self.assertRaises(m.MathSyntaxError):
            m.solve_linear_system(("x",), ("x",))

    def test_ordered_division(self):
        a = m.divide_multivariate("x*y", ("x", "y"), ("x", "y"))
        b = m.divide_multivariate("x*y", ("y", "x"), ("x", "y"))
        self.assertEqual(a.quotients, ((m.MultivariateTerm((0, 1), Fraction(1)),), ()))
        self.assertEqual(b.quotients, ((m.MultivariateTerm((1, 0), Fraction(1)),), ()))
        self.assertEqual(a, m.divide_multivariate("x*y", ("x", "y"), ("y", "x")))
        self.assertEqual(a.variables, ("x", "y"))
        self.assertEqual(a.remainder, ())
        self.assertEqual(b.remainder, ())
        c = m.divide_multivariate("x+y", ("2*x",), ("x", "y"))
        self.assertEqual(c.quotients, ((m.MultivariateTerm((0, 0), Fraction(1, 2)),),))
        self.assertEqual(c.remainder, (m.MultivariateTerm((0, 1), Fraction(1)),))

    def test_division_refusals(self):
        for source in ("x/x", "0*(1/x)", "z", "sin(x)"):
            with self.subTest(source=source), self.assertRaises(m.UnsupportedMathematicsError):
                m.divide_multivariate(source, ("x",), ("x", "y"))
        with self.assertRaises(m.UndefinedAtPointError):
            m.divide_multivariate("x", ("x-x",), ("x",))
        with self.assertRaises(m.ResourceLimitError):
            m.divide_multivariate("x", ("x",) * 33, ("x",))
        with self.assertRaises(m.ResourceLimitError):
            m.divide_multivariate("x^129", ("x",), ("x",))

    def test_root_exact_and_partial(self):
        endpoint = m.refine_real_root_bracket("x-1", "x", "1", "2", "1/8", 0)
        self.assertEqual((endpoint.kind, endpoint.root, endpoint.precision_met),
                         ("exact", Fraction(1), True))
        bracket = m.refine_real_root_bracket("x^2-2", "x", "1", "2", "1/8", 3)
        self.assertEqual(bracket.kind, "sign_change")
        self.assertTrue(bracket.precision_met)
        self.assertEqual(bracket.interval, (Fraction(11, 8), Fraction(3, 2)))
        partial = m.refine_real_root_bracket("x^2-2", "x", "1", "2", "1/8", 0)
        self.assertEqual((partial.kind, partial.precision_met, partial.bisections),
                         ("sign_change", False, 0))
        even = m.refine_real_root_bracket("(x-1)^2", "x", "0", "2", "1/8", 10)
        self.assertEqual((even.kind, even.precision_met), ("unresolved", False))

    def test_root_refusals(self):
        for source in ("0", "x/x", "0*(1/x)"):
            with self.subTest(source=source), self.assertRaises(m.UnsupportedMathematicsError):
                m.refine_real_root_bracket(source, "x", "0", "2", "1/8", 10)
        for lower, upper, width in (("2", "1", "1/8"), ("0", "2", "0"),
                                     ("0*(1/x)", "2", "1/8")):
            with self.assertRaises(m.UnsupportedMathematicsError):
                m.refine_real_root_bracket("x", "x", lower, upper, width, 10)
        with self.assertRaises(TypeError):
            m.refine_real_root_bracket("x", "x", 0.0, "2", "1/8", 10)


if __name__ == "__main__":
    unittest.main()