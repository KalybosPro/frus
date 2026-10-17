//! Tests for `navigation::common::shared`, moved out of `src/` (milestone 664).

use navigation::common::shared::*;

#[test]
fn prices_are_written_in_dollars_and_cents() {
    assert_eq!(format_price(0), "$0.00");
    assert_eq!(format_price(5), "$0.05");
    assert_eq!(format_price(8900), "$89.00");
    assert_eq!(format_price(123456), "$1234.56");
}
