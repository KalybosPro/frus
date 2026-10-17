//! Tests for `http::features::post::states::poststate`, moved out of `src/` (milestone 664).

use http::features::post::states::poststate::*;

#[test]
fn retry_is_a_new_attempt() {
    let mut state = PostState::new();
    state.retry();
    state.retry();
    assert_eq!(state.attempt, 2);
}
