use mathematishia_cli::{CliErrorKind, run};

#[test]
fn checked_results_are_versioned_and_deterministic() {
    for args in [
        &["solveauto", "x=1", "--json"][..],
        &["expandall", "(x+1)^2", "--json"],
        &["integrate", "x^2+sin(2*x)", "x", "--json"],
        &["limit", "(x^2-1)/(x-1)", "x", "1", "--json"],
        &["inequality", "x^2<2", "--json"],
        &["evalexact", "sin(0)+cos(0)", "--json"],
    ] {
        let output = run(args).unwrap();
        assert!(output.starts_with(r#"{"schema_version":1,"request":"#));
        assert!(output.contains(r#""status":"complete""#), "{output}");
        assert_eq!(output, run(args).unwrap());
    }
    let partial = run(&["solveauto", "x^3-2=0", "--json"]).unwrap();
    assert!(partial.contains(r#""status":"partial""#));
    let refused = run(&["solveauto", "sin(x)=0", "--json"]).unwrap();
    assert!(refused.contains(r#""status":"unsupported""#));
    assert!(refused.ends_with(r#""evidence":null}"#));
    let hole = run(&["limit", "(x^2-1)/(x-1)", "x", "1", "--json"]).unwrap();
    assert!(hole.contains(r#""kind":"polynomial_nonzero""#));
}

#[test]
fn isolate_all_roots_or_refuse_without_a_false_certificate() {
    let result = run(&["isolate", "x^2-2", "x", "1/32"]).unwrap();
    assert!(result.contains("roots: 2\n"));
    assert_eq!(result.matches("interval: ").count(), 2);
    assert!(result.contains("complete: true\nconditions: none\nverified: true"));
    assert!(
        run(&["isolate", "x^2+1", "x", "1/32"])
            .unwrap()
            .contains("roots: 0\n")
    );
    for input in ["(x-1)^2", "0", "x+0/x"] {
        assert_eq!(
            run(&["isolate", input, "x", "1/32"]).unwrap_err().kind(),
            CliErrorKind::Unsupported
        );
    }
    assert_eq!(
        run(&["isolate", "x^2-2", "x", "0"]).unwrap_err().kind(),
        CliErrorKind::InvalidInput
    );
}

#[test]
fn checked_school_math_operations_are_available_through_shared_requests() {
    for args in [
        &["natural", "sin(pi/6)", "--json"][..],
        &["natural", "|2x-5|=7", "--json"],
        &["abssolve", "abs(x)=3", "--json"],
        &["trig", "sin", "1/6", "--json"],
        &["solvefield", "(x^2-1)/(x-1)=3", "x", "R", "--json"],
        &["solvefield", "x^2=2", "x", "R", "--json"],
        &["integrate", "x*exp(x)", "--json"],
        &["groebner", "x,y", "x^2-y", "y^2-1", "--json"],
        &["idealmember", "x", "x^2", "x", "--json"],
    ] {
        let output = run(args).unwrap();
        assert!(
            output.contains(r#""status":"complete""#),
            "{args:?}: {output}"
        );
        assert_eq!(output, run(args).unwrap());
    }
    let excluded = run(&["solvefield", "(x^2-1)/(x-1)=2", "x", "R", "--json"]).unwrap();
    assert!(excluded.contains(r#""status":"no_solutions""#));
    assert!(excluded.contains(r#""kind":"polynomial_nonzero""#));
    let identity = run(&["solvefield", "0/x=0", "x", "R", "--json"]).unwrap();
    assert!(identity.contains(r#""status":"all_admissible""#));
    let pole = run(&["natural", "tan(pi/2)", "--json"]).unwrap();
    assert!(pole.contains(r#""status":"undefined""#));
    let zeros = run(&["trigzeros", "sin", "--json"]).unwrap();
    assert!(zeros.contains(r#""status":"partial""#));
    assert_eq!(
        run(&["natural", "sqrt(16)", "--json"]).unwrap_err().kind(),
        CliErrorKind::Unsupported
    );
    assert_eq!(
        run(&["natural", "sqrt(-1)", "--json"]).unwrap_err().kind(),
        CliErrorKind::Domain
    );
}
