use mathematishia_cli::run;

#[test]
fn equation_domains_survive_zero_folding_on_both_sides() {
    let out = run(&["domain", "0*x^-1=0*y^-2"]).unwrap();
    assert!(out.contains("equation: 0 = 0"));
    assert!(out.contains("left condition: x != 0"));
    assert!(out.contains("right condition: y != 0"));
}

#[test]
fn substitution_updates_restrictions_when_expression_is_constant() {
    let output = run(&["subdomain", "0*x^-1", "x", "0"]).unwrap();
    assert!(output.contains("expression: 0"));
    assert!(output.contains("condition: 0 != 0"));
    let replacement = run(&["subdomain", "0*x^-1", "x", "y^-1"]).unwrap();
    assert!(replacement.contains("condition: y != 0"));
    assert!(replacement.contains("expression: 0"));
}
