//! What the "New post" page keeps between rebuilds, and the rules of the form.
//!
//! Plain Rust: the validation, what is sent, and how a request's progress reads as the
//! page's status are all tested below, without a window and without a network.

use frus::form::{Form, Rule};

use crate::common::api::ApiError;
use crate::models::post::{NewPost, Post};

/// The author every post is sent as. The placeholder API has users 1 to 10 and no login.
pub(crate) const AUTHOR_ID: u32 = 1;

pub(crate) struct NewPostState {
    pub title: String,
    pub body: String,
    /// Errors are shown once the user has tried to send, not while they are typing.
    pub submitted: bool,
    /// Which send this is: the request hook runs once per value, so a send bumps it.
    pub attempt: u64,
    /// What the current send carries — `None` before the first one, and after "Write
    /// another".
    pub sending: Option<NewPost>,
}

/// Where the page is, as far as the server is concerned.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SendStatus {
    /// Nothing sent yet: the form is open.
    Idle,
    /// The request is out: the form is locked and the button says so.
    Sending,
    /// The server answered with the post it created.
    Sent(Post),
    /// It failed; the form stays filled in, ready to send again.
    Failed(ApiError),
}

impl NewPostState {
    pub(crate) fn new() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            submitted: false,
            attempt: 0,
            sending: None,
        }
    }

    pub(crate) fn set_title(&mut self, value: String) {
        self.title = value;
    }

    pub(crate) fn set_body(&mut self, value: String) {
        self.body = value;
    }

    pub(crate) fn form(&self) -> Form {
        Form::new()
            .field(
                "title",
                &self.title,
                Rule::all([
                    Rule::required("Give the post a title"),
                    Rule::max_len(100, "At most 100 characters"),
                ]),
            )
            .field(
                "body",
                &self.body,
                Rule::all([
                    Rule::required("Write something"),
                    Rule::min_len(10, "At least 10 characters"),
                ]),
            )
    }

    /// The message to show under a field, once the user has tried to send.
    pub(crate) fn error(&self, key: &str) -> Option<String> {
        if !self.submitted {
            return None;
        }
        self.form().error(key).map(str::to_string)
    }

    /// The Send button. A valid form starts a new send; an invalid one only shows why.
    ///
    /// The button is disabled while a send is out, so this is never called twice for one
    /// post — but the page is not what guarantees that; the caller passes `busy`.
    pub(crate) fn submit(&mut self, busy: bool) {
        self.submitted = true;
        if busy || !self.form().is_valid() {
            return;
        }
        self.sending = Some(NewPost {
            user_id: AUTHOR_ID,
            title: self.title.trim().to_string(),
            body: self.body.trim().to_string(),
        });
        self.attempt += 1;
    }

    /// "Write another": an empty form, and the last answer forgotten.
    pub(crate) fn reset(&mut self) {
        *self = Self {
            attempt: self.attempt,
            ..Self::new()
        };
    }

    /// The page's status, from what the request has given so far.
    ///
    /// `outcome` is the hook's data: `None` while the request is out, then what the
    /// future returned — itself `None` when there was nothing to send.
    pub(crate) fn status(&self, outcome: Option<Option<Result<Post, ApiError>>>) -> SendStatus {
        if self.sending.is_none() {
            return SendStatus::Idle;
        }
        match outcome.flatten() {
            None => SendStatus::Sending,
            Some(Ok(post)) => SendStatus::Sent(post),
            Some(Err(error)) => SendStatus::Failed(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled() -> NewPostState {
        let mut state = NewPostState::new();
        state.set_title("  Hello  ".into());
        state.set_body("A body long enough".into());
        state
    }

    #[test]
    fn errors_wait_for_the_first_send() {
        let mut state = NewPostState::new();
        assert_eq!(state.error("title"), None);
        state.submit(false);
        assert_eq!(
            state.error("title").as_deref(),
            Some("Give the post a title")
        );
        assert_eq!(state.error("body").as_deref(), Some("Write something"));
        assert!(state.sending.is_none(), "an invalid form sends nothing");
    }

    #[test]
    fn a_valid_form_sends_a_trimmed_draft() {
        let mut state = filled();
        state.submit(false);
        assert_eq!(state.attempt, 1);
        assert_eq!(
            state.sending,
            Some(NewPost {
                user_id: AUTHOR_ID,
                title: "Hello".into(),
                body: "A body long enough".into(),
            })
        );
    }

    #[test]
    fn nothing_is_sent_twice_while_busy() {
        let mut state = filled();
        state.submit(false);
        state.submit(true);
        assert_eq!(state.attempt, 1);
    }

    #[test]
    fn the_status_follows_the_request() {
        let mut state = filled();
        assert_eq!(state.status(None), SendStatus::Idle);

        state.submit(false);
        assert_eq!(state.status(None), SendStatus::Sending);

        let created = Post {
            id: 101,
            user_id: AUTHOR_ID,
            title: "Hello".into(),
            body: "A body long enough".into(),
        };
        assert_eq!(
            state.status(Some(Some(Ok(created.clone())))),
            SendStatus::Sent(created)
        );
        assert_eq!(
            state.status(Some(Some(Err(ApiError::Timeout)))),
            SendStatus::Failed(ApiError::Timeout)
        );
    }

    #[test]
    fn write_another_empties_the_form_and_keeps_counting() {
        let mut state = filled();
        state.submit(false);
        state.reset();
        assert!(state.title.is_empty() && state.body.is_empty());
        assert!(!state.submitted);
        assert_eq!(state.status(None), SendStatus::Idle);
        assert_eq!(state.attempt, 1, "the next send must be a new attempt");
    }
}
