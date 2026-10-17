//! An order once it is placed: what the checkout hands the confirmation page.

use crate::models::cart_item::CartItem;

#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub items: Vec<CartItem>,
    pub shipping_cents: u32,
    pub total_cents: u32,
}

impl Order {
    /// How many articles, counting quantities.
    pub fn count(&self) -> u32 {
        self.items.iter().map(|item| item.quantity).sum()
    }
}
