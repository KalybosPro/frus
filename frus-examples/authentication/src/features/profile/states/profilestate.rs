//! What the profile page keeps between rebuilds, and its rules.

use frus::form::{Form, Rule};

use crate::common::auth::Reply;
use crate::common::submission::Submission;

#[derive(Default)]
pub(crate) struct ProfileState {
    /// The display name being edited — starts as the user's current one.
    pub name: String,
    pub submit: Submission,
    /// The last save went through, and nothing has been typed since.
    pub saved: bool,
}

impl ProfileState {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    pub(crate) fn set_name(&mut self, value: String) {
        self.name = value;
        self.saved = false;
        self.submit.edited();
    }

    pub(crate) fn form(&self, current: &str) -> Form {
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

    pub(crate) fn finish(&mut self, reply: Reply) {
        self.saved = reply.is_ok();
        self.submit.finish(reply);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_the_same_name_is_refused() {
        let state = ProfileState::new("Demo User".to_string());
        assert_eq!(
            state.form("Demo User").error("name"),
            Some("This is already your name")
        );
    }

    #[test]
    fn a_new_name_is_valid_and_typing_hides_the_saved_note() {
        let mut state = ProfileState::new("Demo User".to_string());
        state.set_name("Grace".to_string());
        assert!(state.form("Demo User").is_valid());

        assert!(state.submit.start(true));
        state.finish(Ok(()));
        assert!(state.saved);
        state.set_name("Grace H.".to_string());
        assert!(!state.saved);
    }
}
