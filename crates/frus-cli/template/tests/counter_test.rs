//! The counter's tests. Each file in `tests/` is built against the application as a library,
//! so it tests what `src/` makes public. `cargo test` runs them: they need no window and no
//! GPU.

use {{crate_name}}::CounterState;

/// The counter's rules are plain Rust: testable with no widget, no GPU and no window.
#[test]
fn counting_is_plain_state() {
    let mut state = CounterState::default();
    state.increment();
    state.increment();
    state.decrement();
    assert_eq!(state.count, 1);
}

#[test]
fn the_count_does_not_go_below_zero() {
    let mut state = CounterState::default();
    state.decrement();
    assert_eq!(state.count, 0);
}

#[test]
fn automatic_counting_toggles() {
    let mut state = CounterState::default();
    assert!(!state.auto);
    state.toggle_auto();
    assert!(state.auto);
    state.toggle_auto();
    assert!(!state.auto);
}
