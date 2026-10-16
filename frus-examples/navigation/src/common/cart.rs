//! The cart: **one object shared by every page** that needs it.
//!
//! It is made once, in `lib.rs`, and handed to the router, which hands it to the pages
//! it builds. Cloning a `Cart` clones handles, not contents: every clone reads and
//! writes the same lines.
//!
//! Its values are `ValueNotifier`s. Changing one asks the application for a rebuild, so
//! a page that reads the cart while it builds shows the change without subscribing to
//! anything. The router is the one listener: see [`Cart::emptied`].

use std::collections::HashMap;

use frus::{ChangeNotifier, ValueNotifier};

use crate::models::{cart_item::CartItem, product::Product};

/// The most of one line a customer may order.
pub(crate) const MAX_QUANTITY: u32 = 9;

#[derive(Clone, Default)]
pub(crate) struct Cart {
    items: ValueNotifier<Vec<CartItem>>,
    /// The size picked for each product, before it is added. The size page writes it
    /// and pops; the product page reads it. This is how a page **returns a result**.
    sizes: ValueNotifier<HashMap<u32, String>>,
    emptied: ChangeNotifier,
}

impl Cart {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    // ---- reading ------------------------------------------------------------------

    pub(crate) fn items(&self) -> Vec<CartItem> {
        self.items.get()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.with(|items| items.is_empty())
    }

    /// How many articles, counting quantities — the number on the cart's badge.
    pub(crate) fn count(&self) -> u32 {
        self.items
            .with(|items| items.iter().map(|item| item.quantity).sum())
    }

    pub(crate) fn subtotal_cents(&self) -> u32 {
        self.items
            .with(|items| items.iter().map(CartItem::total_cents).sum())
    }

    pub(crate) fn chosen_size(&self, product_id: u32) -> Option<String> {
        self.sizes.with(|sizes| sizes.get(&product_id).cloned())
    }

    // ---- changing -----------------------------------------------------------------

    /// Adds `quantity` of `product` in `size`, to the line already there if there is one.
    pub(crate) fn add(&self, product: &Product, size: &str, quantity: u32) {
        self.items.update(
            |items| match items.iter_mut().find(|item| item.is(product.id, size)) {
                Some(item) => item.quantity = (item.quantity + quantity).min(MAX_QUANTITY),
                None => items.push(CartItem::new(product, size, quantity.min(MAX_QUANTITY))),
            },
        );
    }

    /// Sets a line's quantity; zero removes the line.
    pub(crate) fn set_quantity(&self, product_id: u32, size: &str, quantity: u32) {
        if quantity == 0 {
            self.remove(product_id, size);
            return;
        }
        self.items.update(|items| {
            if let Some(item) = items.iter_mut().find(|item| item.is(product_id, size)) {
                item.quantity = quantity.min(MAX_QUANTITY);
            }
        });
    }

    /// Removes a line and gives it back, so the page can offer to undo.
    pub(crate) fn remove(&self, product_id: u32, size: &str) -> Option<CartItem> {
        let mut removed = None;
        self.change(|items| {
            if let Some(at) = items.iter().position(|item| item.is(product_id, size)) {
                removed = Some(items.remove(at));
            }
        });
        removed
    }

    /// Puts back a line that was removed.
    pub(crate) fn restore(&self, item: CartItem) {
        self.items.update(|items| {
            if !items
                .iter()
                .any(|line| line.is(item.product_id, &item.size))
            {
                items.push(item);
            }
        });
    }

    pub(crate) fn clear(&self) {
        self.change(Vec::clear);
    }

    pub(crate) fn choose_size(&self, product_id: u32, size: &str) {
        self.sizes.update(|sizes| {
            sizes.insert(product_id, size.to_string());
        });
    }

    /// Runs a change that may remove lines, and says so when it leaves the cart empty.
    fn change(&self, change: impl FnOnce(&mut Vec<CartItem>)) {
        let was_empty = self.is_empty();
        self.items.update(change);
        if !was_empty && self.is_empty() {
            self.emptied.notify_listeners();
        }
    }

    // ---- what the router listens to -----------------------------------------------

    /// Notifies when the cart **becomes** empty.
    ///
    /// The checkout guard depends on one fact — is the cart empty? — so the router
    /// listens to exactly that, not to every change of quantity. When it fires the router
    /// asks its redirect again about the page on show, and a customer on the checkout page
    /// who removes the last line is sent back to the cart without any page navigating.
    pub(crate) fn emptied(&self) -> &ChangeNotifier {
        &self.emptied
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use frus::Listenable;

    use super::*;

    fn product(id: u32) -> &'static Product {
        Product::find(id).expect("the catalogue has this product")
    }

    #[test]
    fn adding_the_same_product_and_size_grows_one_line() {
        let cart = Cart::new();
        cart.add(product(1), "42", 1);
        cart.add(product(1), "42", 2);
        cart.add(product(1), "43", 1);
        assert_eq!(cart.items().len(), 2, "two sizes, two lines");
        assert_eq!(cart.count(), 4);
        assert_eq!(cart.subtotal_cents(), 4 * 8900);
    }

    #[test]
    fn a_quantity_never_goes_past_the_maximum() {
        let cart = Cart::new();
        cart.add(product(2), "40", 7);
        cart.add(product(2), "40", 7);
        assert_eq!(cart.count(), MAX_QUANTITY);
        cart.set_quantity(2, "40", 50);
        assert_eq!(cart.count(), MAX_QUANTITY);
    }

    #[test]
    fn a_quantity_of_zero_removes_the_line() {
        let cart = Cart::new();
        cart.add(product(2), "40", 2);
        cart.set_quantity(2, "40", 0);
        assert!(cart.is_empty());
    }

    #[test]
    fn a_removed_line_can_be_put_back() {
        let cart = Cart::new();
        cart.add(product(6), "One size", 1);
        let removed = cart.remove(6, "One size").expect("the line was there");
        assert!(cart.is_empty());
        cart.restore(removed);
        assert_eq!(cart.count(), 1);
        assert!(cart.remove(99, "M").is_none(), "nothing to remove");
    }

    #[test]
    fn a_chosen_size_is_remembered_per_product() {
        let cart = Cart::new();
        assert_eq!(cart.chosen_size(1), None);
        cart.choose_size(1, "42");
        cart.choose_size(4, "M");
        assert_eq!(cart.chosen_size(1).as_deref(), Some("42"));
        assert_eq!(cart.chosen_size(4).as_deref(), Some("M"));
    }

    #[test]
    fn the_router_hears_only_when_the_cart_becomes_empty() {
        let cart = Cart::new();
        let heard = Rc::new(Cell::new(0));
        let _listening = cart.emptied().add_listener({
            let heard = heard.clone();
            move || heard.set(heard.get() + 1)
        });

        cart.add(product(1), "42", 1);
        cart.add(product(2), "40", 1);
        cart.set_quantity(1, "42", 3);
        assert_eq!(
            heard.get(),
            0,
            "filling the cart changes nothing for the guard"
        );

        cart.remove(1, "42");
        assert_eq!(heard.get(), 0, "one line is left");
        cart.set_quantity(2, "40", 0);
        assert_eq!(heard.get(), 1, "the last line went");

        cart.clear();
        assert_eq!(heard.get(), 1, "already empty: nothing changed");
        cart.add(product(1), "42", 1);
        cart.clear();
        assert_eq!(heard.get(), 2);
    }
}
