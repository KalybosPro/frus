//! Tests for `widgets::features::feedback::widgets::dialog`, moved out of `src/` (milestone 664).

use frus::StatefulWidget;
use widgets::features::feedback::widgets::dialog::*;

#[test]
fn cancel_keeps_the_old_name() {
    let mut state = DialogDemo.create_state();
    state.open();
    state.draft = "Road trip".into();
    state.cancel();
    assert_eq!(state.name, "My playlist");
    assert!(!state.open);
}

#[test]
fn save_trims_and_closes() {
    let mut state = DialogDemo.create_state();
    state.open();
    state.draft = "  Road trip ".into();
    state.save();
    assert_eq!(state.name, "Road trip");
    assert!(!state.open);
}
