//! Tests for `widgets::features::inputs::widgets::form`, moved out of `src/` (milestone 664).

use widgets::features::inputs::widgets::form::*;

#[test]
fn a_complete_form_is_valid() {
    let form = validate("Ada", "ada@example.com", "secret123", "secret123");
    assert!(form.is_valid());
}

#[test]
fn the_first_failing_rule_gives_the_message() {
    let form = validate("", "ada@", "short", "other");
    assert_eq!(form.error("name"), Some("Enter your name"));
    assert_eq!(form.error("email"), Some("Enter a valid email address"));
    assert_eq!(form.error("password"), Some("At least 8 characters"));
    assert_eq!(form.error("confirm"), Some("The passwords differ"));
    assert_eq!(form.first_invalid(), Some("name"));
}

#[test]
fn a_password_needs_a_digit() {
    let form = validate("Ada", "ada@example.com", "longenough", "longenough");
    assert_eq!(form.error("password"), Some("Include a digit"));
}
