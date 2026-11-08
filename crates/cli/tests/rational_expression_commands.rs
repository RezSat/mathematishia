use mathematishia_cli::run;

#[test]
fn rational_simplification_keeps_canceled_poles() {
    let result = run(&["rsimplify", "(x^2-1)/(x-1)", "x"]).unwrap();
    assert!(result.contains("numerator: 1 + x"));
    assert!(result.contains("denominator: 1"));
    assert!(result.contains("condition: -1 + x != 0"));

    let zero = run(&["rsimplify", "0/(x-1)", "x"]).unwrap();
    assert!(zero.contains("numerator: 0"));
    assert!(zero.contains("condition: -1 + x != 0"));
}

#[test]
fn evaluation_and_differentiation_keep_original_domain() {
    assert!(run(&["reval", "(x^2-1)/(x-1)", "x", "1"]).is_err());
    assert_eq!(
        run(&["reval", "(x^2-1)/(x-1)", "x", "2"]).unwrap(),
        "value: 3"
    );
    let result = run(&["rdiff", "(x^2-1)/(x-1)", "x"]).unwrap();
    assert!(result.contains("numerator: 1"));
    assert!(result.contains("condition: -1 + x != 0"));
}

#[test]
fn rational_equation_solving_checks_removed_points() {
    let excluded = run(&["rsolve", "(x^2-1)/(x-1)=2", "x"]).unwrap();
    assert!(excluded.contains("rational roots: none"));
    assert!(excluded.contains("condition: -1 + x != 0"));

    let solution = run(&["rsolve", "(x^2-1)/(x-1)=3", "x"]).unwrap();
    assert!(solution.contains("rational roots: 2"));

    let identity = run(&["rsolve", "0/(x-1)=0", "x"]).unwrap();
    assert!(identity.contains("rational roots: all admissible rationals"));
    assert!(identity.contains("condition: -1 + x != 0"));
}

#[test]
fn invalid_rational_domains_are_not_simplified() {
    assert!(run(&["rsimplify", "1/(x-x)", "x"]).is_err());
    assert!(run(&["rsimplify", "1/(x+y)", "x"]).is_err());
    assert!(run(&["rsolve", "x+1", "x"]).is_err());
}
