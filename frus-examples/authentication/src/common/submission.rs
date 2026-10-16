//! What every form of the application does around its submit button.
//!
//! Each form page keeps one `Submission` in its state: whether the person has tried to
//! submit yet (errors are shown only from then on — nobody wants to be told off before
//! typing), whether a request is on its way (the button shows it and ignores more
//! presses), and what the server said when it refused.

use frus::form::Form;

use crate::common::auth::Reply;

#[derive(Debug, Default)]
pub(crate) struct Submission {
    /// The person has pressed submit at least once.
    pub submitted: bool,
    /// A request is on its way.
    pub loading: bool,
    /// The server's refusal, shown above the form.
    pub server_error: Option<String>,
}

impl Submission {
    /// The validation message of field `key`, once the form has been submitted.
    pub(crate) fn error(&self, form: &Form, key: &str) -> Option<String> {
        if self.submitted {
            form.error(key).map(str::to_string)
        } else {
            None
        }
    }

    /// A submit press: whether to send the request. Not when the form is invalid (its
    /// errors now show), and not twice while one is on its way.
    pub(crate) fn start(&mut self, valid: bool) -> bool {
        self.submitted = true;
        if !valid || self.loading {
            return false;
        }
        self.loading = true;
        self.server_error = None;
        true
    }

    /// The server's answer is in.
    pub(crate) fn finish(&mut self, reply: Reply) {
        self.loading = false;
        self.server_error = reply.err().map(|error| error.to_string());
    }

    /// The person is typing again: an old refusal no longer describes what they see.
    pub(crate) fn edited(&mut self) {
        self.server_error = None;
    }
}

#[cfg(test)]
mod tests {
    use frus::form::Rule;

    use super::*;
    use crate::features::auth::controllers::user_store::AuthError;

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
}
