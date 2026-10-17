//! What the login page keeps between rebuilds, and its rules.

use frus::form::{Form, Rule};

use crate::common::submission::Submission;
use crate::features::auth::controllers::user_store::{DEMO_EMAIL, DEMO_PASSWORD};

#[derive(Default)]
pub struct LoginState {
    pub email: String,
    pub password: String,
    /// The password is shown in clear.
    pub visibility: bool,
    pub submit: Submission,
}

impl LoginState {
    pub fn set_email(&mut self, value: String) {
        self.email = value;
        self.submit.edited();
    }

    pub fn set_password(&mut self, value: String) {
        self.password = value;
        self.submit.edited();
    }

    pub fn toggle_visibility(&mut self) {
        self.visibility = !self.visibility;
    }

    /// Fills the form with the demo account's credentials.
    pub fn use_demo_account(&mut self) {
        self.set_email(DEMO_EMAIL.to_string());
        self.set_password(DEMO_PASSWORD.to_string());
    }

    /// The form's validation, a pure function of the state: asked again on every build.
    pub fn form(&self) -> Form {
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
