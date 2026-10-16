//! The sign-up rules: what makes the form valid, and how strong a password is.
//!
//! Validation is a **pure function of what was entered**: [`validate`] takes the values and
//! returns a [`Form`] report, which the page reads to decorate its fields. Nothing here
//! draws, so every rule is tested below without a window.

use frus::form::{Form, Rule};

use crate::models::profile::{Date, Plan, Profile, COUNTRIES, INTERESTS};

/// The fields' keys, shared by the report (`form.error(EMAIL)`), the `keyed(..)` wrapper
/// around each field and the focus request that jumps to it (`host::focus(EMAIL)`).
pub(crate) mod field {
    pub(crate) const NAME: &str = "name";
    pub(crate) const EMAIL: &str = "email";
    pub(crate) const PASSWORD: &str = "password";
    pub(crate) const CONFIRM: &str = "confirm";
    pub(crate) const PHONE: &str = "phone";
    pub(crate) const BIRTH_DATE: &str = "birth_date";
    pub(crate) const COUNTRY: &str = "country";
    pub(crate) const INTERESTS: &str = "interests";
    pub(crate) const BIO: &str = "bio";
    pub(crate) const TERMS: &str = "terms";
}

/// The bio's limit. The field enforces it and counts down to it; the rule is still here
/// because a value can arrive from somewhere other than the keyboard.
pub(crate) const BIO_MAX: usize = 160;

/// The youngest age the service accepts.
pub(crate) const MINIMUM_AGE: i32 = 13;

/// Everything the form holds, as entered — nothing checked yet.
#[derive(Clone, Debug, Default)]
pub(crate) struct SignupInput {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm: String,
    pub phone: String,
    pub birth_date: Option<Date>,
    /// An index into [`COUNTRIES`].
    pub country: Option<usize>,
    pub plan: Plan,
    /// Indices into [`INTERESTS`], in the order they were picked.
    pub interests: Vec<usize>,
    pub newsletter: bool,
    pub bio: String,
    pub terms: bool,
    /// An address the server already turned down. A rule like any other, so the error
    /// shows under the field and in the summary, and goes away once the address changes.
    pub taken_email: Option<String>,
}

/// Checks every field, in the order the page shows them — so `first_invalid()` is the
/// first field in error from the top.
///
/// `today` bounds the date of birth; `None` (no clock to ask) only skips the age check.
pub(crate) fn validate(input: &SignupInput, today: Option<Date>) -> Form {
    let taken = input.taken_email.clone();
    // A rule sees one string. Facts that are not text — a date, a choice, a ticked box — are
    // decided here first, and handed to `Rule::new` as the answer.
    let birth_error = birth_date_error(input.birth_date, today);
    let interests_error = input
        .interests
        .is_empty()
        .then(|| "Pick at least one interest".to_string());
    let birth_text = input.birth_date.map(|d| d.to_string()).unwrap_or_default();
    let country_text = input
        .country
        .and_then(|i| COUNTRIES.get(i))
        .copied()
        .unwrap_or("");

    Form::new()
        .field(
            field::NAME,
            &input.name,
            Rule::all([
                Rule::required("Enter your full name"),
                Rule::min_len(2, "That name looks too short"),
                Rule::max_len(80, "Keep it under 80 characters"),
            ]),
        )
        .field(
            field::EMAIL,
            &input.email,
            Rule::all([
                Rule::required("Enter your email address"),
                Rule::email("Enter a valid email address, like name@example.com"),
                Rule::new(move |value| {
                    let value = value.trim();
                    taken
                        .as_deref()
                        .filter(|taken| taken.eq_ignore_ascii_case(value))
                        .map(|_| "An account already uses this address".to_string())
                }),
            ]),
        )
        .field(
            field::PASSWORD,
            &input.password,
            Rule::all([
                Rule::required("Choose a password"),
                Rule::min_len(8, "Use at least 8 characters"),
                Rule::new(|value| {
                    let letter = value.chars().any(char::is_alphabetic);
                    let digit = value.chars().any(|c| c.is_ascii_digit());
                    (!(letter && digit)).then(|| "Mix letters and numbers".to_string())
                }),
            ]),
        )
        // A **cross-field** rule: `field_with` hands the check the fields declared so far,
        // so it can read the password declared just above.
        .field_with(field::CONFIRM, &input.confirm, |value, form| {
            if value.is_empty() {
                Some("Type your password again".to_string())
            } else if form.value(field::PASSWORD) != Some(value) {
                Some("The passwords do not match".to_string())
            } else {
                None
            }
        })
        .field(
            field::PHONE,
            &input.phone,
            Rule::all([
                Rule::required("Enter a phone number"),
                Rule::new(phone_error),
            ]),
        )
        .field(
            field::BIRTH_DATE,
            &birth_text,
            Rule::new(move |_| birth_error.clone()),
        )
        .field(
            field::COUNTRY,
            country_text,
            Rule::required("Choose your country"),
        )
        .field(
            field::INTERESTS,
            "",
            Rule::new(move |_| interests_error.clone()),
        )
        .field(
            field::BIO,
            &input.bio,
            Rule::max_len(BIO_MAX, format!("Keep it under {BIO_MAX} characters")),
        )
        // A ticked box is the one value `required` needs: "yes", or nothing.
        .field(
            field::TERMS,
            if input.terms { "yes" } else { "" },
            Rule::required("Accept the terms to create an account"),
        )
}

