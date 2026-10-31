//! What the "New post" page keeps between rebuilds, and the rules of the form.
//!
//! Plain Rust: the validation, what is sent, and how a request's progress reads as the
//! page's status are all tested in `tests/newpoststate_test.rs`, without a window and without a network.

use frus::form::{Form, Rule};

use crate::common::api::ApiError;
use crate::models::post::{NewPost, Post};

/// The author every post is sent as. The placeholder API has users 1 to 10 and no login.
pub const AUTHOR_ID: u32 = 1;

pub struct NewPostState {
    pub title: String,
    pub body: String,
    /// Errors are shown once the user has tried to send, not while they are typing.
    pub submitted: bool,
    /// Where the last send has got to.
    pub status: SendStatus,
}

/// Where the page is, as far as the server is concerned.
#[derive(Clone, Debug, PartialEq)]
pub enum SendStatus {
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
    pub fn new() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            submitted: false,
            status: SendStatus::Idle,
        }
    }

    pub fn set_title(&mut self, value: String) {
        self.title = value;
    }

    pub fn set_body(&mut self, value: String) {
        self.body = value;
    }

    pub fn form(&self) -> Form {
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
    pub fn error(&self, key: &str) -> Option<String> {
        if !self.submitted {
            return None;
        }
        self.form().error(key).map(str::to_string)
    }

    /// What Send sends: the trimmed post, when the form is valid and no send is out —
    /// the button is disabled while one is, but the page is not what guarantees that.
    pub fn draft(&self) -> Option<NewPost> {
        if self.status == SendStatus::Sending || !self.form().is_valid() {
            return None;
        }
        Some(NewPost {
            user_id: AUTHOR_ID,
            title: self.title.trim().to_string(),
            body: self.body.trim().to_string(),
        })
    }

    /// The server's answer to the send that was out.
    pub fn finish(&mut self, answer: Result<Post, ApiError>) {
        self.status = match answer {
            Ok(post) => SendStatus::Sent(post),
            Err(error) => SendStatus::Failed(error),
        };
    }

    /// "Write another": an empty form, and the last answer forgotten.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for NewPostState {
    fn default() -> Self {
        Self::new()
    }
}
