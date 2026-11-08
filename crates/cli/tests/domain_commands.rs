use mathematishia_cli::run;

#[test]
fn domain_command_preserves_holes_after_zero_folding() {
    let output = run(&["domain", "0*x^-1"]).unwrap();
    assert!(output.contains("expression: 0"));
    assert!(output.contains("condition: x != 0"));
}

#[test]
fn composite_cancellation_requires_a_matched_premise() {
    let input = "(x+1)*(x+1)^-1";
    let blocked = run(&["rewrite", input, "x"]).unwrap();
    assert!(blocked.contains("status: blocked"));
    assert!(blocked.contains("required: 1 + x != 0"));

    let applied = run(&["rewrite", input, "x+1"]).unwrap();
    assert!(applied.contains("status: applied"));
    assert!(applied.contains("expression: 1"));
    assert!(applied.contains("condition: 1 + x != 0"));
    assert!(applied.contains("rule: algebra.cancel_composite_inverse"));
    assert!(applied.contains("proof_nodes: 3"));
}

#[test]
fn simple_inverse_cancellation_and_unchanged_results_keep_domain() {
    let applied = run(&["rewrite", "x*x^-1", "x"]).unwrap();
    assert!(applied.contains("rule: algebra.cancel_symbol_inverse"));
    assert!(applied.contains("condition: x != 0"));

    let unchanged = run(&["rewrite", "0*x^-1", "x"]).unwrap();
    assert!(unchanged.contains("status: unchanged"));
    assert!(unchanged.contains("condition: x != 0"));
}

#[test]
fn rejects_invalid_premises_and_unsupported_powers() {
    assert!(run(&["rewrite", "x*x^-1", "0"]).is_err());
    assert!(run(&["rewrite", "x*x^-1", "x^-1"]).is_err());
    assert!(run(&["domain", "x^(1/2)"]).is_err());
}
