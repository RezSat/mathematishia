use std::collections::{BTreeMap, BTreeSet};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use mathematishia_cli::{CliErrorKind, run};

const CORPUS: &str = include_str!("school_breadth.tsv");
const LEGACY: &str = include_str!("user_questions.tsv");
const TIME_LIMIT: Duration = Duration::from_secs(5);

type ProcessResult = (i32, String, String, u128);

fn execute(args: &[&str]) -> Result<ProcessResult, String> {
    let start = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            break;
        }
        if start.elapsed() >= TIME_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timeout".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    Ok((
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).map_err(|error| error.to_string())?,
        String::from_utf8(output.stderr).map_err(|error| error.to_string())?,
        start.elapsed().as_micros(),
    ))
}

fn matches_answer(output: &str, expected: &[&str]) -> bool {
    let prefixes: BTreeSet<_> = expected
        .iter()
        .filter_map(|line| line.split_once(':').map(|(key, _)| key))
        .collect();
    for prefix in prefixes {
        let actual: Vec<_> = output
            .lines()
            .filter(|line| line.split_once(':').is_some_and(|(key, _)| key == prefix))
            .collect();
        let wanted: Vec<_> = expected
            .iter()
            .copied()
            .filter(|line| line.split_once(':').is_some_and(|(key, _)| key == prefix))
            .collect();
        if actual != wanted {
            return false;
        }
    }
    output.lines().all(|line| {
        ![
            "condition:",
            "required:",
            "source condition:",
            "left condition:",
            "right condition:",
            "interval:",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
            || expected.contains(&line)
    })
}

// The steps endpoint verifies typed StepRecords against the original evidence
// before rendering. Require nonempty records and the separately keyed conclusion.
fn has_checked_records(output: &str) -> bool {
    let count = output
        .lines()
        .find_map(|line| line.strip_prefix("steps: "))
        .and_then(|n| n.parse::<usize>().ok());
    let records = output
        .lines()
        .filter(|line| {
            line.split_once(". ").is_some_and(|(index, claim)| {
                index.parse::<usize>().is_ok()
                    && claim.contains(": ")
                    && claim.contains("; premises: [")
                    && claim.contains("; conditions: ")
            })
        })
        .count();
    count == Some(records) && records > 0
}

fn classify(
    mode: &str,
    args: &[&str],
    expected: &[&str],
    code: i32,
    stdout: &str,
    stderr: &str,
) -> &'static str {
    if code == 2 && stdout.is_empty() && stderr.starts_with("error: ") {
        return match run(args) {
            Err(error) => match error.kind() {
                CliErrorKind::Unsupported | CliErrorKind::InvalidInput if mode == "gap" => {
                    "unsupported"
                }
                CliErrorKind::Resource => "resource_refusal",
                CliErrorKind::Domain => "domain_refusal",
                CliErrorKind::Verification => "verification_failure",
                _ => "unexpected_refusal",
            },
            Ok(_) => "adapter_mismatch",
        };
    }
    if code != 0 || !stderr.is_empty() {
        return "process_failure";
    }
    if mode == "gap" {
        return if stdout.lines().any(|line| {
            line == "status: partial" || (line.starts_with("expression: ") && line.contains("log("))
        }) {
            "partial_unchecked"
        } else {
            "gap_changed"
        };
    }
    if !matches_answer(stdout, expected) {
        return "wrong_answer";
    }
    if mode == "trace" {
        if has_checked_records(stdout) {
            "complete_checked_steps"
        } else {
            "missing_steps"
        }
    } else {
        "complete_answer"
    }
}

