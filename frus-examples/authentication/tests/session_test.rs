//! Tests for `authentication::common::session`, moved out of `src/` (milestone 662).

use authentication::common::session::*;

#[test]
fn a_saved_session_is_read_back_then_forgotten() {
    let path = std::env::temp_dir()
        .join("frus-authentication-test")
        .join("session-roundtrip");
    let session = Session::at(path);
    session.save("u1");
    assert_eq!(session.load(), Some("u1".to_string()));
    session.clear();
    assert_eq!(session.load(), None);
}

#[test]
fn an_in_memory_session_keeps_nothing() {
    let session = Session::in_memory();
    session.save("u1");
    assert_eq!(session.load(), None);
}
