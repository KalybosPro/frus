//! Tests for `widgets::features::inputs::widgets::date_picker`, moved out of `src/` (milestone 664).

use frus::StatefulWidget;
use widgets::features::inputs::widgets::date_picker::*;

#[test]
fn months_roll_over_years() {
    assert_eq!(shift_month(2026, 12, 1), (2027, 1));
    assert_eq!(shift_month(2026, 1, -1), (2025, 12));
    assert_eq!(shift_month(2026, 5, 0), (2026, 5));
}

#[test]
fn weekdays_are_counted_from_sunday() {
    // 15 October 2026 is a Thursday.
    assert_eq!(weekday((2026, 10, 15)), 4);
    assert_eq!(weekday((2026, 10, 18)), 0);
}

#[test]
fn a_second_click_before_the_start_starts_again() {
    let mut state = DatePickerDemo.create_state();
    state.range = (Some((2026, 10, 12)), None);
    state.pick_range_day((2026, 10, 5));
    assert_eq!(state.range, (Some((2026, 10, 5)), None));
    state.pick_range_day((2026, 10, 9));
    assert_eq!(state.range, (Some((2026, 10, 5)), Some((2026, 10, 9))));
}