#[test]
fn school_breadth_compiled_cli() {
    let legacy: BTreeSet<_> = LEGACY
        .lines()
        .filter_map(|line| line.split('\t').nth(6))
        .collect();
    let mut ids = BTreeSet::new();
    let mut commands = BTreeSet::new();
    let mut counts = BTreeMap::<(&str, &str, &str), usize>::new();
    let mut timings = BTreeMap::<&str, Vec<u128>>::new();
    let mut failures = Vec::new();
    let mut runnable = 0;
    let mut bytes = 0;
    for line in CORPUS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("id\t"))
    {
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 7);
        let [id, category, _family, kind, mode, command, expected] =
            <[&str; 7]>::try_from(columns).unwrap();
        assert!(ids.insert(id));
        assert!(commands.insert(command));
        assert!(!legacy.contains(command));
        let args: Vec<_> = command.split('¦').collect();
        let expected: Vec<_> = expected.split('¦').collect();
        if mode != "gap" {
            runnable += 1;
        }
        let outcome = match execute(&args) {
            Err(error) => {
                failures.push(format!("{id}: {error}"));
                "process_failure"
            }
            Ok((code, stdout, stderr, micros)) => {
                timings.entry(category).or_default().push(micros);
                bytes += stdout.len() + stderr.len();
                let repeat = execute(&args);
                if !matches!(repeat, Ok((c, ref o, ref e, _)) if c == code && o == &stdout && e == &stderr)
                {
                    failures.push(format!("{id}: nondeterministic output"));
                    "nondeterministic"
                } else {
                    let outcome = classify(mode, &args, &expected, code, &stdout, &stderr);
                    if !matches!(
                        outcome,
                        "complete_answer"
                            | "complete_checked_steps"
                            | "unsupported"
                            | "partial_unchecked"
                    ) {
                        failures.push(format!(
                            "{id}: {outcome}; expected {expected:?}; got {stdout} {stderr}"
                        ));
                    }
                    outcome
                }
            }
        };
        *counts.entry((category, kind, outcome)).or_default() += 1;
    }
    println!(
        "school_breadth_cases={} runnable={runnable} output_bytes={bytes}",
        ids.len()
    );
    println!("category\tkind\toutcome\tcount");
    for ((category, kind, outcome), count) in counts {
        println!("{category}\t{kind}\t{outcome}\t{count}");
    }
    println!("category\tprocesses\tmedian_us\tp95_us\tmax_us");
    for (category, mut samples) in timings {
        samples.sort_unstable();
        let n = samples.len();
        println!(
            "{category}\t{n}\t{}\t{}\t{}",
            samples[n / 2],
            samples[(95 * n).div_ceil(100) - 1],
            samples[n - 1]
        );
    }
    assert!(runnable >= 250, "minimum executable breadth");
    assert!(failures.is_empty(), "breadth regressions: {failures:#?}");
}

#[test]
fn breadth_comparator_rejects_extra_roots_and_lost_conditions() {
    let expected = ["intervals: 1", "interval: [1, 2]"];
    assert!(matches_answer("intervals: 1\ninterval: [1, 2]", &expected));
    assert!(!matches_answer(
        "intervals: 1\ninterval: [1, 2]\ninterval: [3, 4]",
        &expected
    ));
    assert!(!matches_answer(
        "intervals: 1\ninterval: [1, 2]\ncondition: x != 0",
        &expected
    ));
    assert!(!matches_answer(
        "intervals: 1\ninterval: [1, 2]",
        &["intervals: 1", "interval: [1, 2]", "condition: x != 0"]
    ));
    assert!(!has_checked_records("steps: 1\nverified: true"));
}

#[test]
fn breadth_invalid_inputs_are_separate() {
    for args in [
        vec!["expr", "1..2"],
        vec!["matdet", "1,2;3"],
        vec!["steps", "linear", "x+5", "x"],
        vec!["limit", "x^2", "x", "0", "upward"],
    ] {
        let (code, stdout, stderr, _) = execute(&args).unwrap();
        assert_eq!(code, 2);
        assert!(stdout.is_empty());
        assert!(stderr.starts_with("error: "));
        assert!(run(&args).is_err());
    }
    println!("school_breadth_invalid_cases=4");
}
