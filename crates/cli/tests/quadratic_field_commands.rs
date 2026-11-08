use mathematishia_cli::run;

#[test]
fn quadratic_commands_evaluate_exact_embedded_numbers() {
    let sqrt_two = run(&["qvalue", "x^2-2", "upper_real", "1", "2", "x"]).unwrap();
    assert!(sqrt_two.contains("value: 1 + (2)*alpha"));
    assert!(sqrt_two.contains("embedding: upper_real"));
    assert!(sqrt_two.contains("root_branch: upper_real"));
    assert!(sqrt_two.contains("norm: -7"));
    assert!(sqrt_two.contains("trace: 2"));

    let square_i = run(&[
        "qop",
        "mul",
        "x^2+1",
        "positive_imaginary",
        "0",
        "1",
        "0",
        "1",
        "x",
    ])
    .unwrap();
    assert!(square_i.contains("value: -1 + (0)*alpha"));
    assert!(square_i.contains("root_branch: rational"));
}

#[test]
fn quadratic_inverse_and_input_rejection() {
    let inverse = run(&["qinv", "x^2-2", "upper_real", "0", "1", "x"]).unwrap();
    assert!(inverse.contains("value: 0 + (1/2)*alpha"));
    assert!(run(&["qinv", "x^2-2", "upper_real", "0", "0", "x"]).is_err());
    assert!(run(&["qvalue", "x^2-4", "upper_real", "1", "1", "x"]).is_err());
    assert!(run(&["qvalue", "x^2+1", "upper_real", "1", "1", "x"]).is_err());
}
