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
