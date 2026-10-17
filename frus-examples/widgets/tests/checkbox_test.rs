//! Tests for `widgets::features::inputs::widgets::checkbox`, moved out of `src/` (milestone 664).

use frus::StatefulWidget;
use widgets::features::inputs::widgets::checkbox::*;

#[test]
fn the_parent_is_partly_on_when_some_children_are() {
    let mut state = CheckboxDemo.create_state();
    assert_eq!(state.all_toppings(), None);
    state.set_all(Some(true));
    assert_eq!(state.all_toppings(), Some(true));
    state.set_all(Some(false));
    assert_eq!(state.all_toppings(), Some(false));
}
