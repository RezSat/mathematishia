use mathematishia_cli::run;

#[test]
fn modular_commands_are_exact() {
    let square = run(&["modop", "mul", "x", "x", "x^2+1", "x"]).unwrap();
    assert!(square.contains("residue: -1"));
    let fourth = run(&["modpow", "x", "x^2+1", "4", "x"]).unwrap();
    assert!(fourth.contains("residue: 1"));
    let negative = run(&["modpow", "x", "x^2+1", "-1", "x"]).unwrap();
    assert!(negative.contains("residue: -1 * x"));

    let reduced = run(&["modreduce", "x^2", "2*x^2+2", "x"]).unwrap();
    assert!(reduced.contains("residue: -1"));
}

#[test]
fn nonunits_and_invalid_moduli_have_distinct_outcomes() {
    let nonunit = run(&["modpow", "x-1", "x^2-1", "-1", "x"]).unwrap();
    assert_eq!(nonunit, "residue: undefined (noninvertible element)");
    assert!(run(&["modreduce", "x", "1", "x"]).is_err());
    assert!(run(&["modreduce", "x", "0", "x"]).is_err());
}
