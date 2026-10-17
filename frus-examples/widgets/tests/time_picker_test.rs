//! Tests for `widgets::features::inputs::widgets::time_picker`, moved out of `src/` (milestone 664).

use frus::{Endpoint, StatefulWidget, TimeField};
use widgets::features::inputs::widgets::time_picker::*;

#[test]
fn a_range_change_touches_one_field_of_one_end() {
    let mut state = TimePickerDemo.create_state();
    state.change_shift(Endpoint::End, TimeField::Hour, 18);
    state.change_shift(Endpoint::Start, TimeField::Minute, 15);
    assert_eq!(state.shift, ((9, 15), (18, 30)));
}
