//! Tests for `widgets::features::display::widgets::datatable`, moved out of `src/` (milestone 664).

use widgets::features::display::widgets::datatable::*;

#[test]
fn sorting_the_same_column_flips_direction() {
    let mut state = PeopleTableState::default();
    state.sort_by(2);
    assert_eq!(state.sort, Some((2, true)));
    state.sort_by(2);
    assert_eq!(state.sort, Some((2, false)));
    state.sort_by(0);
    assert_eq!(state.sort, Some((0, true)));
}

#[test]
fn check_all_toggles_everything() {
    let mut state = PeopleTableState::default();
    state.toggle(3);
    state.toggle_all();
    assert_eq!(state.checked.len(), PEOPLE.len());
    state.toggle_all();
    assert!(state.checked.is_empty());
}
