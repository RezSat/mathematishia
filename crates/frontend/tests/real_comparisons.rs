use mathema::Condition;
use mathema_frontend::{RealComparisonOperator, parse_real_comparison, parse_statement};

#[test]
fn single_real_comparisons_preserve_both_operand_histories() {
    let comparison = parse_real_comparison("0/(x-1) ≤ 0/(x+2)").unwrap();
    assert_eq!(comparison.operator, RealComparisonOperator::LessOrEqual);
    assert_eq!(comparison.left.expression().to_string(), "0");
    assert_eq!(comparison.right.expression().to_string(), "0");
    for operand in [&comparison.left, &comparison.right] {
        assert!(matches!(operand.conditions(), [Condition::NonZero(_)]));
    }
    assert_eq!(
        parse_real_comparison("2x>0.5").unwrap().operator,
        RealComparisonOperator::Greater
    );
    assert_eq!(
        parse_real_comparison("x>=1").unwrap().operator,
        RealComparisonOperator::GreaterOrEqual
    );
    assert_eq!(
        parse_real_comparison("x<1").unwrap().operator,
        RealComparisonOperator::Less
    );
    assert!(parse_statement("x<1").is_err());
}

#[test]
fn malformed_or_chained_comparisons_are_rejected() {
    for input in ["x=1", "x", "x<", "<x", "0<x<1", "(x<1)>0", "x< =1", "x<>1"] {
        assert!(parse_real_comparison(input).is_err(), "{input}");
    }
    assert!(parse_real_comparison(&"x".repeat(32_769)).is_err());
}
