//! Tests for `forms::features::signup::controllers::validation`, moved out of `src/` (milestone 662).

use forms::features::signup::controllers::validation::*;
use forms::models::profile::{Date, Plan};

const TODAY: Option<Date> = Some(Date::new(2026, 10, 15));

/// An input every rule accepts; each test breaks one thing.
fn valid() -> SignupInput {
    SignupInput {
        name: "Ada Lovelace".into(),
        email: "ada@example.com".into(),
        password: "analytical1".into(),
        confirm: "analytical1".into(),
        phone: "+44 20 7946 0958".into(),
        birth_date: Some(Date::new(1995, 12, 10)),
        country: Some(3),
        plan: Plan::Pro,
        interests: vec![0],
        newsletter: false,
        bio: String::new(),
        terms: true,
        taken_email: None,
    }
}

#[test]
fn a_complete_input_is_valid() {
    let report = validate(&valid(), TODAY);
    assert!(report.is_valid(), "{:?}", report.errors());
    let profile = to_profile(&valid(), TODAY).expect("valid input makes a profile");
    assert_eq!(profile.country, "France");
    assert_eq!(profile.interests, vec!["Design".to_string()]);
}

#[test]
fn an_empty_form_reports_every_required_field_from_the_top() {
    let report = validate(&SignupInput::default(), TODAY);
    assert_eq!(report.first_invalid(), Some(field::NAME));
    for key in [
        field::NAME,
        field::EMAIL,
        field::PASSWORD,
        field::CONFIRM,
        field::PHONE,
        field::BIRTH_DATE,
        field::COUNTRY,
        field::INTERESTS,
        field::TERMS,
    ] {
        assert!(report.error(key).is_some(), "{key} should be in error");
    }
    // The bio is optional.
    assert_eq!(report.error(field::BIO), None);
    assert!(to_profile(&SignupInput::default(), TODAY).is_none());
}

#[test]
fn email_must_look_like_an_address() {
    for good in ["ada@example.com", "a.b@mail.example.co"] {
        let input = SignupInput {
            email: good.into(),
            ..valid()
        };
        assert_eq!(validate(&input, TODAY).error(field::EMAIL), None, "{good}");
    }
    for bad in ["ada", "ada@", "@example.com", "ada@example"] {
        let input = SignupInput {
            email: bad.into(),
            ..valid()
        };
        assert!(
            validate(&input, TODAY).error(field::EMAIL).is_some(),
            "{bad}"
        );
    }
}

#[test]
fn a_taken_email_is_an_error_until_it_changes() {
    let input = SignupInput {
        taken_email: Some("ada@example.com".into()),
        ..valid()
    };
    assert_eq!(
        validate(&input, TODAY).error(field::EMAIL),
        Some("An account already uses this address")
    );
    let changed = SignupInput {
        email: "ada.l@example.com".into(),
        ..input
    };
    assert_eq!(validate(&changed, TODAY).error(field::EMAIL), None);
}

#[test]
fn the_confirmation_must_match_the_password() {
    let input = SignupInput {
        confirm: "analytical2".into(),
        ..valid()
    };
    assert_eq!(
        validate(&input, TODAY).error(field::CONFIRM),
        Some("The passwords do not match")
    );
    let empty = SignupInput {
        confirm: String::new(),
        ..valid()
    };
    assert_eq!(
        validate(&empty, TODAY).error(field::CONFIRM),
        Some("Type your password again")
    );
}

#[test]
fn a_password_needs_length_letters_and_digits() {
    let check = |password: &str| {
        let input = SignupInput {
            password: password.into(),
            confirm: password.into(),
            ..valid()
        };
        validate(&input, TODAY)
            .error(field::PASSWORD)
            .map(str::to_string)
    };
    assert_eq!(check("abc1"), Some("Use at least 8 characters".into()));
    assert_eq!(check("abcdefgh"), Some("Mix letters and numbers".into()));
    assert_eq!(check("abcdefg1"), None);
}

#[test]
fn terms_must_be_accepted() {
    let input = SignupInput {
        terms: false,
        ..valid()
    };
    let report = validate(&input, TODAY);
    assert_eq!(
        report.error(field::TERMS),
        Some("Accept the terms to create an account")
    );
    assert_eq!(report.first_invalid(), Some(field::TERMS));
}

#[test]
fn phone_numbers_take_digits_and_punctuation() {
    assert_eq!(phone_error("+1 (555) 010-9999"), None);
    assert!(phone_error("12345").is_some(), "too few digits");
    assert!(phone_error("call me").is_some(), "letters");
}

#[test]
fn the_birth_date_must_be_past_and_old_enough() {
    let at = |date| birth_date_error(Some(date), TODAY);
    assert_eq!(at(Date::new(2013, 10, 15)), None, "13 today");
    assert!(at(Date::new(2013, 10, 16)).is_some(), "13 tomorrow");
    assert!(at(Date::new(2030, 1, 1)).is_some(), "in the future");
    assert!(birth_date_error(None, TODAY).is_some(), "missing");
    // Without a clock, any date is accepted rather than every date refused.
    assert_eq!(birth_date_error(Some(Date::new(2030, 1, 1)), None), None);
}

#[test]
fn the_bio_has_a_limit() {
    let input = SignupInput {
        bio: "x".repeat(BIO_MAX + 1),
        ..valid()
    };
    assert!(validate(&input, TODAY).error(field::BIO).is_some());
}

#[test]
fn strength_grows_with_variety_and_length() {
    assert_eq!(password_strength(""), Strength::Weak);
    assert_eq!(password_strength("abcdefgh"), Strength::Weak);
    assert_eq!(password_strength("abcdefg1"), Strength::Fair);
    assert_eq!(password_strength("Abcdefg1"), Strength::Good);
    assert_eq!(password_strength("Abcdefghijk1!"), Strength::Strong);
}
