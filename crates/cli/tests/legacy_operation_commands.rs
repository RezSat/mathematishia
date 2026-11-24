use mathematishia_cli::{CliErrorKind, run};
use std::process::Command;

#[test]
fn affine_systems_distinguish_unique_empty_and_affine_sets() {
    let unique = run(&["linsystem", "y,x", "x+y=3", "x-y=1"]).unwrap();
    assert!(unique.contains("variables: y, x"));
    assert!(unique.contains("solution: unique"));
    assert!(unique.contains("values: 1,2"));
    let empty = run(&["linsystem", "x,y", "x+y=1", "x+y=2"]).unwrap();
    assert!(empty.contains("solution: empty"));
    let affine = run(&["linsystem", "x,y", "x+y=1/2"]).unwrap();
    assert!(affine.contains("solution: affine"));
    assert!(affine.contains("particular: 1/2,0"));
    assert!(affine.contains("basis: -1,1"));
    for answer in [unique, empty, affine] {
        assert!(answer.ends_with("conditions: none\nverified: true"));
    }
}

#[test]
fn affine_systems_refuse_nonaffine_and_restricted_equations() {
    for (args, kind) in [
        (
            &["linsystem", "x,y", "x*y=1"][..],
            CliErrorKind::Unsupported,
        ),
        (&["linsystem", "x", "x+y=1"][..], CliErrorKind::Unsupported),
        (&["linsystem", "x", "x+0/x=1"][..], CliErrorKind::Domain),
        (&["linsystem", "x,x", "x=1"][..], CliErrorKind::InvalidInput),
        (&["linsystem", "x", "x+1"][..], CliErrorKind::InvalidInput),
        (&["linsystem", "x"][..], CliErrorKind::InvalidInput),
    ] {
        assert_eq!(run(args).unwrap_err().kind(), kind, "{args:?}");
    }
    let mut args = vec!["linsystem", "x"];
    args.extend(["x=1"; 17]);
    assert_eq!(run(&args).unwrap_err().kind(), CliErrorKind::Resource);
}

#[test]
fn ordered_multivariate_division_preserves_priority_and_ring() {
    let first = run(&["mvdivide", "y,x", "x*y+1", "x", "y"]).unwrap();
    let second = run(&["mvdivide", "x,y", "x*y+1", "y", "x"]).unwrap();
    assert!(first.contains("variables: x, y"));
    assert!(first.contains("monomial_order: lexicographic"));
    assert!(first.contains("quotient 1: y"));
    assert!(second.contains("quotient 1: x"));
    for answer in [first, second] {
        assert!(answer.contains("quotient 2: 0"));
        assert!(answer.contains("remainder: 1"));
        assert!(answer.ends_with("conditions: none\nverified: true"));
    }
    let first = run(&["mvdivide", "x,y", "x+y", "x", "x+y"]).unwrap();
    let second = run(&["mvdivide", "x,y", "x+y", "x+y", "x"]).unwrap();
    assert!(first.contains("remainder: y"));
    assert!(second.contains("remainder: 0"));
    let zero = run(&["mvdivide", "x,y", "0", "x"]).unwrap();
    assert!(zero.contains("quotient 1: 0"));
    assert!(zero.contains("remainder: 0"));
}

#[test]
fn ordered_multivariate_division_rejects_inadmissible_inputs() {
    for (args, kind) in [
        (&["mvdivide", "x,y", "x+0/y", "x"][..], CliErrorKind::Domain),
        (&["mvdivide", "x,y", "x", "y+0/x"][..], CliErrorKind::Domain),
        (&["mvdivide", "x,y", "x", "0"][..], CliErrorKind::Domain),
        (&["mvdivide", "x", "y", "x"][..], CliErrorKind::Unsupported),
        (&["mvdivide", "x", "x^129", "x"][..], CliErrorKind::Resource),
        (
            &["mvdivide", "x,x", "x", "x"][..],
            CliErrorKind::InvalidInput,
        ),
        (&["mvdivide", "x", "x"][..], CliErrorKind::InvalidInput),
    ] {
        assert_eq!(run(args).unwrap_err().kind(), kind, "{args:?}");
    }
    let mut args = vec!["mvdivide", "x", "x"];
    args.extend(["x"; 33]);
    assert_eq!(run(&args).unwrap_err().kind(), CliErrorKind::Resource);
}

#[test]
fn rational_root_brackets_distinguish_existence_precision_and_unresolved() {
    let exact = run(&["rootbracket", "x-1", "x", "1", "2", "1/16", "0"]).unwrap();
    assert!(exact.contains("root: 1"));
    assert!(exact.contains("precision_met: true"));
    let midpoint = run(&["rootbracket", "x-1", "x", "0", "2", "1/16", "1"]).unwrap();
    assert!(midpoint.contains("root: 1"));
    assert!(midpoint.contains("bisections: 1"));
    let bracket = run(&["rootbracket", "x^2-2", "x", "1", "2", "1/16", "8"]).unwrap();
    assert!(bracket.contains("status: partial"));
    assert!(bracket.contains("claim: at least one root; uniqueness not established"));
    assert!(bracket.contains("precision_met: true"));
    let exhausted = run(&["rootbracket", "x^2-2", "x", "1", "2", "1/16", "0"]).unwrap();
    assert!(exhausted.contains("interval: (1, 2)"));
    assert!(exhausted.contains("precision_met: false"));
    let same = run(&["rootbracket", "x^2", "x", "-1", "1", "1/16", "8"]).unwrap();
    assert!(same.contains("claim: unresolved; neither existence nor absence established"));
    assert!(same.contains("precision_met: false"));
    for answer in [exact, bracket, exhausted, same] {
        assert!(answer.ends_with("conditions: none\nverified: true"));
    }
}

#[test]
fn root_brackets_reject_bad_bounds_and_discarded_restrictions() {
    for (source, lower, upper, width, max, kind) in [
        ("0", "1", "2", "1/16", "8", CliErrorKind::Unsupported),
        ("x^2-2", "2", "1", "1/16", "8", CliErrorKind::InvalidInput),
        ("x^2-2", "1", "2", "0", "8", CliErrorKind::InvalidInput),
        ("x^2-2", "1", "2", "1/16", "-1", CliErrorKind::InvalidInput),
        ("x^2-2", "1+0/y", "2", "1/16", "8", CliErrorKind::Domain),
        ("x^2+0/x", "1", "2", "1/16", "8", CliErrorKind::Domain),
        ("x^33-2", "1", "2", "1/16", "8", CliErrorKind::Resource),
    ] {
        assert_eq!(
            run(&["rootbracket", source, "x", lower, upper, width, max])
                .unwrap_err()
                .kind(),
            kind,
        );
    }
}

#[test]
fn compiled_legacy_commands_return_checked_answers_and_refusals() {
    let binary = env!("CARGO_BIN_EXE_mathematishia");
    for (args, expected) in [
        (&["linsystem", "x,y", "x+y=1"][..], "solution: affine"),
        (&["mvdivide", "x,y", "x*y", "x", "y"][..], "quotient 1: y"),
        (
            &["rootbracket", "x^2", "x", "-1", "1", "1/8", "8"][..],
            "claim: unresolved",
        ),
    ] {
        let output = Command::new(binary).args(args).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8(output.stdout).unwrap().contains(expected));
    }
    for args in [
        &["linsystem", "x", "x+0/x=1"][..],
        &["mvdivide", "x,y", "x+0/y", "x"][..],
        &["rootbracket", "x^2+0/x", "x", "-1", "1", "1/8", "8"][..],
    ] {
        let output = Command::new(binary).args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("source-domain restrictions")
        );
    }
}
