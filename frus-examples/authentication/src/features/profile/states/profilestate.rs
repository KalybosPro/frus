//! What the profile page keeps between rebuilds, and its rules.

use frus::form::{Form, Rule};

use crate::common::auth::Reply;
use crate::common::submission::Submission;

#[derive(Default)]
pub struct ProfileState {
    /// The display name being edited — starts as the user's current one.
    pub name: String,
    pub submit: Submission,
    /// The last save went through, and nothing has been typed since.
    pub saved: bool,
}

impl ProfileState {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    pub fn set_name(&mut self, value: String) {
        self.name = value;
        self.saved = false;
        self.submit.edited();
    }

    pub fn form(&self, current: &str) -> Form {
        let unchanged = self.name.trim() == current;
        Form::new().field(
            "name",
            &self.name,
            Rule::all([
                Rule::required("Your name is required"),
                Rule::max_len(40, "At most 40 characters"),
                Rule::new(move |_| unchanged.then(|| "This is already your name".to_string())),
            ]),
        )
    }

    pub fn finish(&mut self, reply: Reply) {
        self.saved = reply.is_ok();
        self.submit.finish(reply);
    }
}
