use mathematishia_cli::run;

#[test]
fn polynomial_factorization_distinguishes_certified_from_partial() {
    let complete = run(&["factor", "(x-1)*(x^2+1)", "x"]).unwrap();
    assert!(complete.contains("status: complete"));
    assert!(complete.contains("factor:"));
    assert!(!complete.contains("unresolved:"));
    let partial = run(&["factor", "(x^2+1000)*(x^2+2000)", "x"]).unwrap();
    assert!(partial.contains("status: partial"));
    assert!(partial.contains("unresolved:"));
}

#[test]
fn cubic_field_arithmetic_and_minimal_polynomials_are_exact() {
    let cube = run(&["fieldpow", "x^3-2", "x", "3", "x"]).unwrap();
    assert!(cube.contains("element: 2"));
    let inverse = run(&["fieldop", "mul", "x^3-2", "x", "x^2", "x"]).unwrap();
    assert!(inverse.contains("element: 2"));
    let min = run(&["fieldmin", "x^3-2", "x^2", "x"]).unwrap();
    assert!(min.contains("minimal_polynomial:"));
    assert!(min.contains("verified: true"));
}

#[test]
fn field_commands_reject_reducible_inputs_and_zero_inversion() {
    assert!(run(&["fieldpow", "x^3-1", "x", "2", "x"]).is_err());
    assert!(run(&["fieldpow", "x^3-2", "0", "-1", "x"]).is_err());
    let quartic = run(&["fieldmin", "x^4+1", "x", "x"]).unwrap();
    assert!(quartic.contains("verified: true"));
}
