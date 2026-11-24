use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use mathematishia_cli::{CliErrorKind, run};

const FIXTURES: &str = include_str!("../../../tests/problem_corpus/user_questions.tsv");

#[derive(Debug)]
struct Question<'a> {
    id: &'a str,
    category: &'a str,
    question: &'a str,
    outcome: &'a str,
    expected_exit: i32,
    error_kind: &'a str,
    arguments: Vec<&'a str>,
    semantic_lines: Vec<&'a str>,
    conditions: Vec<&'a str>,
    proof: &'a str,
    reason: &'a str,
    source: &'a str,
}

fn fields(value: &str) -> Vec<&str> {
    if value == "-" {
        Vec::new()
    } else {
        value.split('¦').collect()
    }
}

fn questions() -> Vec<Question<'static>> {
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    let mut commands = BTreeSet::new();

    for (index, line) in FIXTURES.lines().enumerate() {
        if line.starts_with('#') || line.starts_with("id\t") || line.is_empty() {
            continue;
        }
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 12, "fixture line {}", index + 1);
        let arguments = fields(columns[6]);
        assert!(
            !arguments.is_empty(),
            "fixture {} has no command",
            columns[0]
        );
        assert!(
            ids.insert(columns[0]),
            "duplicate question id {}",
            columns[0]
        );
        assert!(
            commands.insert(columns[6]),
            "duplicate CLI invocation {}",
            columns[6]
        );
        assert!(!columns[2].is_empty(), "missing question {}", columns[0]);
        assert!(!columns[10].is_empty(), "missing reason {}", columns[0]);
        assert!(
            !fields(columns[7]).is_empty(),
            "missing semantic expectation {}",
            columns[0]
        );
        result.push(Question {
            id: columns[0],
            category: columns[1],
            question: columns[2],
            outcome: columns[3],
            expected_exit: columns[4].parse().expect("integer exit status"),
            error_kind: columns[5],
            arguments,
            semantic_lines: fields(columns[7]),
            conditions: fields(columns[8]),
            proof: columns[9],
            reason: columns[10],
            source: columns[11],
        });
    }

    assert!(result.len() >= 100, "expected at least 100 questions");
    let categories: BTreeSet<_> = result.iter().map(|question| question.category).collect();
    for required in [
        "arithmetic",
        "algebra",
        "equations",
        "derivatives",
        "integrals",
        "limits",
        "matrices",
        "rational_domains",
        "trigonometry",
        "unsupported",
    ] {
        assert!(categories.contains(required), "missing category {required}");
    }
    for (index, question) in result.iter().take(38).enumerate() {
        assert_eq!(question.id, format!("{:03}", index + 1));
        assert_eq!(question.source, "audit-38");
    }
    result
}

