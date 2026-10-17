//! Tests for `forms::features::signup::states::signupstate`, moved out of `src/` (milestone 662).

use forms::features::signup::controllers::signup_service::SignupError;
use forms::features::signup::controllers::validation::field;
use forms::features::signup::states::signupstate::*;
use forms::models::profile::{Date, Profile};

const TODAY: Option<Date> = Some(Date::new(2026, 10, 15));

/// What the page would show under field `key`.
fn shown(state: &SignupState, key: &str) -> Option<String> {
    state
        .visible_report(TODAY)
        .and_then(|report| report.error(key).map(str::to_string))
}

#[test]
fn errors_stay_hidden_until_the_first_submit() {
    let mut state = SignupState::new(false);
    assert_eq!(shown(&state, field::EMAIL), None);

    assert_eq!(state.begin_submit(TODAY), None, "an empty form is not sent");
    assert!(!state.submitting);
    assert_eq!(
        shown(&state, field::EMAIL).as_deref(),
        Some("Enter your email address")
    );
}

#[test]
fn sample_data_makes_a_valid_form_that_submits_once() {
    let mut state = SignupState::new(false);
    state.fill_sample();
    let profile = state.begin_submit(TODAY).expect("the sample is valid");
    assert_eq!(profile, Profile::sample());
    assert!(state.submitting);
    assert_eq!(
        state.begin_submit(TODAY),
        None,
        "no second request in flight"
    );
}

#[test]
fn a_taken_email_lands_on_the_email_field() {
    let mut state = SignupState::new(false);
    state.fill_sample();
    state.email.set_text("taken@example.com");
    let profile = state.begin_submit(TODAY).expect("valid on the client");
    let answer = state.finish_submit(Err(SignupError::EmailTaken(profile.email)));
    assert_eq!(answer, None);
    assert!(!state.submitting);
    assert_eq!(
        shown(&state, field::EMAIL).as_deref(),
        Some("An account already uses this address")
    );
    // Changing the address clears it.
    state.email.set_text("ada@example.org");
    assert_eq!(shown(&state, field::EMAIL), None);
}

#[test]
fn the_calendar_walks_across_years_and_closes_on_a_pick() {
    let mut state = SignupState::new(false);
    state.open_calendar(TODAY);
    assert_eq!(state.calendar, Some((2006, 10)));
    state.shift_calendar(3);
    assert_eq!(state.calendar, Some((2007, 1)));
    state.shift_calendar(-12);
    assert_eq!(state.calendar, Some((2006, 1)));
    state.pick_day(31);
    assert_eq!(state.birth_date, Some(Date::new(2006, 1, 31)));
    assert_eq!(state.calendar, None);
}

#[test]
fn interests_toggle_and_countries_close_their_menu() {
    let mut state = SignupState::new(false);
    state.toggle_interest(2);
    state.toggle_interest(0);
    assert_eq!(state.interests, vec![2, 0]);
    state.toggle_interest(2);
    assert_eq!(state.interests, vec![0]);

    state.toggle_country_menu();
    state.set_country_query("ger".into());
    state.pick_country(4);
    assert_eq!(state.country, Some(4));
    assert!(!state.country_open);
    assert!(state.country_query.is_empty());
}
