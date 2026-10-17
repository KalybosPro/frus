//! One line of the conversation: plain data, no widgets, tested in `tests/message_test.rs`.

use std::fmt;

/// Who said it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Typed here and sent to the server: drawn on the right.
    Sent,
    /// Pushed by the server: drawn on the left.
    Received,
    /// Said by the app itself — "Connected", "Connection lost" — drawn in the middle.
    Notice,
}

/// A time of day on the reader's clock, to the second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl TimeOfDay {
    pub fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour,
            minute,
            second,
        }
    }

    /// Now, in the device's time zone.
    pub fn now() -> Self {
        use chrono::Timelike;
        let now = chrono::Local::now();
        // `hour()` < 24 and `minute()`/`second()` < 60 (a leap second shows as 59), so
        // the narrowing never loses anything.
        Self::new(now.hour() as u8, now.minute() as u8, now.second() as u8)
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    /// Unique within a conversation: what keeps a bubble's identity in the list.
    pub id: u64,
    pub kind: Kind,
    pub text: String,
    pub at: TimeOfDay,
}

impl Message {
    pub fn new(id: u64, kind: Kind, text: impl Into<String>, at: TimeOfDay) -> Self {
        Self {
            id,
            kind,
            text: text.into(),
            at,
        }
    }

    /// Whether this line came from the person holding the device.
    pub fn is_mine(&self) -> bool {
        self.kind == Kind::Sent
    }
}
