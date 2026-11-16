use mathematishia_cli::run;

#[test]
fn real_derivative_displays_unproved_conditions_and_verified_rules() {
    let output = run(&["diffreal", "tan(log(x))", "x"]).unwrap();
    assert!(output.contains("derivative:"));
    assert!(output.contains("required:"));
    assert!(output.contains("x > 0"));
    assert!(output.contains("calculus.log_chain"));
    assert!(output.contains("calculus.tan_chain"));
    let entire = run(&["diffexpr", "sin(x)", "x"]).unwrap();
    assert!(entire.contains("derivative: cos(x)"));
}

#[test]
fn invalid_function_and_power_inputs_return_explicit_errors() {
    assert!(run(&["diffreal", "custom(x)", "x"]).is_err());
    assert!(run(&["diffreal", "x^(-1)", "x"]).is_err());
}
