//! Tests for `widgets::features::feedback::widgets::snack_bar`, moved out of `src/` (milestone 662).

use frus::{SnackBarKind, StatefulWidget};
use widgets::features::feedback::widgets::snack_bar::*;

#[test]
fn only_the_first_message_starts_a_timer() {
    let mut state = SnackBarDemo.create_state();
    assert_eq!(state.push("one", SnackBarKind::Info, false), Some(0));
    assert_eq!(state.push("two", SnackBarKind::Info, false), None);
    assert_eq!(state.current_id(), Some(0));
    // Closing the first puts the second on screen, and hands back its id.
    assert_eq!(state.dismiss(), Some(1));
    assert_eq!(state.dismiss(), None);
}
