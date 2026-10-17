//! Tests for `widgets::features::scrolling::widgets::kanban`, moved out of `src/` (milestone 662).

use widgets::features::scrolling::widgets::kanban::move_card;

fn board() -> Vec<(String, Vec<String>)> {
    vec![
        ("A".into(), vec!["1".into(), "2".into()]),
        ("B".into(), vec!["3".into()]),
    ]
}

#[test]
fn moves_a_card_to_another_column() {
    let mut columns = board();
    move_card(&mut columns, (0, 0), (1, 1));
    assert_eq!(columns[0].1, ["2"]);
    assert_eq!(columns[1].1, ["3", "1"]);
}

#[test]
fn a_bad_source_changes_nothing() {
    let mut columns = board();
    move_card(&mut columns, (0, 9), (1, 0));
    assert_eq!(columns, board());
}
