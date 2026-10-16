//! What the cart page keeps: the line just removed, so it can be put back.
//!
//! The lines themselves are not here — they are the shared [`Cart`](crate::common::cart::Cart)'s,
//! because other pages read them too. A page's own state is only what that page needs.

use crate::models::cart_item::CartItem;

#[derive(Default)]
pub(crate) struct CartState {
    pub removed: Option<CartItem>,
}

impl CartState {
    pub(crate) fn remember(&mut self, item: Option<CartItem>) {
        self.removed = item;
    }

    /// The line to put back, once.
    pub(crate) fn take_removed(&mut self) -> Option<CartItem> {
        self.removed.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::product::Product;

    #[test]
    fn a_removed_line_can_be_taken_back_once() {
        let product = Product::find(1).expect("product 1 is in the catalogue");
        let mut state = CartState::default();
        state.remember(Some(CartItem::new(product, "42", 1)));
        assert!(state.take_removed().is_some());
        assert!(state.take_removed().is_none());
    }
}
