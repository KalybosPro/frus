//! Tests for `authentication::features::auth::states::loginstate`, moved out of `src/` (milestone 664).

use authentication::features::auth::states::loginstate::*;

#[test]
fn an_empty_form_says_what_is_missing() {
    let state = LoginState::default();
    let form = state.form();
    assert_eq!(form.error("email"), Some("Email is required"));
    assert_eq!(form.error("password"), Some("Password is required"));
}

#[test]
fn a_malformed_email_and_a_short_password_are_refused() {
    let mut state = LoginState::default();
    state.set_email("demo@frus".to_string());
    state.set_password("short".to_string());
    let form = state.form();
    assert_eq!(form.error("email"), Some("Enter a valid email address"));
    assert_eq!(form.error("password"), Some("At least 8 characters"));
    assert_eq!(form.first_invalid(), Some("email"));
}

#[test]
fn the_demo_account_passes_validation() {
    let mut state = LoginState::default();
    state.use_demo_account();
    assert!(state.form().is_valid());
}
