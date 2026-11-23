use mathematishia_cli::{CliErrorKind, run};

#[test]
fn automatic_solve_classifies_linear_quadratic_and_partial_higher_degree_roots() {
    let linear = run(&["solveauto", "2*x+4=10"]).unwrap();
    assert!(linear.contains("status: exact"));
    assert!(linear.contains("solution: x = 3"));
    assert!(linear.contains("verified: true"));

    let irrational = run(&["solveauto", "x^2-2=0"]).unwrap();
    assert!(irrational.contains("status: exact"));
    assert!(irrational.contains("lower_real"));
    assert!(irrational.contains("upper_real"));

    let complex = run(&["solveauto", "x^2+1=0"]).unwrap();
    assert!(complex.contains("negative_imaginary"));
    assert!(complex.contains("positive_imaginary"));

    let cubic = run(&["solveauto", "x^3-2=0"]).unwrap();
    assert!(cubic.contains("status: partial"));
    assert!(cubic.contains("unresolved_cofactor:"));
    assert!(cubic.contains("verified: true"));

    assert!(run(&["solveauto", "x=x"]).unwrap().contains("all rational values"));
    assert!(run(&["solveauto", "x+1=x+2"]).unwrap().contains("solution: none"));
}

#[test]
fn rational_equations_do_not_lose_canceled_poles() {
    let excluded = run(&["solveauto", "(x^2-1)/(x-1)=2"]).unwrap();
    assert!(excluded.contains("status: partial over C"));
    assert!(excluded.contains("rational roots: none"));
    assert!(excluded.contains("condition: -1 + x != 0"));

    let unique = run(&["solveauto", "(x^2-1)/(x-1)=3"]).unwrap();
    assert!(unique.contains("rational roots: 2"));
    assert!(unique.contains("condition: -1 + x != 0"));

    let identity = run(&["solveauto", "0*x^-1=0"]).unwrap();
    assert!(identity.contains("all admissible rational values"));
    assert!(identity.contains("condition: x != 0"));
    assert!(!identity.contains("solution: all complex numbers"));
}

#[test]
fn exact_root_sets_preserve_multiplicity_and_unresolved_cofactors() {
    let repeated = run(&["roots", "x^2-4*x+4"]).unwrap();
    assert!(repeated.contains("status: exact"));
    assert!(repeated.contains("root 1: 2"));
    assert!(repeated.contains("multiplicity: 2"));
    assert!(run(&["roots", "0"]).unwrap().contains("all complex numbers"));
    assert!(run(&["roots", "5"]).unwrap().contains("roots: 0"));

    let partial = run(&["roots", "x^3-2"]).unwrap();
    assert!(partial.contains("status: partial"));
    assert!(partial.contains("unresolved_cofactor:"));
    assert!(!partial.contains("status: exact"));
}

#[test]
fn polynomial_and_elementary_integration_are_checked() {
    let indefinite = run(&["integrate", "x^2"]).unwrap();
    assert!(indefinite.contains("status: exact"));
    assert!(indefinite.contains("1/3"));
    assert!(indefinite.contains("verified: true"));

    let forward = run(&["integrate", "x^2", "x", "0", "2"]).unwrap();
    assert!(forward.contains("integral: 8/3"));
    let reverse = run(&["integrate", "x^2", "x", "2", "0"]).unwrap();
    assert!(reverse.contains("integral: -8/3"));

    let sine = run(&["integrate", "sin(2*x+1)"]).unwrap();
    assert!(sine.contains("calculus.integrate_affine_sine"));
    assert!(sine.contains("verified: true"));
    assert!(run(&["integrate", "exp(x)"])
        .unwrap()
        .contains("calculus.integrate_affine_exponential"));
}

#[test]
fn exact_analytic_identities_do_not_pretend_to_evaluate_general_functions() {
    for (source, expected) in [
        ("sin(0)", "value: 0"),
        ("cos(0)", "value: 1"),
        ("tan(0)", "value: 0"),
        ("exp(0)", "value: 1"),
        ("log(1)", "value: 0"),
        ("sin(0)+cos(0)", "value: 1"),
        ("1/2+1/3", "value: 5/6"),
    ] {
        let out = run(&["evalexact", source]).unwrap();
        assert!(out.contains(expected), "{source}: {out}");
        assert!(out.contains("verified: true"));
    }
    let symbolic = run(&["evalexact", "sin(x)"]).unwrap();
    assert!(symbolic.contains("status: partial"));
    assert!(!symbolic.contains("value:"));
}

