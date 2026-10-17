//! What every form of the application does around its submit button.
//!
//! Each form page keeps one `Submission` in its state: whether the person has tried to
//! submit yet (errors are shown only from then on — nobody wants to be told off before
//! typing), whether a request is on its way (the button shows it and ignores more
//! presses), and what the server said when it refused.

use frus::form::Form;

use crate::common::auth::Reply;

#[derive(Debug, Default)]
pub struct Submission {
    /// The person has pressed submit at least once.
    pub submitted: bool,
    /// A request is on its way.
    pub loading: bool,
    /// The server's refusal, shown above the form.
    pub server_error: Option<String>,
}

impl Submission {
    /// The validation message of field `key`, once the form has been submitted.
    pub fn error(&self, form: &Form, key: &str) -> Option<String> {
        if self.submitted {
            form.error(key).map(str::to_string)
        } else {
            None
        }
    }

    /// A submit press: whether to send the request. Not when the form is invalid (its
    /// errors now show), and not twice while one is on its way.
    pub fn start(&mut self, valid: bool) -> bool {
        self.submitted = true;
        if !valid || self.loading {
            return false;
        }
        self.loading = true;
        self.server_error = None;
        true
    }

    /// The server's answer is in.
    pub fn finish(&mut self, reply: Reply) {
        self.loading = false;
        self.server_error = reply.err().map(|error| error.to_string());
    }

    /// The person is typing again: an old refusal no longer describes what they see.
    pub fn edited(&mut self) {
        self.server_error = None;
    }
}
