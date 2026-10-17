//! Tests for `websocket::features::chat::controllers::reconnect`, moved out of `src/` (milestone 662).

use websocket::features::chat::controllers::reconnect::*;

#[test]
fn the_wait_doubles_then_stops_at_the_ceiling() {
    let schedule: Vec<u64> = (0..7).map(|n| backoff(n).as_secs()).collect();
    assert_eq!(schedule, [1, 2, 4, 8, 16, 30, 30]);
}

#[test]
fn a_very_long_run_of_failures_stays_at_the_ceiling() {
    assert_eq!(backoff(31), MAX_DELAY);
    assert_eq!(backoff(32), MAX_DELAY);
    assert_eq!(backoff(u32::MAX), MAX_DELAY);
}
