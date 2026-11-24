use mathematishia_cli::{CliErrorKind, run};

#[test]
fn finite_limits_preserve_holes_and_classify_poles() {
    let hole = run(&["limit", "(x^2-1)/(x-1)", "x", "1"]).unwrap();
    assert!(hole.contains("limit: 2\n"));
    assert!(hole.contains("excluded_at_point: true"));
    assert!(hole.contains("condition: -1 + x != 0"));
    for (direction, expected) in [
        ("left", "-infinity"),
        ("right", "infinity"),
        ("two-sided", "does not exist"),
    ] {
        let result = run(&["limit", "1/x", "x", "0", direction]).unwrap();
        assert!(result.contains(&format!("limit: {expected}\n")));
        assert!(result.contains("denominator_order: 1"));
    }
    let even = run(&["limit", "1/x^2", "x", "0"]).unwrap();
    assert!(even.contains("limit: infinity\n"));
    assert!(even.contains("denominator_order: 2"));
}

#[test]
fn inequality_routing_and_domains_are_consistent() {
    let explicit = run(&["inequality", "x^2-4>0", "x"]).unwrap();
    assert_eq!(explicit, run(&["solveauto", "x^2-4>0"]).unwrap());
    assert_eq!(explicit, run(&["solve", "x^2-4>0", "x"]).unwrap());
    assert!(explicit.contains("interval: (-infinity, -2)"));
    assert!(explicit.contains("interval: (2, infinity)"));
    let holes = run(&["inequality", "0/(x-1)<=0/(x+2)"]).unwrap();
    assert!(holes.contains("intervals: 3"));
    assert!(holes.contains("condition: -1 + x != 0"));
    assert!(holes.contains("condition: 2 + x != 0"));
    assert!(holes.contains("interval: (-2, 1)"));
    let singleton = run(&["inequality", "x^2≤0"]).unwrap();
    assert!(singleton.contains("interval: [0, 0]"));
    let irrational = run(&["inequality", "x^2<2"]).unwrap();
    assert!(irrational.contains("lower_real of minimal polynomial"));
    assert!(irrational.contains("upper_real of minimal polynomial"));
}

#[test]
fn unsupported_forms_and_invalid_input_are_not_guessed() {
    for args in [
        &["limit", "sin(x)/x", "x", "0"][..],
        &["limit", "1/x", "x", "infinity"],
        &["inequality", "x^3>0"],
        &["inequality", "sin(x)>0"],
    ] {
        assert_eq!(run(args).unwrap_err().kind(), CliErrorKind::Unsupported);
    }
    assert_eq!(
        run(&["inequality", "0<x<1"]).unwrap_err().kind(),
        CliErrorKind::InvalidInput
    );
    assert_eq!(
        run(&["limit", "1/0", "x", "0"]).unwrap_err().kind(),
        CliErrorKind::Domain
    );
}
