//! What the chat page keeps between rebuilds, and every rule about how it changes.
//!
//! The connection's events land here through [`ChatState::apply`], one at a time, on the
//! interface's thread. Nothing in this file touches the network or a widget, so the whole
//! state machine is tested in `tests/chatstate_test.rs` with made-up events.

use std::time::Duration;

use crate::features::chat::controllers::connection::{Connection, Event};
use crate::features::chat::controllers::reconnect::backoff;
use crate::models::message::{Kind, Message, TimeOfDay};

/// Where the connection stands — what the status chip shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
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

pub struct ChatState {
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
    pub fn new(url: impl Into<String>) -> Self {
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
    pub fn begin_connect(&mut self) -> u64 {
        self.close_connection();
        self.wanted = true;
        self.session += 1;
        self.status = Status::Connecting;
        self.session
    }

    /// Keeps the connection opened for the current session.
    pub fn attach(&mut self, connection: Connection) {
        self.connection = Some(connection);
    }

    /// Takes in one event from the connection opened under `session`. Returns the wait
    /// before a retry, when the event was a failure that should be retried.
    pub fn apply(&mut self, session: u64, event: Event, at: TimeOfDay) -> Option<Duration> {
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
    pub fn should_retry(&self, session: u64) -> bool {
        self.wanted && self.session == session && matches!(self.status, Status::Failed { .. })
    }

    /// The reader's Disconnect: close, and stop retrying.
    pub fn disconnect(&mut self, at: TimeOfDay) {
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
    pub fn close_connection(&mut self) {
        if let Some(connection) = self.connection.take() {
            connection.close();
        }
    }

    /// A future that ends when the live connection does, or at once without one. The Web
    /// page waits on it: see `browser.rs`.
    #[cfg(target_arch = "wasm32")]
    pub fn watch(&self) -> crate::features::chat::controllers::connection::Watch {
        use crate::features::chat::controllers::connection::Watch;
        self.connection
            .as_ref()
            .map_or_else(Watch::none, |connection| connection.watch())
    }

    // --- Sending -------------------------------------------------------------------------

    pub fn set_draft(&mut self, text: String) {
        self.draft = text;
    }

    pub fn can_send(&self) -> bool {
        self.status == Status::Connected && !self.draft.trim().is_empty()
    }

    /// Moves the draft into the conversation and returns the text to send, when there is
    /// something to send and somewhere to send it.
    pub fn take_draft(&mut self, at: TimeOfDay) -> Option<String> {
        if !self.can_send() {
            return None;
        }
        let text = self.draft.trim().to_string();
        self.draft.clear();
        self.push(Kind::Sent, text.clone(), at);
        Some(text)
    }

    /// Sends the draft through the live connection.
    pub fn send_draft(&mut self, at: TimeOfDay) {
        if let Some(text) = self.take_draft(at) {
            if let Some(connection) = &self.connection {
                connection.send(text);
            }
        }
    }

    // --- Settings ------------------------------------------------------------------------

    pub fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        // Reopening starts from the address in use, not from an abandoned edit.
        self.url_draft = self.url.clone();
    }

    pub fn set_url_draft(&mut self, text: String) {
        self.url_draft = text;
    }

    /// Takes the edited address. Returns whether it changed, which means: reconnect.
    pub fn apply_url(&mut self) -> bool {
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

    pub fn clear_messages(&mut self) {
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
