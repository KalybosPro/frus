//! Tests for `navigation::features::product::pages::productpage`, moved out of `src/` (milestone 664).

use navigation::features::product::pages::productpage::*;
use navigation::models::product::CATALOG;

#[test]
fn the_next_product_wraps_round() {
    let last = CATALOG.last().expect("the catalogue is not empty");
    assert_eq!(next_after(last).id, CATALOG[0].id);
    assert_eq!(next_after(&CATALOG[0]).id, CATALOG[1].id);
}
