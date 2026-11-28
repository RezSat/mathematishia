use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use mathematishia_cli::{CliErrorKind, run};

const MANIFEST: &str = include_str!("../../../tests/independent_qa/wave2_held_out.tsv");

fn classified_line(line: &str) -> bool {
    [
        "root ",
        "interval: ",
        "known_pi_coefficient: ",
        "condition: ",
        "conditions: ",
        "required: ",
        "left condition: ",
        "right condition: ",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn check_case(args: &[&str], expected: &[&str], outcome: &str) {
    let execute = || {
        Command::new(env!("CARGO_BIN_EXE_mathematishia"))
            .args(args)
            .output()
            .expect("execute compiled CLI")
    };
    let first = execute();
    let second = execute();
    assert_eq!(first, second, "nondeterministic invocation: {args:?}");
    let stdout = String::from_utf8(first.stdout).unwrap();
    let stderr = String::from_utf8(first.stderr).unwrap();

    if outcome == "refused" && !expected[0].starts_with("status: ") {
        assert_eq!(first.status.code(), Some(2), "{args:?}: {stdout}");
        assert!(stdout.is_empty());
        assert!(stderr.starts_with("error: "));
        let kind = match expected[0] {
            "InvalidInput" => CliErrorKind::InvalidInput,
            "Resource" => CliErrorKind::Resource,
            _ => panic!("unknown refusal kind"),
        };
        assert_eq!(run(args).unwrap_err().kind(), kind);
        for line in &expected[1..] {
            assert!(stderr.lines().any(|actual| actual == *line));
        }
        return;
    }

    assert_eq!(first.status.code(), Some(0), "{args:?}: {stderr}");
    assert!(stderr.is_empty());
    let actual: Vec<_> = stdout.lines().collect();
    for line in expected {
        assert!(actual.contains(line), "{args:?}: missing {line:?}: {stdout}");
    }
    let selected = |lines: &[&str]| {
        lines
            .iter()
            .filter(|line| classified_line(line))
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(selected(&actual), selected(expected), "{args:?}");
    let status: Vec<_> = actual
        .iter()
        .filter(|line| line.starts_with("status: "))
        .copied()
        .collect();
    assert_eq!(status, vec![expected[0]]);
    if outcome != "solved" {
        assert!(!actual.contains(&"complete: true"), "{args:?}");
    }
}

#[test]
fn held_out_compiled_cli_matches_exact_oracles() {
    let mut ids = BTreeSet::new();
    let mut invocations = BTreeSet::new();
    let mut counts = BTreeMap::new();
    let mut failures = Vec::new();
    for line in MANIFEST.lines() {
        if line.starts_with('#') || line.starts_with("id\t") || line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 6);
        let [id, category, outcome, arguments, checks, oracle] = fields.as_slice() else {
            unreachable!()
        };
        assert!(ids.insert(*id));
        assert!(invocations.insert(*arguments));
        assert!(!oracle.is_empty());
        assert!(matches!(*outcome, "solved" | "partial" | "refused"));
        let args: Vec<_> = arguments.split('¦').collect();
        let expected: Vec<_> = checks.split('¦').collect();
        let result = std::panic::catch_unwind(|| check_case(&args, &expected, outcome));
        let status = if result.is_ok() {
            *outcome
        } else {
            failures.push(*id);
            "wrong_or_contract_mismatch"
        };
        *counts.entry((*category, status)).or_insert(0usize) += 1;
    }
    assert!(ids.len() >= 100);
    println!("held_out_cases={}", ids.len());
    println!("category\toutcome\tcount");
    for ((category, status), count) in counts {
        println!("{category}\t{status}\t{count}");
    }
    println!("independently_verified_teaching_steps=0");
    assert!(failures.is_empty(), "failed held-out cases: {failures:?}");
}
