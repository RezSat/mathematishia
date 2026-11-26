use mathematishia_cli::{CliErrorKind, run};
use std::process::Command;

#[test]
fn sturm_reports_exact_counts_and_deterministic_evidence() {
    for (source, lower, upper, count) in [
        ("x^2-2", "-2", "2", 2),
        ("x^2-2", "0.0", "2", 1),
        ("x^2+1", "-2", "2", 0),
        ("3", "-1/2", "1/2", 0),
        ("x^3-x", "-2", "2", 3),
    ] {
        let args = ["sturm", source, "x", lower, upper];
        let result = run(&args).unwrap();
        assert!(
            result.contains(&format!("root_count: {count}\n")),
            "{result}"
        );
        assert!(result.ends_with("conditions: none\nverified: true"));
        assert_eq!(result, run(&args).unwrap());
    }
    let result = run(&["sturm", "x^2-2", "x", "0", "2"]).unwrap();
    assert!(result.contains("lower_sign: zero"));
    assert!(result.contains("lower_variations: 1"));
    assert!(result.contains("upper_variations: 0"));
}

#[test]
fn sturm_refuses_invalid_domains_and_unsupported_forms() {
    for (source, lower, upper, kind) in [
        ("0", "-2", "2", CliErrorKind::Unsupported),
        ("(x-1)^2", "-2", "2", CliErrorKind::Unsupported),
        ("(x-5)^2*(x-1)", "-2", "2", CliErrorKind::Unsupported),
        ("x^2-1", "1", "2", CliErrorKind::Domain),
        ("x^2-2", "2", "-2", CliErrorKind::Unsupported),
        ("x^2-2", "2", "2", CliErrorKind::Unsupported),
        ("x^2+0/x", "-2", "2", CliErrorKind::Domain),
        ("(x-x)/x+x", "-2", "2", CliErrorKind::Domain),
        ("x^2-2", "0/y", "2", CliErrorKind::Domain),
        ("x^2-2", "1/0", "2", CliErrorKind::Domain),
        ("x+y", "-2", "2", CliErrorKind::Unsupported),
        ("sin(x)", "-2", "2", CliErrorKind::Unsupported),
        ("x^2-2", "infinity", "2", CliErrorKind::Unsupported),
        ("x^2-2", "0..0", "2", CliErrorKind::InvalidInput),
        ("x^33-2", "-2", "2", CliErrorKind::Resource),
    ] {
        assert_eq!(
            run(&["sturm", source, "x", lower, upper])
                .unwrap_err()
                .kind(),
            kind,
            "{source}, {lower}, {upper}"
        );
    }
    assert_eq!(
        run(&["sturm", "x^2-2", "x+0", "-2", "2"])
            .unwrap_err()
            .kind(),
        CliErrorKind::InvalidInput
    );
    assert_eq!(
        run(&["sturm", "x^2-2", "x", "-2", "2", "--json"])
            .unwrap_err()
            .kind(),
        CliErrorKind::Unsupported
    );
}

#[test]
fn compiled_sturm_command_succeeds_and_refuses_without_false_result() {
    let binary = env!("CARGO_BIN_EXE_mathematishia");
    let result = Command::new(binary)
        .args(["sturm", "x^2-2", "x", "-2", "2"])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("root_count: 2")
    );
    let refused = Command::new(binary)
        .args(["sturm", "x^2+0/x", "x", "-2", "2"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
    assert!(
        String::from_utf8(refused.stderr)
            .unwrap()
            .contains("source-domain restrictions")
    );
}
