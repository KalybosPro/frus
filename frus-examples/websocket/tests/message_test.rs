//! Tests for `websocket::models::message`, moved out of `src/` (milestone 662).

use websocket::models::message::*;

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
