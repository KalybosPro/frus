//! Tests for `websocket::features::chat::states::chatstate`, moved out of `src/` (milestone 664).

use std::time::Duration;
use websocket::features::chat::controllers::connection::Event;
use websocket::features::chat::states::chatstate::*;
use websocket::models::message::{Kind, TimeOfDay};

const AT: TimeOfDay = TimeOfDay {
    hour: 12,
    minute: 0,
    second: 0,
};

fn connected() -> (ChatState, u64) {
    let mut state = ChatState::new("wss://example.test");
    let session = state.begin_connect();
    state.apply(session, Event::Connected, AT);
    (state, session)
}

fn kinds(state: &ChatState) -> Vec<Kind> {
    state.messages.iter().map(|m| m.kind).collect()
}

#[test]
fn connecting_then_connected() {
    let mut state = ChatState::new("wss://example.test");
    assert_eq!(state.status, Status::Closed);
    let session = state.begin_connect();
    assert_eq!(state.status, Status::Connecting);
    assert_eq!(state.apply(session, Event::Connected, AT), None);
    assert_eq!(state.status, Status::Connected);
    assert_eq!(state.messages[0].text, "Connected to wss://example.test");
}

#[test]
fn received_messages_join_the_conversation_in_order() {
    let (mut state, session) = connected();
    state.apply(session, Event::Received("one".into()), AT);
    state.apply(session, Event::Received("two".into()), AT);
    let texts: Vec<&str> = state.messages[1..]
        .iter()
        .map(|m| m.text.as_str())
        .collect();
    assert_eq!(texts, ["one", "two"]);
    assert_eq!(kinds(&state)[1..], [Kind::Received, Kind::Received]);
}

#[test]
fn sending_moves_the_trimmed_draft_into_the_conversation() {
    let (mut state, _) = connected();
    state.set_draft("  hello  ".into());
    assert_eq!(state.take_draft(AT), Some("hello".to_string()));
    assert!(state.draft.is_empty());
    let last = state.messages.last().expect("the sent message");
    assert_eq!((last.kind, last.text.as_str()), (Kind::Sent, "hello"));
}

#[test]
fn nothing_is_sent_blank_or_while_offline() {
    let (mut state, _) = connected();
    state.set_draft("   ".into());
    assert_eq!(state.take_draft(AT), None);

    let mut offline = ChatState::new("wss://example.test");
    offline.set_draft("hello".into());
    assert!(!offline.can_send());
    assert_eq!(offline.take_draft(AT), None);
    assert_eq!(offline.draft, "hello", "the draft is kept for later");
}

#[test]
fn failures_are_retried_with_a_growing_wait() {
    let mut state = ChatState::new("wss://example.test");
    let mut waits = Vec::new();
    for _ in 0..3 {
        let session = state.begin_connect();
        let wait = state.apply(session, Event::Failed("down".into()), AT);
        assert!(state.should_retry(session));
        waits.push(wait.map(|w| w.as_secs()));
    }
    assert_eq!(waits, [Some(1), Some(2), Some(4)]);
    assert_eq!(kinds(&state), [Kind::Notice], "the failure is told once");
    assert!(matches!(
        state.status,
        Status::Failed {
            retry_in: Some(_),
            ..
        }
    ));
}

#[test]
fn a_success_resets_the_wait() {
    let mut state = ChatState::new("wss://example.test");
    for _ in 0..3 {
        let session = state.begin_connect();
        state.apply(session, Event::Failed("down".into()), AT);
    }
    let session = state.begin_connect();
    state.apply(session, Event::Connected, AT);
    let wait = state.apply(session, Event::Failed("lost".into()), AT);
    assert_eq!(wait, Some(Duration::from_secs(1)));
}

#[test]
fn disconnecting_stops_retries_and_silences_the_old_socket() {
    let (mut state, session) = connected();
    state.disconnect(AT);
    assert_eq!(state.status, Status::Closed);
    assert!(!state.should_retry(session));

    // The socket we closed still reports, late: nothing changes.
    let before = state.messages.len();
    assert_eq!(state.apply(session, Event::Failed("late".into()), AT), None);
    state.apply(session, Event::Received("late".into()), AT);
    assert_eq!(state.status, Status::Closed);
    assert_eq!(state.messages.len(), before);
}

#[test]
fn a_retry_is_dropped_once_the_reader_reconnected_by_hand() {
    let mut state = ChatState::new("wss://example.test");
    let first = state.begin_connect();
    state.apply(first, Event::Failed("down".into()), AT);
    assert!(state.should_retry(first));
    let second = state.begin_connect();
    assert!(
        !state.should_retry(first),
        "the old timer must not open a third socket"
    );
    assert!(!state.should_retry(second), "connecting, not failed");
}

#[test]
fn a_clean_close_by_the_server_is_not_retried() {
    let (mut state, session) = connected();
    let wait = state.apply(session, Event::Closed(Some("bye (code 1000)".into())), AT);
    assert_eq!(wait, None);
    assert_eq!(state.status, Status::Closed);
    assert_eq!(
        state.messages.last().map(|m| m.text.as_str()),
        Some("The server closed the connection: bye (code 1000)")
    );
}

#[test]
fn a_new_address_asks_for_a_reconnect_and_the_same_one_does_not() {
    let mut state = ChatState::new("wss://a.test");
    state.toggle_settings();
    state.set_url_draft(" wss://a.test ".into());
    assert!(!state.apply_url());
    state.toggle_settings();
    state.set_url_draft("wss://b.test".into());
    assert!(state.apply_url());
    assert_eq!(state.url, "wss://b.test");
    assert!(!state.settings_open);
}

#[test]
fn every_message_gets_its_own_id() {
    let (mut state, session) = connected();
    state.apply(session, Event::Received("a".into()), AT);
    state.apply(session, Event::Received("a".into()), AT);
    let mut ids: Vec<u64> = state.messages.iter().map(|m| m.id).collect();
    ids.dedup();
    assert_eq!(ids.len(), state.messages.len());
}
