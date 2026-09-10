use crate::{
    engine::{self, Format},
    math,
};
fn value(s: &str) -> String {
    let tape = engine::calculate(s, &Format::default());
    tape.lines
        .last()
        .unwrap()
        .result
        .normalized()
        .to_plain_string()
}
#[test]
fn tape_runs_top_to_bottom() {
    assert_eq!(value("+10\n+2\n*3\n---\n+0"), "36");
}

#[test]
fn welcome_example_teaches_both_calculation_orders() {
    let tape = engine::calculate(crate::INTRO, &Format::default());
    assert!(tape.lines.iter().all(|line| line.error.is_none()));
    assert!(
        tape.lines
            .iter()
            .any(|line| line.kind == engine::Kind::Total && line.result == 36)
    );
    assert_eq!(
        tape.variables.get("x").map(|(_, value)| value),
        Some(&math::Number::from(16))
    );
}
#[test]
fn subtotal_continuation() {
    assert_eq!(value("10\n+2\n---\n+0\n*3\n---\n+0"), "36");
}
#[test]
fn independent_blocks_and_grand() {
    let t = engine::calculate("10\n+2\n---\n+0\n\n100\n-20\n---\n+0", &Format::default());
    assert_eq!(t.grand.normalized().to_string(), "92");
}
#[test]
fn powers_left_associative() {
    assert_eq!(value("2\n^3\n^2\n---\n+0"), "64");
}
#[test]
fn decimal_accuracy() {
    assert_eq!(value("0.1\n+0.2\n---\n+0"), "0.3");
}
#[test]
fn precision_retained() {
    let t = engine::calculate("1\n/3\n*3\n---\n+0", &Format::default());
    assert_eq!(engine::format(&t.grand, &Format::default()), "1.00");
}
#[test]
fn percent_add() {
    assert_eq!(value("1000\n+19%\n---\n+0"), "1190");
}
#[test]
fn percent_deduct() {
    assert_eq!(value("910.50\n-40%\n---\n+0"), "546.3");
}
#[test]
fn chained_percents() {
    assert_eq!(value("100\n+10%\n+10%\n---\n+0"), "121");
}
#[test]
fn percent_factors() {
    assert_eq!(value("100\n*10%\n---\n+0"), "10");
    assert_eq!(value("100\n/10%\n---\n+0"), "1000");
}
#[test]
fn percent_then_multiply_runs_sequentially() {
    let t = engine::calculate("100\n+10%\n*2\n---\n+0", &Format::default());
    assert!(t.lines.iter().all(|l| l.error.is_none()));
    assert_eq!(t.grand.normalized().to_string(), "220");
}
#[test]
fn percent_subtotal_fixes_error() {
    assert_eq!(value("100\n+10%\n---\n+0\n*2\n---\n+0"), "220");
}
#[test]
fn variables_case_insensitive() {
    assert_eq!(value("Tax = 15\n100\n+tax%\n---\n+0"), "115");
}
#[test]
fn assignment_expression() {
    assert_eq!(value("rate = (10+2)*3\n+rate\n+1\n---\n+0"), "37");
}
#[test]
fn assignment_functions() {
    for (expr, expected) in [
        ("sqrt(9)", "3"),
        ("abs(-5)", "5"),
        ("sin(0)", "0"),
        ("cos(0)", "1"),
        ("log(100)", "2"),
    ] {
        let t = engine::calculate(&format!("a={expr}\n+a\n+0\n---\n+0"), &Format::default());
        assert_eq!(
            engine::format(&t.grand, &Format::default()),
            format!("{expected}.00")
        );
    }
}
#[test]
fn sum_variable() {
    assert_eq!(
        value("100\n+20\n---\n+0 = budget\n\n+BUDGET\n-25%\n---\n+0"),
        "90"
    );
}
#[test]
fn duplicate_variable() {
    let t = engine::calculate("Tax=15\ntax=20", &Format::default());
    assert!(
        t.lines[1]
            .error
            .as_ref()
            .unwrap()
            .contains("already in use")
    );
}
#[test]
fn unknown_and_invalid_identifiers() {
    assert!(
        engine::calculate("+unknown", &Format::default()).lines[0]
            .error
            .is_some()
    );
    assert!(
        engine::calculate("tax_rate=20", &Format::default()).lines[0]
            .error
            .is_some()
    );
}
#[test]
fn standalone_comments_break_math() {
    assert_eq!(value("10\nA note\n+20"), "20");
}
#[test]
fn inline_comments_keep_math() {
    assert_eq!(value("10 apples\n+20 pears\n---\n+0"), "30");
}
#[test]
fn pasted_expression_is_comment() {
    assert_eq!(value("+10+2*3\n+1\n---\n+0"), "11");
}
#[test]
fn divide_by_zero() {
    let t = engine::calculate("10\n/0\n---\n+0", &Format::default());
    assert_eq!(t.lines[1].error.as_deref(), Some("Division by zero"));
    assert!(t.lines[3].error.is_some());
}
#[test]
fn invalid_leading_operator() {
    assert!(
        engine::calculate("*3", &Format::default()).lines[0]
            .error
            .is_some()
    );
}
#[test]
fn decimal_comma() {
    let t = engine::calculate(
        "1.234,50\n+5,50\n---\n+0",
        &Format {
            comma: true,
            ..Format::default()
        },
    );
    assert_eq!(t.grand.normalized().to_string(), "1240");
}
#[test]
fn grouping_is_always_in_threes() {
    let f: Format = serde_json::from_str(r#"{"comma":false,"indian":true,"digits":2}"#).unwrap();
    assert_eq!(engine::format(&math::dec("1000000"), &f), "1,000,000.00");
    assert_eq!(engine::format(&math::dec("1234567.89"), &f), "1,234,567.89");
    assert!(!serde_json::to_string(&f).unwrap().contains("indian"));
}
#[test]
fn thirty_two_integer_digits() {
    assert!(engine::number("12345678901234567890123456789012", &Format::default()).is_ok());
    assert!(engine::number("123456789012345678901234567890123", &Format::default()).is_err());
}
#[test]
fn negative_operand() {
    assert_eq!(value("10\n*-2\n---\n+0"), "-20");
}
#[test]
fn root() {
    assert_eq!(value("9\n^0.5\n---\n+0"), "3");
}
#[test]
fn running_result_tracks_whole_block() {
    let t = engine::calculate("100\n+20\n---\n+0", &Format::default());
    assert_eq!(t.lines[0].result.normalized().to_string(), "120");
    assert_eq!(
        t.lines[0].value.as_ref().unwrap().normalized().to_string(),
        "100"
    );
}

#[test]
fn scientific_functions_use_radians_and_explicit_rounding() {
    let f = Format::default();
    let t = engine::calculate("a=sin(90)\n+a\n+0\n---\n+0", &f);
    assert_eq!(engine::format(&t.grand, &f), "0.89");
    assert_eq!(value("a=round(1.2345)\n+a\n*10000\n---\n+0"), "12300");
}
#[test]
fn assignments_keep_standard_precedence() {
    assert_eq!(value("x=10+2*3\n+x\n+0\n---\n+0"), "16");
    assert_eq!(value("x=(10+2)*3\n+x\n+0\n---\n+0"), "36");
}
