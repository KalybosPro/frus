//! What the login page keeps between rebuilds, and its rules.

use frus::form::{Form, Rule};

use crate::common::submission::Submission;
use crate::features::auth::controllers::user_store::{DEMO_EMAIL, DEMO_PASSWORD};

#[derive(Default)]
pub(crate) struct LoginState {
    pub email: String,
    pub password: String,
    /// The password is shown in clear.
    pub visibility: bool,
    pub submit: Submission,
}

impl LoginState {
    pub(crate) fn set_email(&mut self, value: String) {
        self.email = value;
        self.submit.edited();
    }

    pub(crate) fn set_password(&mut self, value: String) {
        self.password = value;
        self.submit.edited();
    }

    pub(crate) fn toggle_visibility(&mut self) {
        self.visibility = !self.visibility;
    }

    /// Fills the form with the demo account's credentials.
    pub(crate) fn use_demo_account(&mut self) {
        self.set_email(DEMO_EMAIL.to_string());
        self.set_password(DEMO_PASSWORD.to_string());
    }

    /// The form's validation, a pure function of the state: asked again on every build.
    pub(crate) fn form(&self) -> Form {
        Form::new()
            .field(
                "email",
                &self.email,
                Rule::all([
                    Rule::required("Email is required"),
                    Rule::email("Enter a valid email address"),
                ]),
            )
            .field(
                "password",
                &self.password,
                Rule::all([
                    Rule::required("Password is required"),
                    Rule::min_len(8, "At least 8 characters"),
                ]),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
