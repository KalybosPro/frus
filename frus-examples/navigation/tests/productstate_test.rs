//! Tests for `navigation::features::product::states::productstate`, moved out of `src/` (milestone 664).

use navigation::common::cart::MAX_QUANTITY;
use navigation::features::product::states::productstate::*;

#[test]
fn the_quantity_stays_between_one_and_the_maximum() {
    let mut state = ProductState::new();
    state.fewer();
    assert_eq!(state.quantity, 1);
    for _ in 0..20 {
        state.more();
    }
    assert_eq!(state.quantity, MAX_QUANTITY);
}

#[test]
fn adding_resets_the_quantity_and_remembers() {
    let mut state = ProductState::new();
    state.more();
    state.added();
    assert_eq!(state.quantity, 1);
    assert!(state.added);
}
