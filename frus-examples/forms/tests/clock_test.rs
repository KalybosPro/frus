//! Tests for `forms::features::signup::controllers::clock`, moved out of `src/` (milestone 662).

use forms::features::signup::controllers::clock::*;
use forms::models::profile::Date;

#[test]
fn days_since_the_epoch_become_dates() {
    assert_eq!(civil_from_days(0), Date::new(1970, 1, 1));
    assert_eq!(civil_from_days(11_016), Date::new(2000, 2, 29));
    assert_eq!(civil_from_days(20_741), Date::new(2026, 10, 15));
}
