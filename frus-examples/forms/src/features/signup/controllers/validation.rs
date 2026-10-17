//! The sign-up rules: what makes the form valid, and how strong a password is.
//!
//! Validation is a **pure function of what was entered**: [`validate`] takes the values and
//! returns a [`Form`] report, which the page reads to decorate its fields. Nothing here
//! draws, so every rule is tested in `tests/validation_test.rs` without a window.

use frus::form::{Form, Rule};

use crate::models::profile::{Date, Plan, Profile, COUNTRIES, INTERESTS};

/// The fields' keys, shared by the report (`form.error(EMAIL)`), the `keyed(..)` wrapper
/// around each field and the focus request that jumps to it (`host::focus(EMAIL)`).
pub mod field {
    pub const NAME: &str = "name";
    pub const EMAIL: &str = "email";
    pub const PASSWORD: &str = "password";
    pub const CONFIRM: &str = "confirm";
    pub const PHONE: &str = "phone";
    pub const BIRTH_DATE: &str = "birth_date";
    pub const COUNTRY: &str = "country";
    pub const INTERESTS: &str = "interests";
    pub const BIO: &str = "bio";
    pub const TERMS: &str = "terms";
}

/// The bio's limit. The field enforces it and counts down to it; the rule is still here
/// because a value can arrive from somewhere other than the keyboard.
pub const BIO_MAX: usize = 160;

/// The youngest age the service accepts.
pub const MINIMUM_AGE: i32 = 13;

/// Everything the form holds, as entered — nothing checked yet.
#[derive(Clone, Debug, Default)]
pub struct SignupInput {
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
pub fn validate(input: &SignupInput, today: Option<Date>) -> Form {
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
pub fn to_profile(input: &SignupInput, today: Option<Date>) -> Option<Profile> {
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
pub fn phone_error(value: &str) -> Option<String> {
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

pub fn birth_date_error(birth_date: Option<Date>, today: Option<Date>) -> Option<String> {
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
pub enum Strength {
    Weak,
    Fair,
    Good,
    Strong,
}

impl Strength {
    pub fn label(self) -> &'static str {
        match self {
            Strength::Weak => "Weak",
            Strength::Fair => "Fair",
            Strength::Good => "Good",
            Strength::Strong => "Strong",
        }
    }

    /// How full the meter under the field is.
    pub fn fraction(self) -> f32 {
        match self {
            Strength::Weak => 0.25,
            Strength::Fair => 0.5,
            Strength::Good => 0.75,
            Strength::Strong => 1.0,
        }
    }
}

/// One point for each of: 8 characters, 12 characters, both cases, a digit, a symbol.
pub fn password_strength(password: &str) -> Strength {
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
