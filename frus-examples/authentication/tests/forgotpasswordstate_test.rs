//! Tests for `authentication::features::auth::states::forgotpasswordstate`, moved out of `src/` (milestone 662).

use authentication::features::auth::states::forgotpasswordstate::*;

#[test]
fn an_answer_shows_where_the_link_went() {
    let mut state = ForgotPasswordState::default();
    state.set_email(" ada@frus.dev ".to_string());
    assert!(state.form().is_valid());
    assert!(state.submit.start(true));
    state.finish(Ok(()));
    assert_eq!(state.sent_to.as_deref(), Some("ada@frus.dev"));
    assert!(!state.submit.loading);

    state.try_again();
    assert_eq!(state.sent_to, None);
    assert!(!state.submit.submitted, "a fresh form shows no errors");
}

#[test]
fn a_malformed_address_is_refused() {
    let mut state = ForgotPasswordState::default();
    state.set_email("ada".to_string());
    assert_eq!(
        state.form().error("email"),
        Some("Enter a valid email address")
    );
}
