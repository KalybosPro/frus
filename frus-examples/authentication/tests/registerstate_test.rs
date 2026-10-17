//! Tests for `authentication::features::auth::states::registerstate`, moved out of `src/` (milestone 662).

use authentication::features::auth::states::registerstate::*;

fn filled() -> RegisterState {
    RegisterState {
        name: "Ada".to_string(),
        email: "ada@frus.dev".to_string(),
        password: "analytical1".to_string(),
        confirm: "analytical1".to_string(),
        ..RegisterState::default()
    }
}

#[test]
fn a_complete_form_is_valid() {
    assert!(filled().form().is_valid());
}

#[test]
fn the_confirmation_must_match() {
    let mut state = filled();
    state.set_confirm("analytical2".to_string());
    assert_eq!(state.form().error("confirm"), Some("The passwords differ"));
}

#[test]
fn a_password_needs_a_digit() {
    let mut state = filled();
    state.set_password("analytical".to_string());
    assert_eq!(
        state.form().error("password"),
        Some("Add at least one digit")
    );
}

#[test]
fn a_blank_name_is_refused() {
    let mut state = filled();
    state.set_name("   ".to_string());
    assert_eq!(state.form().error("name"), Some("Your name is required"));
}
