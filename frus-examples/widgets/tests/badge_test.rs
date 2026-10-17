//! Tests for `widgets::features::display::widgets::badge`, moved out of `src/` (milestone 662).

use widgets::features::display::widgets::badge::count_label;

#[test]
fn large_counts_are_capped() {
    assert_eq!(count_label(7), "7");
    assert_eq!(count_label(99), "99");
    assert_eq!(count_label(100), "99+");
}
