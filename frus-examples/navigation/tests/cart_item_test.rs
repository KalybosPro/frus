//! Tests for `navigation::models::cart_item`, moved out of `src/` (milestone 662).

use navigation::models::cart_item::*;
use navigation::models::product::Product;

#[test]
fn the_total_is_the_price_times_the_quantity() {
    let product = Product::find(1).expect("product 1 is in the catalogue");
    let item = CartItem::new(product, "42", 3);
    assert_eq!(item.total_cents(), 3 * 8900);
    assert!(item.is(1, "42"));
    assert!(!item.is(1, "43"));
}
