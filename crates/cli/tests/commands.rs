use mathematishia_cli::{HELP, run};

#[test]
fn displays_help_with_zero_arguments() {
    assert_eq!(run(&[]).unwrap(), HELP);
    assert!(run(&["--help"]).unwrap().contains("mathematishia solve"));
}

#[test]
fn expression_command_performs_conservative_semantic_normalization() {
    assert_eq!(run(&["expr", "1+2*3"]).unwrap(), "expression: 7");
    assert_eq!(run(&["expr", "1/2+1/3"]).unwrap(), "expression: 5/6");
    assert_eq!(run(&["expr", "x^2"]).unwrap(), "expression: x ^ 2");
}

#[test]
fn expansion_shows_complete_and_partial_derivations() {
    let full = run(&["expand", "2*(x+3)", "2"]).unwrap();
    assert!(full.starts_with("result: "));
    assert!(full.contains("steps: 1\ncomplete: true"));
    assert!(full.contains("[algebra.distribute_product_over_sum]"));
    let partial = run(&["expand", "2*(x+3)", "0"]).unwrap();
    assert!(partial.contains("steps: 0\ncomplete: false"));
}

#[test]
fn linear_solver_reports_classification_and_justified_steps() {
    let solved = run(&["solve", "2*x+4=10", "x"]).unwrap();
    assert!(solved.starts_with("solution: x = 3"));
    assert!(solved.contains("[equation.divide_by_nonzero_coefficient]"));
    assert!(solved.contains("(verified: 2 != 0)"));
    assert_eq!(
        run(&["solve", "x+1=x+2", "x"]).unwrap().lines().next(),
        Some("solution: none in Q")
    );
    assert_eq!(
        run(&["solve", "x=x", "x"]).unwrap().lines().next(),
        Some("solution: all rational values")
    );
}

#[test]
fn rejects_invalid_input_without_panicking() {
    for command in [
        vec!["solve", "x=2"],
        vec!["solve", "x+3", "x"],
        vec!["solve", "x^2=4", "x"],
        vec!["expr", "3/0"],
        vec!["expand", "x*(1+y)", "13"],
        vec!["expand", "x*(1+y)", "abc"],
        vec!["solve", "x=1", "2"],
        vec!["unknown"],
    ] {
        assert!(run(&command).is_err(), "command should fail: {command:?}");
    }
}

#[test]
fn polynomial_cli_reports_exact_coefficients() {
    let output = run(&["poly", "(x+1)^2", "x"]).unwrap();
    assert!(output.contains("degree: 2"));
    assert!(output.contains("coefficients: [1, 2, 1]"));
    let zero = run(&["poly", "0", "x"]).unwrap();
    assert!(zero.contains("degree: undefined"));
    assert!(zero.contains("coefficients: []"));
}

#[test]
fn polynomial_evaluation_is_exact() {
    assert_eq!(run(&["eval", "x^2+1/2", "x", "1/2"]).unwrap(), "value: 3/4");
    assert_eq!(run(&["eval", "x^2+1", "x", "-2"]).unwrap(), "value: 5");
}

#[test]
fn cli_division_reconstructs_exact_quotient_and_remainder() {
    let output = run(&["divide", "x^2-1", "x-1", "x"]).unwrap();
    assert!(output.contains("quotient: "));
    assert!(output.contains("remainder: 0"));
    let remainder = run(&["divide", "x^2+1", "x+1", "x"]).unwrap();
    assert!(remainder.contains("remainder: 2"));
}

#[test]
fn cli_gcd_is_monic_and_deterministic() {
    let lhs = run(&["gcd", "x^2-1", "x-1", "x"]).unwrap();
    let rhs = run(&["gcd", "x-1", "x^2-1", "x"]).unwrap();
    assert_eq!(lhs, rhs);
    assert!(lhs.starts_with("gcd: "));
    assert_eq!(run(&["gcd", "0", "0", "x"]).unwrap(), "gcd: 0");
}

