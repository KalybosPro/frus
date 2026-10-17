//! Tests for `counter::features::counter::states::counterstate`, moved out of `src/` (milestone 664).

use counter::features::counter::states::counterstate::*;

#[test]
fn counting_follows_the_step() {
    let mut state = CounterState::new(1);
    state.increment();
    state.increment();
    state.decrement();
    assert_eq!(state.count, 1);

    state.set_step(10);
    state.increment();
    assert_eq!(state.count, 11);
}

#[test]
fn reset_also_stops_the_timer() {
    let mut state = CounterState::new(1);
    state.increment();
    state.toggle_auto();
    state.reset();
    assert_eq!(state.count, 0);
    assert!(!state.auto);
}

#[test]
fn the_count_survives_a_reload() {
    let mut before = CounterState::new(1);
    before.set_step(5);
    before.increment();
    let mut after = CounterState::new(1);
    after.take_saved(&before.to_saved());
    assert_eq!((after.count, after.step), (5, 5));

    let mut fresh = CounterState::new(1);
    fresh.take_saved("not a counter");
    assert_eq!((fresh.count, fresh.step), (0, 1));
}
