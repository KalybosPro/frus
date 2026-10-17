//! Tests for `http::features::posts::widgets::post_tile`, moved out of `src/` (milestone 664).

use http::features::posts::widgets::post_tile::*;

#[test]
fn capitalize_touches_only_the_first_letter() {
    assert_eq!(capitalize("sunt aut facere"), "Sunt aut facere");
    assert_eq!(capitalize(""), "");
    assert_eq!(capitalize("éclair"), "Éclair");
}
