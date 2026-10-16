//! What the password-reset page keeps between rebuilds, and its rules.

use frus::form::{Form, Rule};

use crate::common::auth::Reply;
use crate::common::submission::Submission;

#[derive(Default)]
pub(crate) struct ForgotPasswordState {
    pub email: String,
    pub submit: Submission,
    /// Where the link was sent, once the server has answered: the page then shows
    /// "check your email" instead of the form.
    pub sent_to: Option<String>,
}

impl ForgotPasswordState {
    pub(crate) fn set_email(&mut self, value: String) {
        self.email = value;
        self.submit.edited();
    }

    pub(crate) fn form(&self) -> Form {
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
    pub(crate) fn finish(&mut self, reply: Reply) {
        if reply.is_ok() {
            self.sent_to = Some(self.email.trim().to_string());
        }
        self.submit.finish(reply);
    }

    /// Back to the form, to try another address.
    pub(crate) fn try_again(&mut self) {
        self.sent_to = None;
        self.submit = Submission::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
