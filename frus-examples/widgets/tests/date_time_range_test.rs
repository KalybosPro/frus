//! Tests for `widgets::features::inputs::widgets::date_time_range`, moved out of `src/` (milestone 662).

use frus::StatefulWidget;
use widgets::features::inputs::widgets::date_time_range::*;

#[test]
fn two_clicks_make_a_range_and_a_third_starts_again() {
    let mut state = BookingDemo.create_state();
    state.start = None;
    state.end = None;
    state.pick((2026, 10, 30));
    state.pick((2026, 11, 2));
    assert_eq!(
        (state.start, state.end),
        (Some((2026, 10, 30)), Some((2026, 11, 2)))
    );
    state.pick((2026, 10, 1));
    assert_eq!((state.start, state.end), (Some((2026, 10, 1)), None));
}