/// A valid submission as a [`Profile`]; `None` while any rule fails.
pub(crate) fn to_profile(input: &SignupInput, today: Option<Date>) -> Option<Profile> {
    if !validate(input, today).is_valid() {
        return None;
    }
    Some(Profile {
        full_name: input.name.trim().to_string(),
        email: input.email.trim().to_string(),
        phone: input.phone.trim().to_string(),
        birth_date: input.birth_date?,
        country: COUNTRIES.get(input.country?)?.to_string(),
        plan: input.plan,
        interests: input
            .interests
            .iter()
            .filter_map(|&i| INTERESTS.get(i))
            .map(|s| s.to_string())
            .collect(),
        newsletter: input.newsletter,
        bio: input.bio.trim().to_string(),
    })
}

/// A phone number: digits, with the punctuation people write them with, and between 7 and
/// 15 digits (the longest an international number can be).
fn phone_error(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        return None; // `required` speaks for an empty field.
    }
    let allowed = |c: char| c.is_ascii_digit() || " +-().".contains(c);
    if !value.chars().all(allowed) {
        return Some("Use digits, spaces and + - ( ) only".to_string());
    }
    let digits = value.chars().filter(char::is_ascii_digit).count();
    (!(7..=15).contains(&digits)).then(|| "A phone number has 7 to 15 digits".to_string())
}

fn birth_date_error(birth_date: Option<Date>, today: Option<Date>) -> Option<String> {
    let Some(birth) = birth_date else {
        return Some("Pick your date of birth".to_string());
    };
    let today = today?;
    if birth > today {
        Some("That date is in the future".to_string())
    } else if birth.years_until(today) < MINIMUM_AGE {
        Some(format!("You must be at least {MINIMUM_AGE} to sign up"))
    } else {
        None
    }
}

/// How hard a password would be to guess, roughly — a hint for the person typing, not a
/// rule: the rules above decide what is accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Strength {
    Weak,
    Fair,
    Good,
    Strong,
}

impl Strength {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Strength::Weak => "Weak",
            Strength::Fair => "Fair",
            Strength::Good => "Good",
            Strength::Strong => "Strong",
        }
    }

    /// How full the meter under the field is.
    pub(crate) fn fraction(self) -> f32 {
        match self {
            Strength::Weak => 0.25,
            Strength::Fair => 0.5,
            Strength::Good => 0.75,
            Strength::Strong => 1.0,
        }
    }
}

/// One point for each of: 8 characters, 12 characters, both cases, a digit, a symbol.
pub(crate) fn password_strength(password: &str) -> Strength {
    let length = password.chars().count();
    let checks = [
        length >= 8,
        length >= 12,
        password.chars().any(char::is_lowercase) && password.chars().any(char::is_uppercase),
        password.chars().any(|c| c.is_ascii_digit()),
        password
            .chars()
            .any(|c| !c.is_alphanumeric() && !c.is_whitespace()),
    ];
    match checks.iter().filter(|&&passed| passed).count() {
        0 | 1 => Strength::Weak,
        2 => Strength::Fair,
        3 | 4 => Strength::Good,
        _ => Strength::Strong,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
