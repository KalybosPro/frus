//! What the chat page keeps between rebuilds, and every rule about how it changes.
//!
//! The connection's events land here through [`ChatState::apply`], one at a time, on the
//! interface's thread. Nothing in this file touches the network or a widget, so the whole
//! state machine is tested below with made-up events.

use std::time::Duration;

use crate::features::chat::controllers::connection::{Connection, Event};
use crate::features::chat::controllers::reconnect::backoff;
use crate::models::message::{Kind, Message, TimeOfDay};

/// Where the connection stands — what the status chip shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Status {
    Connecting,
    Connected,
    /// Not connected, and not trying: the reader disconnected, or the server closed.
    Closed,
    /// The last attempt failed. `retry_in` is the wait before the next one, if one is
    /// coming.
    Failed {
        error: String,
        retry_in: Option<Duration>,
    },
}

pub(crate) struct ChatState {
    /// The server the next connection goes to.
    pub url: String,
    /// What the server field holds while it is being edited.
    pub url_draft: String,
    pub settings_open: bool,
    /// What the message field holds.
    pub draft: String,
    pub status: Status,
    pub messages: Vec<Message>,
    /// Failures in a row since the last successful connection: picks the next wait.
    pub attempt: u32,
    /// Whether the reader wants to be connected. Disconnect clears it, so that a failure
    /// that comes after is not retried.
    pub wanted: bool,
    /// Bumped by every connect and every disconnect. Events and retries carry the number
    /// they were started under; an older number belongs to a socket that is gone, and is
    /// ignored — the one guard against a late event from a previous connection.
    pub session: u64,
    next_id: u64,
    /// The live connection, if any. Dropping it closes the socket.
    connection: Option<Connection>,
}

impl ChatState {
    pub(crate) fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            url_draft: url.clone(),
            url,
            settings_open: false,
            draft: String::new(),
            status: Status::Closed,
            messages: Vec::new(),
            attempt: 0,
            wanted: false,
            session: 0,
            next_id: 0,
            connection: None,
        }
    }

    // --- The connection's lifecycle ----------------------------------------------------

    /// Starts a new session: any previous connection is closed and its late events will be
    /// ignored. Returns the session number the new connection must report under.
    pub(crate) fn begin_connect(&mut self) -> u64 {
        self.close_connection();
        self.wanted = true;
        self.session += 1;
        self.status = Status::Connecting;
        self.session
    }

    /// Keeps the connection opened for the current session.
    pub(crate) fn attach(&mut self, connection: Connection) {
        self.connection = Some(connection);
    }

    /// Takes in one event from the connection opened under `session`. Returns the wait
    /// before a retry, when the event was a failure that should be retried.
    pub(crate) fn apply(&mut self, session: u64, event: Event, at: TimeOfDay) -> Option<Duration> {
        if session != self.session {
            return None;
        }
        match event {
            Event::Connected => {
                self.status = Status::Connected;
                self.attempt = 0;
                self.notice(format!("Connected to {}", self.url), at);
                None
            }
            Event::Received(text) => {
                self.push(Kind::Received, text, at);
                None
            }
            Event::Closed(reason) => {
                self.connection = None;
                self.status = Status::Closed;
                let text = match reason {
                    Some(reason) => format!("The server closed the connection: {reason}"),
                    None => "The server closed the connection".to_string(),
                };
                self.notice(text, at);
                None
            }
            Event::Failed(error) => {
                self.connection = None;
                let retry_in = self.wanted.then(|| backoff(self.attempt));
                // Said once per run of failures: the retries that follow show in the
                // status strip, not as one more line every half minute.
                if self.attempt == 0 {
                    self.notice(format!("Connection failed: {error}"), at);
                }
                self.attempt = self.attempt.saturating_add(1);
                self.status = Status::Failed { error, retry_in };
                retry_in
            }
        }
    }

    /// Whether a retry scheduled under `session` should still happen when its time comes:
    /// not if the reader disconnected, connected again by hand, or left the page.
    pub(crate) fn should_retry(&self, session: u64) -> bool {
        self.wanted && self.session == session && matches!(self.status, Status::Failed { .. })
    }

    /// The reader's Disconnect: close, and stop retrying.
    pub(crate) fn disconnect(&mut self, at: TimeOfDay) {
        let was_live = self.connection.is_some();
        self.close_connection();
        self.wanted = false;
        self.session += 1;
        self.attempt = 0;
        self.status = Status::Closed;
        if was_live {
            self.notice("Disconnected", at);
        }
    }

    /// Closes the socket, if there is one. Called when the page goes away, too.
    pub(crate) fn close_connection(&mut self) {
        if let Some(connection) = self.connection.take() {
            connection.close();
        }
    }

    /// A future that ends when the live connection does, or at once without one. The Web
    /// page waits on it: see `browser.rs`.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn watch(&self) -> crate::features::chat::controllers::connection::Watch {
        use crate::features::chat::controllers::connection::Watch;
        self.connection
            .as_ref()
            .map_or_else(Watch::none, |connection| connection.watch())
    }

    // --- Sending -------------------------------------------------------------------------

    pub(crate) fn set_draft(&mut self, text: String) {
        self.draft = text;
    }

    pub(crate) fn can_send(&self) -> bool {
        self.status == Status::Connected && !self.draft.trim().is_empty()
    }

    /// Moves the draft into the conversation and returns the text to send, when there is
    /// something to send and somewhere to send it.
    pub(crate) fn take_draft(&mut self, at: TimeOfDay) -> Option<String> {
        if !self.can_send() {
            return None;
        }
        let text = self.draft.trim().to_string();
        self.draft.clear();
        self.push(Kind::Sent, text.clone(), at);
        Some(text)
    }

    /// Sends the draft through the live connection.
    pub(crate) fn send_draft(&mut self, at: TimeOfDay) {
        if let Some(text) = self.take_draft(at) {
            if let Some(connection) = &self.connection {
                connection.send(text);
            }
        }
    }

    // --- Settings ------------------------------------------------------------------------

    pub(crate) fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        // Reopening starts from the address in use, not from an abandoned edit.
        self.url_draft = self.url.clone();
    }

    pub(crate) fn set_url_draft(&mut self, text: String) {
        self.url_draft = text;
    }

    /// Takes the edited address. Returns whether it changed, which means: reconnect.
    pub(crate) fn apply_url(&mut self) -> bool {
        let url = self.url_draft.trim().to_string();
        self.settings_open = false;
        if url.is_empty() || url == self.url {
            self.url_draft = self.url.clone();
            return false;
        }
        self.url = url;
        self.url_draft = self.url.clone();
        true
    }

    pub(crate) fn clear_messages(&mut self) {
        self.messages.clear();
    }

    // --- Helpers -------------------------------------------------------------------------

    fn push(&mut self, kind: Kind, text: impl Into<String>, at: TimeOfDay) {
        self.next_id += 1;
        self.messages
            .push(Message::new(self.next_id, kind, text, at));
    }

    fn notice(&mut self, text: impl Into<String>, at: TimeOfDay) {
        self.push(Kind::Notice, text, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
