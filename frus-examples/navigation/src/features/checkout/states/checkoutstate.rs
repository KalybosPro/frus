//! What the checkout keeps — the delivery option — and the arithmetic of the order.

/// Standard delivery is free from this subtotal up.
pub(crate) const FREE_SHIPPING_FROM: u32 = 10_000;
pub(crate) const STANDARD_SHIPPING: u32 = 590;
pub(crate) const EXPRESS_SHIPPING: u32 = 1_490;

#[derive(Default)]
pub(crate) struct CheckoutState {
    pub express: bool,
}

impl CheckoutState {
    pub(crate) fn set_express(&mut self, express: bool) {
        self.express = express;
    }

    pub(crate) fn shipping_cents(&self, subtotal_cents: u32) -> u32 {
        match (self.express, subtotal_cents >= FREE_SHIPPING_FROM) {
            (true, _) => EXPRESS_SHIPPING,
            (false, true) => 0,
            (false, false) => STANDARD_SHIPPING,
        }
    }

    pub(crate) fn total_cents(&self, subtotal_cents: u32) -> u32 {
        subtotal_cents + self.shipping_cents(subtotal_cents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
