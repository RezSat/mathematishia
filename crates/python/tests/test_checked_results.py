import json
import unittest
from fractions import Fraction

import mathematishia as m


class CheckedResultTests(unittest.TestCase):
    def test_dispatch_schema(self):
        result = m.solveauto("x=1")
        data = json.loads(result.to_json())
        self.assertEqual(data["schema_version"], 1)
        self.assertEqual(data["outcome"]["status"], "complete")
        self.assertEqual(data["outcome"]["answer"]["roots"][0]["root"]["value"],
                         {"numerator": "1", "denominator": "1"})
        self.assertIsNotNone(data["evidence"])
        self.assertEqual(result, m.problem("solveauto", "x=1", "x"))
        self.assertEqual(m.evalexact("sin(0)+cos(0)").as_dict()["outcome"]["answer"]["value"],
                         {"numerator": "1", "denominator": "1"})
        partial = m.solveauto("x^3-2=0")
        self.assertEqual(partial.status, "partial")
        self.assertEqual(m.solveauto("sin(x)=0").status, "unsupported")

    def test_shared_operations_and_restrictions(self):
        cases = (
            ("expandall", "(x+1)^2"),
            ("integrate", "x^2+sin(2*x)", "x"),
            ("limit", "(x^2-1)/(x-1)", "x", "1"),
            ("inequality", "0/(x-1)<=0/(x+2)", "x"),
        )
        for case in cases:
            result = m.problem(*case)
            self.assertEqual(result.status, "complete")
            self.assertEqual(result.to_json(), m.problem(*case).to_json())
        limit = m.problem(*cases[2]).as_dict()
        self.assertTrue(limit["restrictions"])
        self.assertTrue(m.problem(*cases[3]).as_dict()["restrictions"])
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.problem("limit", "x^2", "x", "1+0/x")

    def test_root_isolation(self):
        roots = m.isolate_real_roots("x^2-2", "x", "1/32")
        self.assertEqual(len(roots), 2)
        self.assertLess(roots[0].upper, roots[1].lower)
        for root in roots:
            self.assertLessEqual(root.upper-root.lower, Fraction(1, 32))
            self.assertEqual(root.midpoint, (root.lower+root.upper)/2)
            self.assertEqual(root.error_bound, (root.upper-root.lower)/2)
            self.assertLess((root.lower**2-2)*(root.upper**2-2), 0)
        self.assertEqual(m.isolate_real_roots("x^2+1", "x", "1/32"), ())
        for source in ["(x-1)^2", "0", "x+0/x"]:
            with self.assertRaises(m.UnsupportedMathematicsError):
                m.isolate_real_roots(source, "x", "1/32")


if __name__ == "__main__":
    unittest.main()
