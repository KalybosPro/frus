//! What the product page keeps: how many to add, and whether they were added.

use crate::common::cart::MAX_QUANTITY;

pub(crate) struct ProductState {
    pub quantity: u32,
    /// Set once this page added something, to offer the way to the cart.
    pub added: bool,
}

impl ProductState {
    pub(crate) fn new() -> Self {
        Self {
            quantity: 1,
            added: false,
        }
    }

    pub(crate) fn more(&mut self) {
        self.quantity = (self.quantity + 1).min(MAX_QUANTITY);
    }

    pub(crate) fn fewer(&mut self) {
        self.quantity = self.quantity.saturating_sub(1).max(1);
    }

    /// After adding: back to one, and the page says it was added.
    pub(crate) fn added(&mut self) {
        self.quantity = 1;
        self.added = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_quantity_stays_between_one_and_the_maximum() {
        let mut state = ProductState::new();
        state.fewer();
        assert_eq!(state.quantity, 1);
        for _ in 0..20 {
            state.more();
        }
        assert_eq!(state.quantity, MAX_QUANTITY);
    }

    #[test]
    fn adding_resets_the_quantity_and_remembers() {
        let mut state = ProductState::new();
        state.more();
        state.added();
        assert_eq!(state.quantity, 1);
        assert!(state.added);
    }
}
