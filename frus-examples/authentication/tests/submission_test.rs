//! Tests for `authentication::common::submission`, moved out of `src/` (milestone 664).

use frus::form::Form;
use frus::form::Rule;

use authentication::common::submission::*;
use authentication::features::auth::controllers::user_store::AuthError;

#[test]
fn errors_wait_for_the_first_submit() {
    let form = Form::new().field("email", "", Rule::required("Email is required"));
    let mut submission = Submission::default();
    assert_eq!(submission.error(&form, "email"), None);

    assert!(
        !submission.start(form.is_valid()),
        "an invalid form is not sent"
    );
    assert_eq!(
        submission.error(&form, "email").as_deref(),
        Some("Email is required")
    );
}

#[test]
fn one_request_at_a_time() {
    let mut submission = Submission::default();
    assert!(submission.start(true));
    assert!(submission.loading);
    assert!(
        !submission.start(true),
        "a second press while loading is ignored"
    );
}

#[test]
fn a_refusal_is_kept_until_the_person_types() {
    let mut submission = Submission::default();
    submission.start(true);
    submission.finish(Err(AuthError::WrongPassword));
    assert!(!submission.loading);
    assert_eq!(
        submission.server_error,
        Some(AuthError::WrongPassword.to_string())
    );
    submission.edited();
    assert_eq!(submission.server_error, None);
}
