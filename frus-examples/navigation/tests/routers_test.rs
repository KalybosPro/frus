//! Tests for `navigation::common::routers`, moved out of `src/` (milestone 664).

use navigation::common::routers::*;
use navigation::common::routes::*;

#[test]
fn an_empty_cart_cannot_reach_the_checkout() {
    assert_eq!(guard_redirect(true, CHECKOUT), Some(CART.to_string()));
    assert_eq!(
        guard_redirect(true, "/checkout?from=drawer"),
        Some(CART.to_string()),
        "the query does not get it past the guard"
    );
}

#[test]
fn a_full_cart_goes_through() {
    assert_eq!(guard_redirect(false, CHECKOUT), None);
}

#[test]
fn the_guard_leaves_every_other_page_alone() {
    for location in [HOME, SEARCH, CART, "/products/3", "/nowhere", "/checkouts"] {
        assert_eq!(guard_redirect(true, location), None, "{location}");
    }
}
