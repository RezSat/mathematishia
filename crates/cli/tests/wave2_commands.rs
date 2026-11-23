use mathematishia_cli::{CliErrorKind, run};

fn expanded(arguments: &[&str]) -> String {
    run(arguments)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("expanded: "))
        .unwrap()
        .to_owned()
}

#[test]
fn expands_binomials_and_products_to_verified_exact_polynomials() {
    assert_eq!(
        expanded(&["expandall", "(x+1)^2"]),
        expanded(&["expandall", "x^2+2*x+1"])
    );
    assert_eq!(
        expanded(&["expandall", "(x-y)^3"]),
        expanded(&["expandall", "x^3-3*x^2*y+3*x*y^2-y^3"])
    );
    assert_eq!(
        expanded(&["expandall", "(x+2)*(x-2)"]),
        expanded(&["expandall", "x^2-4"])
    );
    assert_eq!(
        expanded(&["expandall", "1/2*(x+1)^2"]),
        expanded(&["expandall", "1/2*x^2+x+1/2"])
    );

    let square = run(&["expandall", "(x+1)^2"]).unwrap();
    assert!(square.contains("status: exact"));
    assert!(square.contains("domain: Q[x]"));
    assert!(square.contains("complete: true"));
    assert!(square.contains("verified: true"));
    assert!(!square.contains("polynomial_expansion: not certified"));

    let cube = run(&["expandall", "(x-y)^3"]).unwrap();
    assert!(cube.contains("domain: Q[x, y]"));
    assert!(cube.contains("complete: true"));
}

#[test]
fn explicit_rings_are_canonical_and_incomplete_rings_fail() {
    let first = run(&["expandall", "(x-y)^3", "x", "y"]).unwrap();
    let reversed = run(&["expandall", "(x-y)^3", "y", "x"]).unwrap();
    assert_eq!(first, reversed);
    assert_eq!(first, run(&["expandall", "(x-y)^3"]).unwrap());

    let duplicate = run(&["expandall", "x+y", "x", "x"]).unwrap_err();
    assert_eq!(duplicate.kind(), CliErrorKind::InvalidInput);
    let undeclared = run(&["expandall", "x+y", "x"]).unwrap_err();
    assert_eq!(undeclared.kind(), CliErrorKind::Unsupported);
    assert!(undeclared.to_string().contains("undeclared variable"));
    assert_eq!(
        run(&["expandall", "1", "1"]).unwrap_err().kind(),
        CliErrorKind::InvalidInput
    );
}

#[test]
fn expansion_preserves_historical_exclusions_and_explicit_status() {
    let canceled = run(&["expandall", "0*x^-1"]).unwrap();
    assert!(canceled.contains("status: conditional"));
    assert!(canceled.contains("expanded: 0"));
    assert!(canceled.contains("required: x != 0"));
    assert!(canceled.contains("complete: true"));
    assert!(canceled.contains("verified: true"));

    let denominator = run(&["expandall", "0/(x-1)"]).unwrap();
    assert!(denominator.contains("status: conditional"));
    assert!(denominator.contains("expanded: 0"));
    assert!(denominator.contains("!= 0"));

    let constant = run(&["expandall", "7"]).unwrap();
    assert!(constant.contains("domain: Q[]"));
    assert!(constant.contains("expanded: 7"));
    assert!(constant.contains("conditions: none"));
}

#[test]
fn polynomial_expansion_refuses_unsupported_and_resource_limited_inputs() {
    for input in ["sin(x)", "log(x)", "x^0", "x^-1"] {
        let refusal = run(&["expandall", input]).unwrap_err();
        assert_eq!(
            refusal.kind(),
            CliErrorKind::Unsupported,
            "{input}: {refusal}"
        );
    }

    assert_eq!(
        run(&["expandall", "x^129"]).unwrap_err().kind(),
        CliErrorKind::Resource
    );
    assert_eq!(
        run(&["expandall", "x^(1/2)"]).unwrap_err().kind(),
        CliErrorKind::Domain
    );

    let too_many_variables = (0..17).map(|i| format!("v{i}")).collect::<Vec<_>>();
    let mut arguments = vec!["expandall", "1"];
    arguments.extend(too_many_variables.iter().map(String::as_str));
    assert_eq!(run(&arguments).unwrap_err().kind(), CliErrorKind::Resource);
}

