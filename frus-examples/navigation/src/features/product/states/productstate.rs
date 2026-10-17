//! What the product page keeps: how many to add, and whether they were added.

use crate::common::cart::MAX_QUANTITY;

pub struct ProductState {
    pub quantity: u32,
    /// Set once this page added something, to offer the way to the cart.
    pub added: bool,
}

impl ProductState {
    pub fn new() -> Self {
        Self {
            quantity: 1,
            added: false,
        }
    }

    pub fn more(&mut self) {
        self.quantity = (self.quantity + 1).min(MAX_QUANTITY);
    }

    pub fn fewer(&mut self) {
        self.quantity = self.quantity.saturating_sub(1).max(1);
    }

    /// After adding: back to one, and the page says it was added.
    pub fn added(&mut self) {
        self.quantity = 1;
        self.added = true;
    }
}

impl Default for ProductState {
    fn default() -> Self {
        Self::new()
    }
}
