import unittest
from fractions import Fraction

import mathematishia as m


def rational(value):
    return Fraction(int(value["numerator"]), int(value["denominator"]))


class SchoolStrategiesTests(unittest.TestCase):
    def check_school(self, method, args, status="complete"):
        result = m.school(*args)
        self.assertEqual(result["kind"], "checked_school_result")
        self.assertEqual(result["schema_version"], 1)
        self.assertEqual(result["status"], status)
        self.assertEqual(result["method"], method)
        self.assertEqual(result, m.school(*args))
        return result

    def test_pattern_and_methods(self):
        result = self.check_school("polynomial.power_substitution", ("solve", "x^4-5*x^2+4=0", "x"))
        self.assertEqual({rational(r["root"]["value"]): r["multiplicity"] for r in result["answer"]},
                         {Fraction(-2): 1, Fraction(-1): 1, Fraction(1): 1, Fraction(2): 1})
        self.assertEqual(result["evidence"]["substitution_power"], 2)
        self.assertEqual([rational(c) for c in result["evidence"]["outer_polynomial"]], [4, -5, 1])
        natural = m.school("natural", "solve: x^4−5x^2+4=0")
        self.assertEqual(result["answer"], natural["answer"])
        methods = m.school("methods", "rootsfield", "x^2-2*x+1", "x", "C")
        self.assertEqual(methods["kind"], "checked_method_results")
        self.assertEqual(methods["alternative"]["status"], "verified")
        self.assertEqual(methods["primary"]["outcome"], methods["alternative"]["result"]["outcome"])

    def test_radicals(self):
        exact = self.check_school("radical.principal_real_square_root", ("sqrt", "16"))
        self.assertEqual(rational(exact["answer"]["value"]), 4)
        surd = self.check_school("radical.principal_real_square_root", ("sqrt", "2"))
        self.assertEqual(surd["answer"]["branch"], "upper_real")
        self.assertEqual([rational(c) for c in surd["answer"]["minimal_polynomial"]], [-2, 0, 1])
        self.assertEqual(m.school("sqrt", "-1")["status"], "undefined")
        square = self.check_school("radical.principal_real_square_root", ("sqrt", "x^2"))
        self.assertEqual(square["answer"]["expression"]["kind"], "function")
        self.assertEqual(m.school("sqrt", "x")["status"], "unsupported")

    def test_rational_derivative_and_domain(self):
        result = self.check_school("calculus.rational_quotient_rule",
                                   ("differentiate", "(x^2-1)/(x-1)", "x"), "conditional")
        self.assertEqual(len(result["evidence"]), 2)
        self.assertTrue(result["answer"]["conditions"])
        self.assertEqual(result["request"]["statement"]["source"]["construction_domain"], "R")
        last = result["evidence"][-1]["rational_value"]
        self.assertEqual([rational(c) for c in last["numerator"]], [1])
        self.assertEqual([rational(c) for c in last["denominator"]], [1])
        self.assertTrue(last["excluded_zero_sets"])

    def test_integrals(self):
        parts = self.check_school("calculus.polynomial_trigonometric_parts", ("integrate", "x*sin(x)", "x"))
        self.assertEqual([rational(c) for c in parts["evidence"]["sine_polynomial"]], [1])
        self.assertEqual([rational(c) for c in parts["evidence"]["cosine_polynomial"]], [0, -1])
        self.assertEqual(len(parts["evidence"]["reductions"]), 2)
        substitution = self.check_school("calculus.polynomial_substitution", ("integrate", "2*x*cos(x^2)", "x"))
        self.assertEqual([rational(c) for c in substitution["evidence"]["inner"]], [0, 0, 1])
        self.assertEqual([rational(c) for c in substitution["evidence"]["inner_derivative"]], [0, 2])
        self.assertEqual(rational(substitution["evidence"]["scale"]), 1)

    def test_limits(self):
        finite = self.check_school("calculus.standard_limit", ("limit", "sin(3*x)/x", "x", "0"))
        self.assertEqual(rational(finite["answer"]["value"]), 3)
        cosine = self.check_school("calculus.standard_limit", ("limit", "(1-cos(2*x))/x^2", "x", "0"))
        self.assertEqual(rational(cosine["answer"]["value"]), 2)
        infinity = self.check_school("calculus.rational_infinity_limit", ("limit", "(3*x^2+1)/(2*x^2-1)", "x", "+infinity"))
        self.assertEqual(rational(infinity["answer"]["value"]), Fraction(3, 2))
        negative = self.check_school("calculus.rational_infinity_limit", ("limit", "x^3/(x^2+1)", "x", "-infinity"))
        self.assertEqual(negative["answer"]["kind"], "negative_infinity")

    def test_periodic(self):
        result = self.check_school("trig.periodic_families", ("periodic", "sin", "2", "0", "1/2", "x"))
        self.assertEqual({(rational(f["offset_pi"]), rational(f["period_pi"]), f["parameter_domain"]) for f in result["answer"]},
                         {(Fraction(1, 12), Fraction(1), "Z"), (Fraction(5, 12), Fraction(1), "Z")})
        tangent = self.check_school("trig.periodic_families", ("periodic", "tan", "1", "0", "1", "x"))
        self.assertEqual(rational(tangent["answer"][0]["offset_pi"]), Fraction(1, 4))
        self.assertEqual(rational(tangent["evidence"]["excluded_poles"]["offset_pi"]), Fraction(1, 2))
        empty = self.check_school("trig.periodic_families", ("periodic", "sin", "1", "0", "2", "x"), "no_solutions")
        self.assertEqual(empty["answer"], [])
        identity = self.check_school("trig.squared_identity", ("trig_identity", "1", "1", "0", "-1"))
        self.assertTrue(identity["answer"])

    def test_sign_chart_and_piecewise(self):
        chart = self.check_school("inequality.factored_sign_chart", ("inequality", "(x-1)*(x+2)*(x-3)>=0", "x"))
        intervals = chart["answer"]
        self.assertEqual(len(intervals), 2)
        self.assertEqual(rational(intervals[0]["lower"]["value"]["value"]), -2)
        self.assertEqual(rational(intervals[0]["upper"]["value"]["value"]), 1)
        self.assertEqual(rational(intervals[1]["lower"]["value"]["value"]), 3)
        self.assertEqual(intervals[1]["upper"]["kind"], "positive_infinity")
        self.assertTrue(intervals[0]["lower_closed"])
        self.assertTrue(intervals[0]["upper_closed"])
        self.assertEqual(chart["evidence"]["segment_signs"], [-1, 1, -1, 1])
        abs_sum = self.check_school("piecewise_affine_partition", ("abs_sum", "abs(x-1)+abs(x+1)=4", "x"))
        self.assertEqual([(rational(i["lower"]), rational(i["upper"])) for i in abs_sum["answer"]], [(-2, -2), (2, 2)])
        interval = self.check_school("piecewise_affine_partition", ("abs_sum", "abs(x-1)+abs(x+1)=2", "x"))
        self.assertEqual([(rational(i["lower"]), rational(i["upper"])) for i in interval["answer"]], [(-1, 1)])
        guarded = self.check_school("piecewise_affine_partition", ("piecewise", "x", "-inf,0,0,0,-1,0;0,+inf,1,0,1,0", "-inf,+inf,0,0,0,2"))
        self.assertEqual(guarded["answer"], abs_sum["answer"])
        with self.assertRaises(m.UndefinedAtPointError):
            m.school("abs_sum", "abs(x+0/x)=3", "x")

    def test_legacy_paths_are_unchanged(self):
        self.assertEqual(m.problem("naturalv2", "2x+4=10").status, "complete")
        self.assertEqual(m.problem("natural", "sin(pi/6)").status, "complete")
        self.assertEqual(m.problem("trigzeros", "sin").status, "partial")


if __name__ == "__main__":
    unittest.main()