#[test]
fn negative_inputs_and_unsupported_domains_are_refused() {
    for args in [
        &["solveauto", "x^2-1"][..],
        &["solveauto", "sin(x)=0"][..],
        &["solveauto", "x+y=1", "x"][..],
        &["roots", "x^-1"][..],
        &["roots", "x^33"][..],
        &["integrate", "exp(x)*cos(x)"][..],
        &["integrate", "1/x"][..],
        &["integrate", "x^-1", "x", "0", "1"][..],
        &["evalexact", "log(-1)"][..],
        &["evalexact", "0^0"][..],
        &["solveauto", "x=1", "--json"][..],
        &["solveauto", "x=2", "2"][..],
        &["roots", "x^2", "x+y"][..],
    ] {
        assert!(run(args).is_err(), "unexpected success: {args:?}");
    }
    assert_eq!(
        run(&["roots", "x^33"]).unwrap_err().kind(),
        CliErrorKind::Resource
    );
    assert_eq!(
        run(&["solveauto", "x=1", "--json"]).unwrap_err().kind(),
        CliErrorKind::Unsupported
    );
    assert_eq!(
        run(&["roots", "x^-1"]).unwrap_err().kind(),
        CliErrorKind::Domain
    );
}

#[test]
fn previous_expand_command_does_not_claim_certified_power_expansion() {
    let untouched = run(&["expand", "(x+1)^2"]).unwrap();
    assert!(untouched.contains("polynomial_expansion: not certified"));
    assert!(!untouched.contains("\ncomplete: true"));
    let distributed = run(&["expand", "2*(x+3)", "2"]).unwrap();
    assert!(distributed.contains("distribution_complete: true"));
}

#[test]
fn fixed_38_case_cli_smoke_corpus_is_deterministic() {
    let cases: &[(&[&str], &str)] = &[
        (&["solveauto", "2*x+4=10"], "solution: x = 3"),
        (&["solveauto", "x=0"], "solution: x = 0"),
        (&["solveauto", "x+1=x+2"], "solution: none"),
        (&["solveauto", "x=x"], "all rational values"),
        (&["solveauto", "x^2-2=0"], "lower_real"),
        (&["solveauto", "x^2+1=0"], "negative_imaginary"),
        (&["solveauto", "x^2-4=0"], "status: exact"),
        (&["solveauto", "x^3-2=0"], "status: partial"),
        (&["solveauto", "(x^2-1)/(x-1)=3"], "rational roots: 2"),
        (&["solveauto", "(x^2-1)/(x-1)=2"], "rational roots: none"),
        (&["solveauto", "0*x^-1=0"], "all admissible rational values"),
        (&["roots", "0"], "all complex numbers"),
        (&["roots", "1"], "roots: 0"),
        (&["roots", "x"], "root 1: 0"),
        (&["roots", "x^2-4*x+4"], "multiplicity: 2"),
        (&["roots", "x^2-2"], "upper_real"),
        (&["roots", "x^2+1"], "positive_imaginary"),
        (&["roots", "x^3-2"], "unresolved_cofactor:"),
        (&["integrate", "x"], "primitive:"),
        (&["integrate", "x^2"], "1/3"),
        (&["integrate", "5"], "primitive:"),
        (&["integrate", "0"], "primitive: 0"),
        (&["integrate", "x^2", "x", "0", "2"], "integral: 8/3"),
        (&["integrate", "x^2", "x", "2", "0"], "integral: -8/3"),
        (&["integrate", "sin(2*x+1)"], "calculus.integrate_affine_sine"),
        (&["integrate", "cos(x)"], "calculus.integrate_affine_cosine"),
        (&["integrate", "exp(x)"], "calculus.integrate_affine_exponential"),
        (&["evalexact", "sin(0)"], "value: 0"),
        (&["evalexact", "cos(0)"], "value: 1"),
        (&["evalexact", "tan(0)"], "value: 0"),
        (&["evalexact", "exp(0)"], "value: 1"),
        (&["evalexact", "log(1)"], "value: 0"),
        (&["evalexact", "sin(0)+cos(0)"], "value: 1"),
        (&["evalexact", "1/2+1/3"], "value: 5/6"),
        (&["evalexact", "sin(x)"], "status: partial"),
        (&["expand", "(x+1)^2"], "polynomial_expansion: not certified"),
        (&["expand", "2*(x+3)"], "distribution_complete: true"),
        (&["poly", "(x+1)^2", "x"], "coefficients: [1, 2, 1]"),
    ];
    assert_eq!(cases.len(), 38);
    for (args, expected) in cases {
        let output = run(args).unwrap_or_else(|error| panic!("{args:?}: {error}"));
        assert!(
            output.contains(expected),
            "{args:?}: expected {expected:?}, got {output:?}"
        );
        assert_eq!(run(args).unwrap(), output, "nondeterministic output: {args:?}");
    }
}
