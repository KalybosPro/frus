//! Tests for `navigation::common::cart`, moved out of `src/` (milestone 662).

use navigation::models::product::Product;
use std::cell::Cell;
use std::rc::Rc;

use frus::Listenable;

use navigation::common::cart::*;

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
