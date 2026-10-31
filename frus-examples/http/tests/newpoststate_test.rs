//! Tests for `http::features::new_post::states::newpoststate`, moved out of `src/` (milestone 664).

use http::common::api::ApiError;
use http::features::new_post::states::newpoststate::*;
use http::models::post::{NewPost, Post};

fn filled() -> NewPostState {
    let mut state = NewPostState::new();
    state.set_title("  Hello  ".into());
    state.set_body("A body long enough".into());
    state
}

fn created() -> Post {
    Post {
        id: 101,
        user_id: AUTHOR_ID,
        title: "Hello".into(),
        body: "A body long enough".into(),
    }
}

#[test]
fn errors_wait_for_the_first_send() {
    let mut state = NewPostState::new();
    assert_eq!(state.error("title"), None);
    state.submitted = true;
    assert_eq!(
        state.error("title").as_deref(),
        Some("Give the post a title")
    );
    assert_eq!(state.error("body").as_deref(), Some("Write something"));
    assert_eq!(state.draft(), None, "an invalid form sends nothing");
}

#[test]
fn a_valid_form_sends_a_trimmed_draft() {
    assert_eq!(
        filled().draft(),
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
    state.status = SendStatus::Sending;
    assert_eq!(state.draft(), None);
}

#[test]
fn the_status_follows_the_answer() {
    let mut state = filled();
    assert_eq!(state.status, SendStatus::Idle);

    state.finish(Ok(created()));
    assert_eq!(state.status, SendStatus::Sent(created()));

    state.finish(Err(ApiError::Timeout));
    assert_eq!(state.status, SendStatus::Failed(ApiError::Timeout));
    assert!(state.draft().is_some(), "a failed send can be sent again");
}

#[test]
fn write_another_empties_the_form() {
    let mut state = filled();
    state.submitted = true;
    state.finish(Ok(created()));
    state.reset();
    assert!(state.title.is_empty() && state.body.is_empty());
    assert!(!state.submitted);
    assert_eq!(state.status, SendStatus::Idle);
}
