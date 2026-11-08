use std::process::Command;

#[test]
fn actual_binary_solves_a_linear_equation() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["solve", "2*x+4=10", "x"])
        .output()
        .expect("CLI binary should run");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("solution: x = 3"));
    assert!(text.contains("verified: 2 != 0"));
}

#[test]
fn actual_binary_reports_exact_polynomial_division() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["divide", "x^2-1", "x-1", "x"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("remainder: 0")
    );
}

#[test]
fn actual_binary_reports_errors_on_stderr_with_nonzero_status() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["solve", "x^2=4", "x"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("not linear")
    );
}

#[test]
fn actual_binary_reports_exact_square_free_factors() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["sqfree", "2*(x-1)^2*(x+2)^3", "x"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("unit: 2\n"));
    assert!(stdout.contains("factor (multiplicity 2):"));
    assert!(stdout.contains("factor (multiplicity 3):"));
}

#[test]
fn actual_binary_rejects_zero_square_free_decomposition() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["sqfree", "0", "x"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("undefined for zero")
    );
}

#[test]
fn actual_binary_exposes_verified_symbolic_derivatives() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["diffexpr", "x*y+x^3", "x"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("derivative: "));
    assert!(stdout.contains("calculus.product; derivative_of"));
}

#[test]
fn actual_binary_rejects_undefined_negative_powers() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["diffexpr", "x^-1", "x"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("positive integer exponents")
    );
}

#[test]
fn actual_binary_respects_canceled_rational_holes() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["rateval", "x^2-1", "x-1", "x", "1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("undefined at this point")
    );
}

#[test]
fn actual_binary_solves_exact_rational_quadratic_roots() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["solveq", "x^2-5*x+6", "x"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "rational roots: 2, 3"
    );
}

#[test]
fn actual_binary_rejects_canceled_quadratic_root() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["ratsolveq", "x^2-5*x+6", "x-2", "0", "1", "x"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("rational roots: 3\n"));
    assert!(stdout.contains("condition:"));
}

#[test]
fn binary_reports_exact_irrational_algebraic_roots_and_proof() {
    let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(["roots2", "x^2-2", "x"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let displayed = String::from_utf8(output.stdout).unwrap();
    assert!(displayed.contains("lower_real"));
    assert!(displayed.contains("roots.quadratic.discriminant"));
}