#[test]
fn integrates_polynomial_and_affine_analytic_sums_with_checked_term_evidence() {
    let sum = run(&["integrate", "x^2+sin(2*x)"]).unwrap();
    assert!(sum.contains("status: exact"));
    assert!(sum.contains("terms: 2"));
    assert!(sum.contains("exact polynomial primitive"));
    assert!(sum.contains("calculus.integrate_affine_sine"));
    assert!(sum.contains("verified: true"));

    let mixed = run(&["integrate", "x^2+sin(2*x)+cos(3*x)+exp(x)"]).unwrap();
    assert!(mixed.contains("terms: 4"));
    assert!(mixed.contains("calculus.integrate_affine_sine"));
    assert!(mixed.contains("calculus.integrate_affine_cosine"));
    assert!(mixed.contains("calculus.integrate_affine_exponential"));
    assert!(mixed.contains("conditions: none"));
    assert!(mixed.contains("verified: true"));

    assert!(
        run(&["integrate", "x^2", "x", "0", "2"])
            .unwrap()
            .contains("integral: 8/3")
    );
    assert!(
        run(&["integrate", "x^2+1", "x", "0", "2"])
            .unwrap()
            .contains("integral: 14/3")
    );
}

#[test]
fn integration_retains_source_restrictions_and_refuses_unsafe_definite_integrals() {
    let historical = run(&["integrate", "x^2+0*x^-1"]).unwrap();
    assert!(historical.contains("status: conditional"));
    assert!(historical.contains("required: x != 0"));
    assert!(historical.contains("verified: true"));

    let sum = run(&["integrate", "sin(x)+0*x^-1"]).unwrap();
    assert!(sum.contains("status: conditional"));
    assert!(sum.contains("required: x != 0"));
    assert!(sum.contains("calculus.integrate_affine_sine"));

    assert_eq!(
        run(&["integrate", "x^2+0*x^-1", "x", "1", "2"])
            .unwrap_err()
            .kind(),
        CliErrorKind::Domain
    );
    assert_eq!(
        run(&["integrate", "x^2+sin(x)", "x", "0", "1"])
            .unwrap_err()
            .kind(),
        CliErrorKind::Unsupported
    );
}

#[test]
fn integration_rejects_unsupported_forms_and_budgets_without_false_results() {
    for expression in [
        "exp(x)*cos(x)",
        "x*sin(x)",
        "tan(x)",
        "log(x)",
        "1/x",
        "sin(x^2)",
        "sin(x)+y",
        "2*(sin(x)+cos(x))",
    ] {
        let error = run(&["integrate", expression]).unwrap_err();
        assert_eq!(
            error.kind(),
            CliErrorKind::Unsupported,
            "{expression}: {error}"
        );
    }
    assert_eq!(
        run(&["integrate", "x^33"]).unwrap_err().kind(),
        CliErrorKind::Resource
    );

    let many_terms = (1..=65)
        .map(|i| format!("sin(x+{i})"))
        .collect::<Vec<_>>()
        .join("+");
    assert_eq!(
        run(&["integrate", &many_terms]).unwrap_err().kind(),
        CliErrorKind::Resource
    );
}

#[test]
fn new_commands_are_deterministic_and_do_not_change_legacy_expansion() {
    let cases: &[&[&str]] = &[
        &["expandall", "(x+1)^2"],
        &["expandall", "(x-y)^3"],
        &["expandall", "0*x^-1"],
        &["integrate", "x^2+sin(2*x)"],
        &["integrate", "x^2+0*x^-1"],
        &["integrate", "x^2+1", "x", "2", "0"],
    ];
    for &args in cases {
        assert_eq!(run(args).unwrap(), run(args).unwrap(), "{args:?}");
    }

    let legacy = run(&["expand", "(x+1)^2"]).unwrap();
    assert!(legacy.contains("polynomial_expansion: not certified"));
    assert!(!legacy.contains("\ncomplete: true"));
    assert_eq!(
        run(&["expandall", "(x+1)^2", "--json"]).unwrap_err().kind(),
        CliErrorKind::Unsupported
    );
    assert_eq!(
        run(&["integrate", "x^2", "x", "0", "1", "--json"])
            .unwrap_err()
            .kind(),
        CliErrorKind::Unsupported
    );
}
