use mathematishia_cli::run;

#[test]
fn exact_matrix_commands_produce_rational_results() {
    assert_eq!(run(&["matdet", "2,1;5,3"]).unwrap(), "determinant: 1");
    assert_eq!(run(&["matinv", "2,1;5,3"]).unwrap(), "inverse: 3,-1;-5,2");
    assert_eq!(run(&["matmul", "2,1;5,3", "3,-1;-5,2"]).unwrap(),
        "product: 1,0;0,1");
    assert_eq!(run(&["matsolve", "2,1;5,3", "5,13"]).unwrap(),
        "solution: unique\nvalue: 2,1");
}

#[test]
fn matrix_commands_reject_invalid_shapes_and_handle_affine_solutions() {
    let affine=run(&["matsolve","1,1,1;2,2,2","3,6"]).unwrap();
    assert!(affine.contains("solution: infinite"));
    assert!(affine.contains("nullspace:"));
    assert_eq!(run(&["matsolve","1,1;2,2","3,7"]).unwrap(),
        "solution: inconsistent");
    assert_eq!(run(&["matinv","1,2;2,4"]).unwrap(),"inverse: none (singular)");
    assert!(run(&["matdet","1,2;3"]).is_err());
    assert!(run(&["matdet",""]).is_err());
    assert!(run(&["matmul","1,2","1,2"]).is_err());
}
