//! Tests for `widgets::features::scrolling::widgets::reorderable`, moved out of `src/` (milestone 664).

use widgets::features::scrolling::widgets::reorderable::move_item;

#[test]
fn moves_down_and_up() {
    let mut list = vec!['a', 'b', 'c', 'd'];
    move_item(&mut list, 0, 2);
    assert_eq!(list, ['b', 'c', 'a', 'd']);
    move_item(&mut list, 3, 0);
    assert_eq!(list, ['d', 'b', 'c', 'a']);
}
