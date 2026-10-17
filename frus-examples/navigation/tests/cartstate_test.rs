//! Tests for `navigation::features::cart::states::cartstate`, moved out of `src/` (milestone 664).

use navigation::features::cart::states::cartstate::*;
use navigation::models::cart_item::CartItem;
use navigation::models::product::Product;

#[test]
fn a_removed_line_can_be_taken_back_once() {
    let product = Product::find(1).expect("product 1 is in the catalogue");
    let mut state = CartState::default();
    state.remember(Some(CartItem::new(product, "42", 1)));
    assert!(state.take_removed().is_some());
    assert!(state.take_removed().is_none());
}
