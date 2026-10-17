//! Tests for `http::features::new_post::states::newpoststate`, moved out of `src/` (milestone 662).

use http::common::api::ApiError;
use http::features::new_post::states::newpoststate::*;
use http::models::post::{NewPost, Post};

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
