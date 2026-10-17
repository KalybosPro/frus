//! Tests for `authentication::features::profile::states::profilestate`, moved out of `src/` (milestone 662).

use authentication::features::profile::states::profilestate::*;

#[test]
fn saving_the_same_name_is_refused() {
    let state = ProfileState::new("Demo User".to_string());
    assert_eq!(
        state.form("Demo User").error("name"),
        Some("This is already your name")
    );
}

#[test]
fn a_new_name_is_valid_and_typing_hides_the_saved_note() {
    let mut state = ProfileState::new("Demo User".to_string());
    state.set_name("Grace".to_string());
    assert!(state.form("Demo User").is_valid());

    assert!(state.submit.start(true));
    state.finish(Ok(()));
    assert!(state.saved);
    state.set_name("Grace H.".to_string());
    assert!(!state.saved);
}
