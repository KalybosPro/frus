//! Tests for `navigation::models::product`, moved out of `src/` (milestone 664).

use navigation::models::product::*;

#[test]
fn a_product_is_found_by_its_id() {
    assert_eq!(Product::find(3).map(|p| p.name), Some("Road racer"));
    assert!(Product::find(99).is_none());
}

#[test]
fn a_location_parameter_that_is_not_an_id_finds_nothing() {
    assert_eq!(Product::from_param(Some("2")).map(|p| p.id), Some(2));
    assert!(Product::from_param(Some("abc")).is_none());
    assert!(Product::from_param(Some("-1")).is_none());
    assert!(Product::from_param(None).is_none());
}

#[test]
fn the_rating_is_the_average_of_the_reviews() {
    assert_eq!(Product::find(1).and_then(Product::rating), Some(4.5));
    assert_eq!(Product::find(4).and_then(Product::rating), None);
}

#[test]
fn ids_are_unique() {
    for (i, a) in CATALOG.iter().enumerate() {
        assert!(CATALOG[i + 1..].iter().all(|b| b.id != a.id), "{}", a.id);
    }
}
