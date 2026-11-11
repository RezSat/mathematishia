use mathema::{Integer, Rational};
use mathematishia_kernel::{
    EvaluationContext, ExactMatrix, LinearSystemSolution, MatrixError,
    verify_linear_system_vector, verify_matrix_inverse,
};

fn q(value: i64) -> Rational { Rational::from(Integer::from(value)) }
fn m(rows: &[&[i64]]) -> ExactMatrix {
    ExactMatrix::new(&EvaluationContext::default(), rows.iter().map(
        |row| row.iter().copied().map(q).collect()
    ).collect()).unwrap()
}

#[test]
fn exact_determinants_inverse_and_singular_detection() {
    let ctx=EvaluationContext::default();
    let a=m(&[&[2,1],&[5,3]]);
    assert_eq!(a.determinant(&ctx).unwrap(),q(1));
    let inv=a.inverse(&ctx).unwrap().unwrap();
    assert_eq!(inv, m(&[&[3,-1],&[-5,2]]));
    assert!(verify_matrix_inverse(&ctx,&a,&inv).unwrap());
    let forged=m(&[&[3,-1],&[-5,3]]);
    assert!(!verify_matrix_inverse(&ctx,&a,&forged).unwrap());
    let singular=m(&[&[1,2],&[2,4]]);
    assert_eq!(singular.determinant(&ctx).unwrap(),q(0));
    assert_eq!(singular.inverse(&ctx).unwrap(),None);
}

#[test]
fn rank_and_rref_are_exact_for_rectangular_matrices() {
    let ctx=EvaluationContext::default();
    let a=m(&[&[1,2,3],&[2,4,6],&[1,1,1]]);
    let reduced=a.rref(&ctx).unwrap();
    assert_eq!(reduced.rank(),2);
    assert_eq!(reduced.pivot_columns(),&[0,1]);
    assert_eq!(reduced.matrix(),&m(&[&[1,0,-1],&[0,1,2],&[0,0,0]]));
    assert_eq!(a.rank(&ctx).unwrap(),2);
    assert_eq!(a.transpose(&ctx).unwrap().row_count(),3);
}

#[test]
fn unique_inconsistent_and_affine_solutions_are_distinct() {
    let ctx=EvaluationContext::default();
    let a=m(&[&[2,1],&[5,3]]);
    let rhs=[q(5),q(13)];
    let LinearSystemSolution::Unique(value)=a.solve(&ctx,&rhs).unwrap()
        else {panic!("expected unique solution")};
    assert_eq!(value,vec![q(2),q(1)]);
    assert!(verify_linear_system_vector(&ctx,&a,&rhs,&value).unwrap());

    let dependent=m(&[&[1,1,1],&[2,2,2]]);
    assert_eq!(dependent.solve(&ctx,&[q(3),q(7)]).unwrap(),
        LinearSystemSolution::Inconsistent);
    let LinearSystemSolution::Infinite{particular,nullspace_basis}=
        dependent.solve(&ctx,&[q(3),q(6)]).unwrap()
        else {panic!("expected affine family")};
    assert_eq!(particular,vec![q(3),q(0),q(0)]);
    assert_eq!(nullspace_basis.len(),2);
    assert!(verify_linear_system_vector(&ctx,&dependent,&[q(3),q(6)],&particular).unwrap());
    for basis in nullspace_basis {
        assert!(verify_linear_system_vector(&ctx,&dependent,&[q(0),q(0)],&basis).unwrap());
    }
}

#[test]
fn dimensions_and_invalid_inputs_are_checked() {
    let ctx=EvaluationContext::default();
    assert_eq!(ExactMatrix::new(&ctx,vec![]),Err(MatrixError::Empty));
    assert_eq!(ExactMatrix::new(&ctx,vec![vec![q(1)],vec![q(1),q(2)]]),
        Err(MatrixError::NonRectangular));
    assert_eq!(ExactMatrix::identity(&ctx,17),Err(MatrixError::DimensionLimit));
    assert_eq!(m(&[&[1,2]]).determinant(&ctx),Err(MatrixError::NotSquare));
    assert_eq!(m(&[&[1,2]]).multiply(&m(&[&[1,2]]),&ctx),
        Err(MatrixError::DimensionMismatch));
    assert_eq!(m(&[&[1]]).solve(&ctx,&[]),Err(MatrixError::DimensionMismatch));
}

#[test]
fn determinant_and_inverse_match_exhaustive_small_integer_grid() {
    let ctx=EvaluationContext::default();
    for a in -2..=2 {
        for b in -2..=2 {
            for c in -2..=2 {
                for d in -2..=2 {
                    let matrix=m(&[&[a,b],&[c,d]]);
                    assert_eq!(matrix.determinant(&ctx).unwrap(),q(a*d-b*c));
                    let inv=matrix.inverse(&ctx).unwrap();
                    assert_eq!(inv.is_some(),a*d-b*c!=0);
                    if let Some(inv)=inv {
                        assert!(verify_matrix_inverse(&ctx,&matrix,&inv).unwrap());
                    }
                }
            }
        }
    }
}
