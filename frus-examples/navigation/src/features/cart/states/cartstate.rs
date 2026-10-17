//! What the cart page keeps: the line just removed, so it can be put back.
//!
//! The lines themselves are not here — they are the shared [`Cart`](crate::common::cart::Cart)'s,
//! because other pages read them too. A page's own state is only what that page needs.

use crate::models::cart_item::CartItem;

#[derive(Default)]
pub struct CartState {
    pub removed: Option<CartItem>,
}

impl CartState {
    pub fn remember(&mut self, item: Option<CartItem>) {
        self.removed = item;
    }

    /// The line to put back, once.
    pub fn take_removed(&mut self) -> Option<CartItem> {
        self.removed.take()
    }
}
