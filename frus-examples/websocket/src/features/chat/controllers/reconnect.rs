//! How long to wait before trying again after a connection failed.
//!
//! Exponential backoff: each failure in a row doubles the wait, up to a ceiling. A server
//! that is down is not hammered, and one that comes back is found again within
//! [`MAX_DELAY`]. A success resets the count (see `ChatState::apply`).

use std::time::Duration;

/// The wait after the first failure.
pub(crate) const FIRST_DELAY: Duration = Duration::from_secs(1);

/// The longest wait, however many failures came before.
pub(crate) const MAX_DELAY: Duration = Duration::from_secs(30);

/// The wait before retry number `attempt` (0 for the first retry): 1 s, 2 s, 4 s, 8 s,
/// 16 s, then 30 s for good.
///
/// An app with thousands of clients would add some randomness ("jitter") so that they
/// do not all retry in the same second after an outage; one client does not need it, and
/// a fixed schedule is one a test can check.
pub(crate) fn backoff(attempt: u32) -> Duration {
    // `checked_shl` is `None` from 32 doublings on: far past the ceiling anyway.
    let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
    FIRST_DELAY.saturating_mul(factor).min(MAX_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
