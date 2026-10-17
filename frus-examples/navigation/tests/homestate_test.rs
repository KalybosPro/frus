//! Tests for `navigation::features::home::states::homestate`, moved out of `src/` (milestone 664).

use navigation::features::home::states::homestate::*;
use navigation::models::product::{Category, CATALOG};

#[test]
fn everything_shows_until_a_category_is_picked() {
    let mut state = HomeState::default();
    assert_eq!(state.visible().len(), CATALOG.len());

    state.show(Some(Category::Bags));
    let bags = state.visible();
    assert!(!bags.is_empty());
    assert!(bags.iter().all(|p| p.category == Category::Bags));

    state.show(None);
    assert_eq!(state.visible().len(), CATALOG.len());
}
