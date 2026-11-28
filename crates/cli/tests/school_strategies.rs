use std::process::Command;

use mathematishia_cli::run;

#[test]
fn compiled_school_strategies_are_deterministic_and_checked() {
    for (args, method, status) in [
        (
            &["school", "solve", "x^4-5*x^2+4=0", "x"][..],
            "polynomial.power_substitution",
            "complete",
        ),
        (
            &["school", "differentiate", "(x^2-1)/(x-1)", "x"],
            "calculus.rational_quotient_rule",
            "conditional",
        ),
        (
            &["school", "integrate", "x*sin(x)", "x"],
            "calculus.polynomial_trigonometric_parts",
            "complete",
        ),
        (
            &["school", "integrate", "2*x*cos(x^2)", "x"],
            "calculus.polynomial_substitution",
            "complete",
        ),
        (
            &["school", "limit", "sin(3*x)/x", "x", "0"],
            "calculus.standard_limit",
            "complete",
        ),
        (
            &["school", "limit", "x^3/(x^2+1)", "x", "-infinity"],
            "calculus.rational_infinity_limit",
            "complete",
        ),
        (
            &["school", "sqrt", "2"],
            "radical.principal_real_square_root",
            "complete",
        ),
        (
            &["school", "periodic", "sin", "2", "0", "1/2", "x"],
            "trig.periodic_families",
            "complete",
        ),
        (
            &["school", "abs_sum", "abs(x-1)+abs(x+1)=4", "x"],
            "piecewise_affine_partition",
            "complete",
        ),
        (
            &["school", "inequality", "(x-1)*(x+2)*(x-3)>=0", "x"],
            "inequality.factored_sign_chart",
            "complete",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_mathematishia"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {:?}", output.stderr);
        let expected = run(args).unwrap();
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim_end(),
            expected
        );
        assert!(
            expected.contains(&format!("\"method\":\"{method}\"")),
            "{args:?}: {expected}"
        );
        assert!(
            expected.contains(&format!("\"status\":\"{status}\"")),
            "{args:?}: {expected}"
        );
        assert_eq!(expected, run(args).unwrap());
    }
}
