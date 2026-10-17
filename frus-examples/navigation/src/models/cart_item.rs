//! One line of the cart: a product, in one size, some number of times.

use crate::models::product::Product;

#[derive(Clone, Debug, PartialEq)]
pub struct CartItem {
    pub product_id: u32,
    pub name: String,
    pub size: String,
    /// The price when it was added, in cents.
    pub unit_price_cents: u32,
    pub quantity: u32,
}

impl CartItem {
    pub fn new(product: &Product, size: &str, quantity: u32) -> Self {
        Self {
            product_id: product.id,
            name: product.name.to_string(),
            size: size.to_string(),
            unit_price_cents: product.price_cents,
            quantity,
        }
    }

    /// Whether this line is for that product in that size: the same shoe in two sizes is
    /// two lines.
    pub fn is(&self, product_id: u32, size: &str) -> bool {
        self.product_id == product_id && self.size == size
    }

    pub fn total_cents(&self) -> u32 {
        self.unit_price_cents * self.quantity
    }
}
