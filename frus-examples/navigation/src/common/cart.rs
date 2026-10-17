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
pub const MAX_QUANTITY: u32 = 9;

#[derive(Clone, Default)]
pub struct Cart {
    items: ValueNotifier<Vec<CartItem>>,
    /// The size picked for each product, before it is added. The size page writes it
    /// and pops; the product page reads it. This is how a page **returns a result**.
    sizes: ValueNotifier<HashMap<u32, String>>,
    emptied: ChangeNotifier,
}

impl Cart {
    pub fn new() -> Self {
        Self::default()
    }

    // ---- reading ------------------------------------------------------------------

    pub fn items(&self) -> Vec<CartItem> {
        self.items.get()
    }

    pub fn is_empty(&self) -> bool {
        self.items.with(|items| items.is_empty())
    }

    /// How many articles, counting quantities — the number on the cart's badge.
    pub fn count(&self) -> u32 {
        self.items
            .with(|items| items.iter().map(|item| item.quantity).sum())
    }

    pub fn subtotal_cents(&self) -> u32 {
        self.items
            .with(|items| items.iter().map(CartItem::total_cents).sum())
    }

    pub fn chosen_size(&self, product_id: u32) -> Option<String> {
        self.sizes.with(|sizes| sizes.get(&product_id).cloned())
    }

    // ---- changing -----------------------------------------------------------------

    /// Adds `quantity` of `product` in `size`, to the line already there if there is one.
    pub fn add(&self, product: &Product, size: &str, quantity: u32) {
        self.items.update(
            |items| match items.iter_mut().find(|item| item.is(product.id, size)) {
                Some(item) => item.quantity = (item.quantity + quantity).min(MAX_QUANTITY),
                None => items.push(CartItem::new(product, size, quantity.min(MAX_QUANTITY))),
            },
        );
    }

    /// Sets a line's quantity; zero removes the line.
    pub fn set_quantity(&self, product_id: u32, size: &str, quantity: u32) {
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
    pub fn remove(&self, product_id: u32, size: &str) -> Option<CartItem> {
        let mut removed = None;
        self.change(|items| {
            if let Some(at) = items.iter().position(|item| item.is(product_id, size)) {
                removed = Some(items.remove(at));
            }
        });
        removed
    }

    /// Puts back a line that was removed.
    pub fn restore(&self, item: CartItem) {
        self.items.update(|items| {
            if !items
                .iter()
                .any(|line| line.is(item.product_id, &item.size))
            {
                items.push(item);
            }
        });
    }

    pub fn clear(&self) {
        self.change(Vec::clear);
    }

    pub fn choose_size(&self, product_id: u32, size: &str) {
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
    pub fn emptied(&self) -> &ChangeNotifier {
        &self.emptied
    }
}
