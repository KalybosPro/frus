//! Tests for `http::features::posts::states::postsstate`, moved out of `src/` (milestone 662).

use http::features::posts::states::postsstate::*;

#[test]
fn every_reload_is_a_new_generation() {
    let mut state = PostsState::new();
    state.reload();
    state.pull_to_refresh();
    assert_eq!(state.generation, 2);
}

#[test]
fn the_state_remembers_who_asked() {
    let mut state = PostsState::new();
    state.pull_to_refresh();
    assert!(state.pulled);
    state.reload();
    assert!(!state.pulled);
}
