use mathematishia_cli::{CliErrorKind, run};

#[test]
fn exact_algebra_frontend() {
    let basis = run(&["groebner", "x,y", "x*y-1", "y^2-1"]).unwrap();
    assert!(basis.contains("basis_count: "));
    assert!(basis.contains("verified: true"));
    let yes = run(&["idealmember", "x,y", "x-y", "x*y-1", "y^2-1"]).unwrap();
    assert!(yes.contains("member: true\nremainder: 0"));
    let no = run(&["idealmember", "x,y", "x*y", "x^2", "y^2"]).unwrap();
    assert!(no.contains("member: false"));
    assert_eq!(
        run(&["groebner", "x,y", "x+0/y"]).unwrap_err().kind(),
        CliErrorKind::Domain
    );
    let rational = run(&["rootsfield", "x^2-2", "x", "Q"]).unwrap();
    assert!(rational.contains(r#""status":"no_solutions""#));
    let real = run(&["rootsfield", "x^2-2", "x", "R"]).unwrap();
    assert!(real.contains(r#""result_domain":"R""#));
    assert!(real.contains(r#""status":"complete""#));
    assert!(real.contains("source_bound_polynomial"));
    let partial = run(&["rootsfield", "x^3-2", "x", "C"]).unwrap();
    assert!(partial.contains(r#""status":"partial""#));
    assert!(run(&["rootsfield", "x+0/x", "x", "R"]).is_err());
}

#[test]
fn exact_angles_and_absolute_intervals() {
    let sine = run(&["trig", "sin", "1/6"]).unwrap();
    assert!(sine.contains("value: 1/2"));
    assert!(run(&["trig", "tan", "1/2"]).unwrap().contains("status: undefined"));
    assert!(run(&["trig", "sin", "1/7"]).is_err());
    let zeros = run(&["trigzeros", "cos"]).unwrap();
    assert!(zeros.contains("status: partial"));
    assert!(zeros.contains("known_pi_coefficient: 3/2"));
    let intervals = run(&["abssolve", "abs(abs(x)-2)=1"]).unwrap();
    for root in [-3, -1, 1, 3] {
        assert!(intervals.contains(&format!("interval: [{root}, {root}]")));
    }
    assert!(run(&["abssolve", "abs(x)=x"]).unwrap().contains("interval: [0, infinity)"));
    assert!(run(&["abssolve", "abs(x+0/x)=3"]).is_err());
    assert!(run(&["abssolve", "abs(x^2)=3"]).is_err());
}

#[test]
fn calculus_and_step_records() {
    let result = run(&["checkcalculus", "identity", "(x^2-1)/(x-1)", "x+1", "x"]).unwrap();
    assert!(result.contains("condition: "));
    assert!(run(&["checkcalculus", "identity", "x+1", "(x^2-1)/(x-1)", "x"]).is_err());
    assert!(run(&["checkcalculus", "derivative", "x^2", "3*x", "x"]).is_err());
    assert!(run(&["checkcalculus", "derivative", "x^2", "2*x", "x"]).unwrap().contains("verified: true"));
    for args in [
        &["steps", "linear", "2*x+4=10", "x"][..],
        &["steps", "derivative", "sin(x^2)", "x"],
        &["steps", "expand", "2*(x+1)", "x"],
    ] {
        let result = run(args).unwrap();
        assert!(result.contains("verified: true"), "{result}");
        assert!(result.contains("steps: "), "{result}");
    }
}