#[test]
fn polynomial_cli_rejects_invalid_domains_and_division_by_zero() {
    for command in [
        vec!["poly", "y+1", "x"],
        vec!["poly", "x^-1", "x"],
        vec!["poly", "x^33", "x"],
        vec!["eval", "x", "x", "y"],
        vec!["divide", "x", "0", "x"],
        vec!["divide", "x", "y", "x"],
        vec!["gcd", "x", "x", "2"],
    ] {
        assert!(run(&command).is_err(), "must reject {command:?}");
    }
}

#[test]
fn cli_uses_resource_context_instead_of_unbounded_polynomial_execution() {
    let huge = format!(
        "{}^32",
        "99999999999999999999999999999999999999999999999999999999"
    );
    let response = run(&["poly", &huge, "x"]);
    assert!(response.is_err());
    assert!(
        response
            .unwrap_err()
            .to_string()
            .contains("resource limit exceeded")
    );
}

#[test]
fn differentiates_supported_polynomials_exactly() {
    let result = run(&["diff", "x^3+1/2*x", "x"]).unwrap();
    assert!(result.starts_with("derivative: "));
    assert!(result.contains("3 * x ^ 2"));
    assert_eq!(run(&["diff", "5", "x"]).unwrap(), "derivative: 0");
    assert_eq!(run(&["diff", "0", "x"]).unwrap(), "derivative: 0");
}

#[test]
fn polynomial_differentiation_rejects_unsupported_domains() {
    for expr in ["x^-1", "x^33", "x+y"] {
        assert!(run(&["diff", expr, "x"]).is_err());
    }
}

#[test]
fn square_free_cli_reports_unit_and_multiplicity_classes() {
    let output = run(&["sqfree", "2*(x-1)^2*(x+2)^3", "x"]).unwrap();
    assert!(output.starts_with("unit: 2\n"));
    assert!(output.contains("factor (multiplicity 2):"));
    assert!(output.contains("factor (multiplicity 3):"));
    assert!(output.find("multiplicity 2").unwrap() < output.find("multiplicity 3").unwrap());

    assert_eq!(run(&["sqfree", "-3/4", "x"]).unwrap(), "unit: -3/4");
    let irreducible = run(&["sqfree", "x^2+1", "x"]).unwrap();
    assert!(irreducible.contains("factor (multiplicity 1):"));
    assert_eq!(irreducible.matches("factor (multiplicity").count(), 1);
}

#[test]
fn square_free_cli_rejects_zero_and_unsupported_domains() {
    for expr in ["0", "x^-1", "x^33", "y+1"] {
        assert!(run(&["sqfree", expr, "x"]).is_err());
    }
}

#[test]
fn exact_symbolic_derivative_cli_shows_verified_rule_ids() {
    let result = run(&["diffexpr", "x*y+x^3", "x"]).unwrap();
    assert!(result.starts_with("derivative: "));
    assert!(result.contains("steps: "));
    assert!(result.contains("[calculus.product; derivative_of]"));
    assert!(result.contains("[calculus.positive_integer_power; derivative_of]"));
    assert_eq!(
        run(&["diffexpr", "x*y", "y"]).unwrap().lines().next(),
        Some("derivative: x")
    );
    assert_eq!(
        run(&["diffexpr", "x^2", "x"]).unwrap().lines().next(),
        Some("derivative: 2 * x")
    );
}

#[test]
fn symbolic_derivative_cli_rejects_unsupported_exponents() {
    for expression in ["x^-1", "x^0", "x^(1/2)", "x^y", "x^33"] {
        assert!(run(&["diffexpr", expression, "x"]).is_err());
    }
}

