//! Tests for `navigation::features::checkout::states::checkoutstate`, moved out of `src/` (milestone 662).

use navigation::features::checkout::states::checkoutstate::*;

#[test]
fn standard_delivery_is_free_on_a_big_order() {
    let state = CheckoutState::default();
    assert_eq!(
        state.shipping_cents(FREE_SHIPPING_FROM - 1),
        STANDARD_SHIPPING
    );
    assert_eq!(state.shipping_cents(FREE_SHIPPING_FROM), 0);
    assert_eq!(state.total_cents(2_000), 2_000 + STANDARD_SHIPPING);
}

#[test]
fn express_delivery_always_costs() {
    let mut state = CheckoutState::default();
    state.set_express(true);
    assert_eq!(state.shipping_cents(50_000), EXPRESS_SHIPPING);
}
