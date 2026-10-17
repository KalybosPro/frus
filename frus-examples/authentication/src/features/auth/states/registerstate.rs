//! What the registration page keeps between rebuilds, and its rules.

use frus::form::{Form, Rule};

use crate::common::submission::Submission;

#[derive(Default)]
pub struct RegisterState {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm: String,
    pub visibility: bool,
    pub submit: Submission,
}

impl RegisterState {
    pub fn set_name(&mut self, value: String) {
        self.name = value;
        self.submit.edited();
    }

    pub fn set_email(&mut self, value: String) {
        self.email = value;
        self.submit.edited();
    }

    pub fn set_password(&mut self, value: String) {
        self.password = value;
        self.submit.edited();
    }

    pub fn set_confirm(&mut self, value: String) {
        self.confirm = value;
        self.submit.edited();
    }

    pub fn toggle_visibility(&mut self) {
        self.visibility = !self.visibility;
    }

    pub fn form(&self) -> Form {
        Form::new()
            .field(
                "name",
                &self.name,
                Rule::all([
                    Rule::required("Your name is required"),
                    Rule::max_len(40, "At most 40 characters"),
                ]),
            )
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
                    // `Rule::new` takes any check: here, at least one digit.
                    Rule::new(|v| {
                        (!v.chars().any(|c| c.is_ascii_digit()))
                            .then(|| "Add at least one digit".to_string())
                    }),
                ]),
            )
            // A rule across two fields: declared after the one it compares with.
            .matches("confirm", &self.confirm, "password", "The passwords differ")
    }
}
