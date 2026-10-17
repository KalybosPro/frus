//! What the password-reset page keeps between rebuilds, and its rules.

use frus::form::{Form, Rule};

use crate::common::auth::Reply;
use crate::common::submission::Submission;

#[derive(Default)]
pub struct ForgotPasswordState {
    pub email: String,
    pub submit: Submission,
    /// Where the link was sent, once the server has answered: the page then shows
    /// "check your email" instead of the form.
    pub sent_to: Option<String>,
}

impl ForgotPasswordState {
    pub fn set_email(&mut self, value: String) {
        self.email = value;
        self.submit.edited();
    }

    pub fn form(&self) -> Form {
        Form::new().field(
            "email",
            &self.email,
            Rule::all([
                Rule::required("Email is required"),
                Rule::email("Enter a valid email address"),
            ]),
        )
    }

    /// The server's answer.
    pub fn finish(&mut self, reply: Reply) {
        if reply.is_ok() {
            self.sent_to = Some(self.email.trim().to_string());
        }
        self.submit.finish(reply);
    }

    /// Back to the form, to try another address.
    pub fn try_again(&mut self) {
        self.sent_to = None;
        self.submit = Submission::default();
    }
}
