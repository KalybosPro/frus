//! What the checkout keeps — the delivery option — and the arithmetic of the order.

/// Standard delivery is free from this subtotal up.
pub const FREE_SHIPPING_FROM: u32 = 10_000;
pub const STANDARD_SHIPPING: u32 = 590;
pub const EXPRESS_SHIPPING: u32 = 1_490;

#[derive(Default)]
pub struct CheckoutState {
    pub express: bool,
}

impl CheckoutState {
    pub fn set_express(&mut self, express: bool) {
        self.express = express;
    }

    pub fn shipping_cents(&self, subtotal_cents: u32) -> u32 {
        match (self.express, subtotal_cents >= FREE_SHIPPING_FROM) {
            (true, _) => EXPRESS_SHIPPING,
            (false, true) => 0,
            (false, false) => STANDARD_SHIPPING,
        }
    }

    pub fn total_cents(&self, subtotal_cents: u32) -> u32 {
        subtotal_cents + self.shipping_cents(subtotal_cents)
    }
}
