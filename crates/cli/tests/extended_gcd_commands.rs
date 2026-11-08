use mathematishia_cli::run;

#[test]
fn extended_gcd_displays_complete_bezout_witness() {
    let output = run(&["egcd", "x^3-1", "x^2-1", "x"]).unwrap();
    assert!(output.contains("gcd: -1 + x"));
    assert!(output.contains("left_coefficient:"));
    assert!(output.contains("right_coefficient:"));
    assert!(output.contains("divisions:"));
    assert!(output.contains(" = ("));
}

#[test]
fn modular_inverse_has_distinct_success_and_nonunit_outcomes() {
    let inverse = run(&["modinv", "x", "x^2+1", "x"]).unwrap();
    assert!(inverse.starts_with("inverse:"));
    assert!(inverse.contains('x'));

    let absent = run(&["modinv", "x-1", "x^2-1", "x"]).unwrap();
    assert_eq!(absent, "inverse: none (noncoprime polynomials)");
    assert!(run(&["modinv", "x", "0", "x"]).is_err());
}
