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

    def test_exact_quadratic_roots_are_typed_and_domain_explicit(self):
        result = m.solve_quadratic("x^2-5*x+6", "x")
        self.assertEqual(result.kind, "finite")
        self.assertEqual(result.roots, (Fraction(2), Fraction(3)))
        self.assertTrue(result.has_rational_roots)
        self.assertEqual(m.solve_quadratic("(x-2)^2", "x").roots, (Fraction(2),))
        irrational = m.solve_quadratic("x^2-2", "x")
        self.assertEqual(irrational.roots, ())
        self.assertFalse(irrational.has_rational_roots)
        all_values = m.solve_quadratic("0", "x")
        self.assertEqual(all_values.kind, "all_rationals")
        self.assertTrue(all_values.has_rational_roots)
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.solve_quadratic("x^3+1", "x")
        with self.assertRaises(FrozenInstanceError):
            result.roots = ()

    def test_exact_real_and_complex_quadratic_roots_with_proof(self):
        irrational = m.algebraic_roots("x^2-2", "x")
        self.assertEqual(irrational.kind, "finite")
        self.assertEqual([root.kind for root in irrational.roots],
                         ["real_quadratic", "real_quadratic"])
        self.assertEqual([root.branch for root in irrational.roots],
                         ["lower_real", "upper_real"])
        self.assertEqual(irrational.roots[0].minimal_polynomial.coefficients,
                         (Fraction(-2), Fraction(0), Fraction(1)))
        self.assertEqual(irrational.steps[1].discriminant, Fraction(8))
        self.assertEqual(irrational.steps[1].rule, "roots.quadratic.discriminant")
        self.assertEqual(irrational.steps[-1].rule,
                         "roots.quadratic.irrational_real_pair")

        complex_roots = m.algebraic_roots("x^2+1", "x")
        self.assertEqual([root.kind for root in complex_roots.roots],
                         ["complex_quadratic", "complex_quadratic"])
        self.assertEqual([root.branch for root in complex_roots.roots],
                         ["negative_imaginary", "positive_imaginary"])

        rational = m.algebraic_roots("x^2-5*x+6", "x")
        self.assertEqual([root.rational for root in rational.roots],
                         [Fraction(2), Fraction(3)])
        self.assertEqual(m.algebraic_roots("0", "x").kind, "all_complex")
        self.assertEqual(m.algebraic_roots("7", "x").roots, ())
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.algebraic_roots("x^3-1", "x")
        with self.assertRaises(FrozenInstanceError):
            irrational.roots[0].branch = "upper_real"

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

    def test_domain_aware_rational_function_and_holes(self):
        result = m.rational_function("x^2-1", "x-1", "x")
        self.assertEqual(result.numerator.coefficients, (Fraction(1), Fraction(1)))
        self.assertEqual(result.denominator.coefficients, (Fraction(1),))
        self.assertEqual(
            [p.coefficients for p in result.excluded_zero_sets],
            [(Fraction(-1), Fraction(1))],
        )
        self.assertEqual(m.rational_evaluate("x^2-1", "x-1", "x", "2"), Fraction(3))
        with self.assertRaises(m.UndefinedAtPointError):
            m.rational_evaluate("x^2-1", "x-1", "x", "1")
        with self.assertRaises(FrozenInstanceError):
            result.denominator = m.polynomial("1", "x")

    def test_exact_quotient_rule_on_rational_functions(self):
        result = m.rational_differentiate("1", "x-1", "x")
        self.assertEqual(result.numerator.coefficients, (Fraction(-1),))
        self.assertEqual(
            result.denominator.coefficients,
            (Fraction(1), Fraction(-2), Fraction(1)),
        )
        self.assertEqual(len(result.excluded_zero_sets), 1)

    def test_rational_arithmetic_and_typed_invalid_domains(self):
        for operation in ["add", "sub", "mul", "div"]:
            result = m.rational_operation(operation, "x+1", "x-1", "1", "x", "x")
            self.assertIsInstance(result, m.RationalFunction)
            self.assertTrue(result.excluded_zero_sets)
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rational_function("x", "0", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rational_operation("div", "1", "1", "0", "1", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rational_operation("??", "1", "1", "1", "1", "x")

    def test_domain_aware_exact_quadratic_equations(self):
        result = m.rational_solve_quadratic("x^2-5*x+6", "x-2", "0", "1", "x")
        self.assertEqual(result.kind, "finite")
        self.assertEqual(result.roots, (Fraction(3),))
        self.assertEqual(
            [poly.coefficients for poly in result.excluded_zero_sets],
            [(Fraction(-2), Fraction(1))],
        )
        identity = m.rational_solve_quadratic("x-1", "x-1", "1", "1", "x")
        self.assertEqual(identity.kind, "all_admissible")
        self.assertEqual(identity.roots, ())
        self.assertTrue(identity.excluded_zero_sets)
        irrational = m.rational_solve_quadratic("x^2-2", "x", "0", "1", "x")
        self.assertEqual(irrational.roots, ())
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rational_solve_quadratic("x^3", "1", "0", "1", "x")
        with self.assertRaises(FrozenInstanceError):
            result.roots = ()

    def test_typed_errors(self):
        with self.assertRaises(m.MathSyntaxError):
            m.expression("x // 2")
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



class DomainApiTests(unittest.TestCase):
    def test_domain_values_survive_zero_normalization(self):
        value = m.domain_expression("0*x^-1")
        self.assertEqual(value.expression, "0")
        self.assertEqual(value.conditions, ("x != 0",))
        with self.assertRaises(FrozenInstanceError):
            value.expression = "1"

    def test_blocked_and_checked_composite_rewriting(self):
        expression = "(x+1)*(x+1)^-1"
        blocked = m.rewrite_domain(expression)
        self.assertEqual(blocked.status, "blocked")
        self.assertEqual(blocked.outstanding, ("1 + x != 0",))
        rewritten = m.rewrite_domain(expression, ("x+1",))
        self.assertEqual(rewritten.status, "applied")
        self.assertEqual(rewritten.expression, "1")
        self.assertEqual(rewritten.conditions, ("1 + x != 0",))
        self.assertEqual(rewritten.rule, "algebra.cancel_composite_inverse")
        self.assertEqual(len(rewritten.proof), 3)
        self.assertEqual(rewritten.proof[-1].parents, (0, 1))
        self.assertEqual(rewritten.proof[-1].rule, rewritten.rule)

    def test_atomic_rewrite_and_original_holes(self):
        result = m.rewrite_domain("x*x^-1", ("x",))
        self.assertEqual(result.status, "applied")
        self.assertEqual(result.conditions, ("x != 0",))
        self.assertEqual(result.rule, "algebra.cancel_symbol_inverse")
        original = m.rewrite_domain("0*x^-1", ("x",))
        self.assertEqual(original.status, "unchanged")
        self.assertEqual(original.conditions, ("x != 0",))

    def test_domain_equations_and_substitution(self):
        equation = m.domain_equation("0*x^-1=0*y^-2")
        self.assertEqual(equation.left.expression, "0")
        self.assertEqual(equation.left.conditions, ("x != 0",))
        self.assertEqual(equation.right.expression, "0")
        self.assertEqual(equation.right.conditions, ("y != 0",))

        replaced = m.substitute_domain("0*x^-1", "x", "0")
        self.assertEqual(replaced.expression, "0")
        self.assertEqual(replaced.conditions, ("0 != 0",))
        replacement = m.substitute_domain("0*x^-1", "x", "y^-1")
        self.assertEqual(replacement.expression, "0")
        self.assertEqual(len(replacement.conditions), 2)
        self.assertIn("y != 0", replacement.conditions)

        with self.assertRaises(m.MathSyntaxError):
            m.domain_equation("x+1")

    def test_domain_errors_are_typed(self):
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.domain_expression("x^(1/2)")
        with self.assertRaises(m.MathSyntaxError):
            m.domain_expression("x//2")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rewrite_domain("x*x^-1", ("0",))
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rewrite_domain("x*x^-1", ("x^-1",))



class RationalExpressionApiTests(unittest.TestCase):
    def test_reduces_full_expression_and_preserves_canceled_domain(self):
        result = m.rational_simplify("(x^2-1)/(x-1)", "x")
        self.assertEqual(result.numerator.coefficients, (Fraction(1), Fraction(1)))
        self.assertEqual(result.denominator.coefficients, (Fraction(1),))
        self.assertEqual(
            [poly.coefficients for poly in result.excluded_zero_sets],
            [(Fraction(-1), Fraction(1))],
        )
        zero = m.rational_simplify("0/(x-1)", "x")
        self.assertEqual(zero.numerator.coefficients, ())
        self.assertEqual(len(zero.excluded_zero_sets), 1)

    def test_exact_calculus_and_evaluation_reject_removed_poles(self):
        self.assertEqual(
            m.rational_value("(x^2-1)/(x-1)", "x", "2"),
            Fraction(3),
        )
        with self.assertRaises(m.UndefinedAtPointError):
            m.rational_value("(x^2-1)/(x-1)", "x", "1")
        derivative = m.rational_derivative("(x^2-1)/(x-1)", "x")
        self.assertEqual(derivative.numerator.coefficients, (Fraction(1),))
        self.assertEqual(
            derivative.excluded_zero_sets[0].coefficients,
            (Fraction(-1), Fraction(1)),
        )

    def test_equations_keep_original_domain(self):
        excluded = m.rational_solve("(x^2-1)/(x-1)=2", "x")
        self.assertEqual(excluded.kind, "finite")
        self.assertEqual(excluded.roots, ())
        self.assertEqual(
            excluded.excluded_zero_sets[0].coefficients,
            (Fraction(-1), Fraction(1)),
        )
        self.assertEqual(m.rational_solve("(x^2-1)/(x-1)=3", "x").roots,
                         (Fraction(2),))
        identity = m.rational_solve("0/(x-1)=0", "x")
        self.assertEqual(identity.kind, "all_admissible")
        self.assertEqual(len(identity.excluded_zero_sets), 1)

    def test_rejects_unsupported_domains(self):
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rational_simplify("1/(x-x)", "x")
        with self.assertRaises(m.UnsupportedMathematicsError):
            m.rational_simplify("1/(x+y)", "x")
        with self.assertRaises(m.MathSyntaxError):
            m.rational_solve("x+1", "x")



class RationalExpressionCrossChecks(unittest.TestCase):
    def test_linear_factor_reduction_and_removed_roots(self):
        # Rational root cancellation is compared against exact integer
        # arithmetic across 49 distinct polynomials.
        for removed in range(-3, 4):
            for surviving in range(-3, 4):
                source = (
                    f"(x-({removed}))*(x-({surviving}))/"
                    f"(x-({removed}))"
                )
                with self.subTest(removed=removed, surviving=surviving):
                    reduced = m.rational_simplify(source, "x")
                    self.assertEqual(
                        reduced.numerator.coefficients,
                        (Fraction(-surviving), Fraction(1)),
                    )
                    self.assertEqual(
                        reduced.denominator.coefficients,
                        (Fraction(1),),
                    )
                    self.assertEqual(
                        reduced.excluded_zero_sets[0].coefficients,
                        (Fraction(-removed), Fraction(1)),
                    )
                    with self.assertRaises(m.UndefinedAtPointError):
                        m.rational_value(source, "x", str(removed))
                    admissible = removed + 1
                    self.assertEqual(
                        m.rational_value(source, "x", str(admissible)),
                        Fraction(admissible - surviving),
                    )
                    roots = m.rational_solve(source + "=0", "x")
                    expected = () if removed == surviving else (Fraction(surviving),)
                    self.assertEqual(roots.roots, expected)


if __name__ == "__main__":
    unittest.main()
