//! One line of the conversation: plain data, no widgets, tested below.

use std::fmt;

/// Who said it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Typed here and sent to the server: drawn on the right.
    Sent,
    /// Pushed by the server: drawn on the left.
    Received,
    /// Said by the app itself — "Connected", "Connection lost" — drawn in the middle.
    Notice,
}

/// A time of day on the reader's clock, to the second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl TimeOfDay {
    pub(crate) fn new(hour: u8, minute: u8, second: u8) -> Self {
        Self {
            hour,
            minute,
            second,
        }
    }

    /// Now, in the device's time zone.
    pub(crate) fn now() -> Self {
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
pub(crate) struct Message {
    /// Unique within a conversation: what keeps a bubble's identity in the list.
    pub id: u64,
    pub kind: Kind,
    pub text: String,
    pub at: TimeOfDay,
}

impl Message {
    pub(crate) fn new(id: u64, kind: Kind, text: impl Into<String>, at: TimeOfDay) -> Self {
        Self {
            id,
            kind,
            text: text.into(),
            at,
        }
    }

    /// Whether this line came from the person holding the device.
    pub(crate) fn is_mine(&self) -> bool {
        self.kind == Kind::Sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_of_day_reads_with_two_digits_each() {
        assert_eq!(TimeOfDay::new(9, 5, 3).to_string(), "09:05:03");
        assert_eq!(TimeOfDay::new(23, 59, 59).to_string(), "23:59:59");
    }

    #[test]
    fn only_a_sent_message_is_mine() {
        let at = TimeOfDay::new(12, 0, 0);
        assert!(Message::new(1, Kind::Sent, "hi", at).is_mine());
        assert!(!Message::new(2, Kind::Received, "hi", at).is_mine());
        assert!(!Message::new(3, Kind::Notice, "Connected", at).is_mine());
    }
}