fn condition_line(line: &str) -> bool {
    [
        "condition: ",
        "conditions: ",
        "required: ",
        "left condition: ",
        "right condition: ",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn check_proof(question: &Question<'_>, output: &str) {
    let lines: Vec<_> = output.lines().collect();
    let mut parts = question.proof.split(';');
    let specification = parts.next().expect("nonempty proof specification");
    if specification == "none" {
        assert!(
            !lines.iter().any(|line| {
                line.starts_with("steps: ")
                    || line.starts_with("evidence: ")
                    || line.starts_with("rule: ")
            }),
            "{}: unexpected claimed proof in {output:?}",
            question.id
        );
        assert_eq!(
            parts.count(),
            0,
            "{}: invalid proof specification",
            question.id
        );
        return;
    }
    let (kind, count) = specification
        .split_once('=')
        .expect("proof needs steps=N, evidence=N or rules=N");
    let expected: usize = count.parse().expect("proof count is a nonnegative integer");
    match kind {
        "steps" | "evidence" => {
            assert!(
                lines.contains(&format!("{kind}: {expected}").as_str()),
                "{}: incorrect proof count in {output:?}",
                question.id
            );
            let recorded = lines
                .iter()
                .filter(|line| {
                    let (number, _) = line.split_once(". ").unwrap_or(("", ""));
                    !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
                })
                .count();
            assert_eq!(
                recorded, expected,
                "{}: proof record count does not match header",
                question.id
            );
        }
        "terms" => {
            assert!(lines.contains(&format!("terms: {expected}").as_str()));
            assert_eq!(
                lines
                    .iter()
                    .filter(|line| line.starts_with("term "))
                    .count(),
                expected,
                "{}: unexpected integration evidence count",
                question.id
            );
        }
        "rules" => {
            assert_eq!(
                lines
                    .iter()
                    .filter(|line| line.starts_with("rule: "))
                    .count(),
                expected,
                "{}: unexpected analytic rewrite count",
                question.id
            );
        }
        _ => panic!("{}: unknown proof form {kind}", question.id),
    }
    for rule in parts {
        assert!(
            lines.iter().any(|line| {
                line.contains(&format!("[{rule}"))
                    || *line == format!("rule: {rule}")
                    || (line.starts_with("term ") && line.ends_with(&format!(": {rule}")))
            }),
            "{}: missing required proof rule {rule} in {output:?}",
            question.id
        );
    }
}

fn check_question(question: &Question<'_>) {
    let execute = || {
        Command::new(env!("CARGO_BIN_EXE_mathematishia"))
            .args(&question.arguments)
            .output()
            .unwrap_or_else(|error| panic!("{}: cannot execute CLI: {error}", question.id))
    };
    let output = execute();
    let repeated = execute();
    assert_eq!(
        (output.status.code(), &output.stdout, &output.stderr),
        (repeated.status.code(), &repeated.stdout, &repeated.stderr),
        "{}: nondeterministic process output",
        question.id
    );
    assert_eq!(
        output.status.code(),
        Some(question.expected_exit),
        "{} {} ({:?}): unexpected exit status",
        question.id,
        question.question,
        question.arguments
    );
    let stdout = String::from_utf8(output.stdout).expect("CLI output must be UTF-8");
    let stderr = String::from_utf8(output.stderr).expect("CLI error must be UTF-8");
    let displayed = if question.expected_exit == 0 {
        assert!(
            stderr.is_empty(),
            "{}: unexpected stderr: {stderr}",
            question.id
        );
        assert_eq!(
            question.error_kind, "-",
            "{}: success has error kind",
            question.id
        );
        &stdout
    } else {
        assert!(
            stdout.is_empty(),
            "{}: failure emitted stdout: {stdout}",
            question.id
        );
        assert!(
            stderr.starts_with("error: ") && stderr.trim().len() > "error: ".len(),
            "{}: no typed error text: {stderr:?}",
            question.id
        );
        let error = run(&question.arguments).expect_err("binary error must be typed in library");
        let expected = match question.error_kind {
            "InvalidInput" => CliErrorKind::InvalidInput,
            "Unsupported" => CliErrorKind::Unsupported,
            "Domain" => CliErrorKind::Domain,
            "Resource" => CliErrorKind::Resource,
            "Verification" => CliErrorKind::Verification,
            kind => panic!("{}: unknown error kind {kind}", question.id),
        };
        assert_eq!(
            error.kind(),
            expected,
            "{}: incorrect typed error",
            question.id
        );
        &stderr
    };

    let actual: Vec<_> = displayed.lines().collect();
    for expected in &question.semantic_lines {
        if *expected == "error:*" {
            assert_eq!(
                question.expected_exit, 2,
                "{}: wildcard is for errors",
                question.id
            );
            continue;
        }
        assert!(
            actual.contains(expected),
            "{} {}: expected exact semantic line {expected:?}; got {displayed:?}",
            question.id,
            question.question
        );
    }
    let actual_conditions: Vec<_> = actual
        .into_iter()
        .filter(|line| condition_line(line))
        .collect();
    assert_eq!(
        actual_conditions, question.conditions,
        "{}: domain conditions changed unexpectedly: {displayed:?}",
        question.id
    );
    check_proof(question, displayed);
    match question.outcome {
        "solved" | "partial" => {
            assert_eq!(
                question.expected_exit, 0,
                "{}: success needs exit 0",
                question.id
            );
        }
        "unsupported" => assert!(
            question.expected_exit == 0 || question.expected_exit == 2,
            "{}: unsupported outcome exit is unexpected",
            question.id
        ),
        "domain" | "invalid" | "resource" => {
            assert_eq!(
                question.expected_exit, 2,
                "{}: refusal needs exit 2",
                question.id
            );
        }
        outcome => panic!("{}: unknown outcome {outcome}", question.id),
    }
}

#[test]
fn compiled_cli_answers_or_honestly_refuses_student_questions() {
    let corpus = questions();
    let mut counts = BTreeMap::<(&str, &str, &str), usize>::new();
    let mut failures = Vec::new();
    for question in &corpus {
        if std::panic::catch_unwind(|| check_question(question)).is_err() {
            failures.push(question.id);
        } else {
            *counts
                .entry((question.category, question.outcome, question.reason))
                .or_default() += 1;
        }
    }
    println!("compiled_cli_corpus_total={}", corpus.len());
    println!("category\toutcome\treason\tcount");
    for ((category, outcome, reason), count) in counts {
        println!("{category}\t{outcome}\t{reason}\t{count}");
    }
    assert!(failures.is_empty(), "failed question ids: {failures:?}");
}