#[test]
fn rational_cli_displays_removed_poles_and_exact_values() {
    let fraction = run(&["rat", "x^2-1", "x-1", "x"]).unwrap();
    assert!(fraction.contains("numerator: "));
    assert!(fraction.contains("denominator: 1"));
    assert!(fraction.contains("condition: "));
    assert!(fraction.contains("!= 0"));
    assert_eq!(
        run(&["rateval", "x^2-1", "x-1", "x", "2"]).unwrap(),
        "value: 3"
    );
    assert!(run(&["rateval", "x^2-1", "x-1", "x", "1"]).is_err());
}

#[test]
fn rational_cli_supports_all_four_exact_binary_operations() {
    for operation in ["add", "sub", "mul", "div"] {
        let args = ["ratop", operation, "x+1", "x-1", "1", "x", "x"];
        let result = run(&args).unwrap();
        assert!(result.starts_with("numerator: "));
        assert!(result.contains("denominator: "));
        assert!(result.contains("condition: "));
    }
    assert!(run(&["ratop", "div", "1", "1", "0", "1", "x"]).is_err());
    assert!(run(&["ratop", "bad", "1", "1", "1", "1", "x"]).is_err());
}

#[test]
fn rational_cli_uses_quotient_rule_and_rejects_zero_denominator() {
    let derivative = run(&["ratdiff", "1", "x-1", "x"]).unwrap();
    assert!(derivative.contains("numerator: -1"));
    assert!(derivative.contains("denominator: "));
    assert!(derivative.contains("condition: "));
    assert!(run(&["rat", "x", "0", "x"]).is_err());
}

#[test]
fn exact_quadratic_command_distinguishes_no_rational_roots_from_no_solutions() {
    assert_eq!(
        run(&["solveq", "x^2-5*x+6", "x"]).unwrap(),
        "rational roots: 2, 3"
    );
    assert_eq!(
        run(&["solveq", "x^2-4*x+4", "x"]).unwrap(),
        "rational roots: 2"
    );
    assert_eq!(
        run(&["solveq", "x^2-2", "x"]).unwrap(),
        "rational roots: none (other roots may exist)"
    );
    assert_eq!(
        run(&["solveq", "0", "x"]).unwrap(),
        "rational roots: all rational values"
    );
    assert!(run(&["solveq", "x^3-1", "x"]).is_err());
}

#[test]
fn quadratic_rational_cli_preserves_removed_poles() {
    let answer = run(&["ratsolveq", "x^2-5*x+6", "x-2", "0", "1", "x"]).unwrap();
    assert!(answer.starts_with("rational roots: 3\n"));
    assert!(answer.contains("condition:"));
    assert!(answer.contains("!= 0"));
    let identity = run(&["ratsolveq", "x-1", "x-1", "1", "1", "x"]).unwrap();
    assert!(identity.starts_with("rational roots: all admissible rationals"));
    assert!(identity.contains("condition:"));
    assert!(run(&["ratsolveq", "x^3", "1", "0", "1", "x"]).is_err());
}

#[test]
fn exact_algebraic_roots_cli_exposes_typed_evidence() {
    let real = run(&["roots2", "x^2-2", "x"]).unwrap();
    assert!(real.starts_with("roots: 2\n"));
    assert!(real.contains("lower_real"));
    assert!(real.contains("upper_real"));
    assert!(real.contains("roots.normalize_monic"));
    assert!(real.contains("roots.quadratic.discriminant"));
    assert!(real.contains("roots.quadratic.irrational_real_pair"));

    let complex = run(&["roots2", "x^2+1", "x"]).unwrap();
    assert!(complex.contains("negative_imaginary"));
    assert!(complex.contains("positive_imaginary"));
    assert!(complex.contains("roots.quadratic.nonreal_conjugates"));

    let rational = run(&["roots2", "x^2-5*x+6", "x"]).unwrap();
    assert!(rational.contains("root 1: 2"));
    assert!(rational.contains("root 2: 3"));
    assert_eq!(
        run(&["roots2", "0", "x"]).unwrap().lines().next(),
        Some("roots: all complex numbers")
    );
    assert!(run(&["roots2", "x^3-1", "x"]).is_err());
}
