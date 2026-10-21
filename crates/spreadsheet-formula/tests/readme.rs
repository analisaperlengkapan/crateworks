//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};

    struct Grid;
    impl Sheet for Grid {
        fn value(&self, cell: CellRef) -> Value {
            match (cell.column, cell.row) {
                (0, 0) => Value::Number(2.0),
                (0, 1) => Value::Number(3.0),
                _ => Value::Empty,
            }
        }
    }

    assert_eq!(
        Evaluator::new(&Grid).eval("=SUM(A1:A2)*2").unwrap(),
        Value::Number(10.0)
    );
}
