//! One line of the cart: a product, in one size, some number of times.

use crate::models::product::Product;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CartItem {
    pub product_id: u32,
    pub name: String,
    pub size: String,
    /// The price when it was added, in cents.
    pub unit_price_cents: u32,
    pub quantity: u32,
}

impl CartItem {
    pub(crate) fn new(product: &Product, size: &str, quantity: u32) -> Self {
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
    pub(crate) fn is(&self, product_id: u32, size: &str) -> bool {
        self.product_id == product_id && self.size == size
    }

    pub(crate) fn total_cents(&self) -> u32 {
        self.unit_price_cents * self.quantity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_total_is_the_price_times_the_quantity() {
        let product = Product::find(1).expect("product 1 is in the catalogue");
        let item = CartItem::new(product, "42", 3);
        assert_eq!(item.total_cents(), 3 * 8900);
        assert!(item.is(1, "42"));
        assert!(!item.is(1, "43"));
    }
}
