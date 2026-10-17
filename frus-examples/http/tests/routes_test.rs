//! Tests for `http::common::routes`, moved out of `src/` (milestone 664).

use http::common::routes::*;

#[test]
fn locations_are_full_addresses() {
    assert_eq!(new_post_location(), "/posts/new");
    assert_eq!(post_location(42), "/posts/42");
}
