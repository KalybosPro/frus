//! Tests for `forms::models::profile`, moved out of `src/` (milestone 664).

use forms::models::profile::*;

#[test]
fn an_age_counts_only_birthdays_already_passed() {
    let birth = Date::new(2000, 6, 15);
    assert_eq!(birth.years_until(Date::new(2018, 6, 14)), 17);
    assert_eq!(birth.years_until(Date::new(2018, 6, 15)), 18);
    assert_eq!(birth.years_until(Date::new(2018, 12, 1)), 18);
}

#[test]
fn dates_print_and_compare_in_calendar_order() {
    assert_eq!(Date::new(2001, 4, 9).to_string(), "2001-04-09");
    assert!(Date::new(2001, 4, 9) < Date::new(2001, 10, 1));
}

#[test]
fn a_plan_round_trips_through_its_index() {
    for plan in Plan::ALL {
        assert_eq!(Plan::from_index(plan.index()), plan);
    }
    assert_eq!(Plan::from_index(99), Plan::Free);
}
