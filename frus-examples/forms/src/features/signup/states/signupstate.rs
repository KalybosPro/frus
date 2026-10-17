//! What the sign-up page keeps between rebuilds, and every way it can change.
//!
//! The text fields' values live in `TextEditingController`s rather than in `String`s: a
//! controller is a field's text held **outside** the field, so code can write into it —
//! "Fill with sample data" does — and the field shows it on the next frame. Everything
//! else is a plain field. No method here draws, so the behaviour is tested in `tests/signupstate_test.rs` with no
//! window.

use frus::form::Form;
use frus::TextEditingController;

use crate::features::signup::controllers::signup_service::SignupError;
use crate::features::signup::controllers::validation::{
    to_profile, validate, SignupInput, BIO_MAX,
};
use crate::models::profile::{Date, Plan, Profile, COUNTRIES, INTERESTS};

pub struct SignupState {
    pub name: TextEditingController,
    pub email: TextEditingController,
    pub password: TextEditingController,
    pub confirm: TextEditingController,
    pub phone: TextEditingController,
    pub bio: TextEditingController,

    pub birth_date: Option<Date>,
    /// An index into `COUNTRIES`.
    pub country: Option<usize>,
    /// What is typed into the country field to filter the list, while it is open.
    pub country_query: String,
    pub country_open: bool,
    pub plan: Plan,
    /// Indices into `INTERESTS`, in the order they were picked.
    pub interests: Vec<usize>,
    pub newsletter: bool,
    pub terms: bool,

    /// The password fields show their text instead of dots.
    pub show_password: bool,
    /// The month the date-of-birth calendar shows, while its dialog is open.
    pub calendar: Option<(i32, u32)>,

    /// Errors are shown once the person has tried to submit — not while they are still
    /// filling the form in for the first time.
    pub submitted: bool,
    /// A request is in flight: the button shows a spinner and does not take a second press.
    pub submitting: bool,
    /// An address the server turned down; see `SignupInput::taken_email`.
    pub taken_email: Option<String>,
}

impl SignupState {
    /// `show_errors` opens the form as if a submit had already failed — see the router.
    pub fn new(show_errors: bool) -> Self {
        Self {
            name: TextEditingController::new(),
            email: TextEditingController::new(),
            password: TextEditingController::new(),
            confirm: TextEditingController::new(),
            phone: TextEditingController::new(),
            bio: TextEditingController::new(),
            birth_date: None,
            country: None,
            country_query: String::new(),
            country_open: false,
            plan: Plan::Free,
            interests: Vec::new(),
            newsletter: false,
            terms: false,
            show_password: false,
            calendar: None,
            submitted: show_errors,
            submitting: false,
            taken_email: None,
        }
    }

    /// A snapshot of every value, for the rules to check.
    pub fn input(&self) -> SignupInput {
        SignupInput {
            name: self.name.text(),
            email: self.email.text(),
            password: self.password.text(),
            confirm: self.confirm.text(),
            phone: self.phone.text(),
            birth_date: self.birth_date,
            country: self.country,
            plan: self.plan,
            interests: self.interests.clone(),
            newsletter: self.newsletter,
            bio: self.bio.text(),
            terms: self.terms,
            taken_email: self.taken_email.clone(),
        }
    }

    /// The validation report, once errors are showing (after the first submit); `None`
    /// before that, so no field is decorated while the form is still being filled in.
    pub fn visible_report(&self, today: Option<Date>) -> Option<Form> {
        self.submitted.then(|| validate(&self.input(), today))
    }

    // --- Editing -------------------------------------------------------------------------

    pub fn toggle_password(&mut self) {
        self.show_password = !self.show_password;
    }

    pub fn toggle_country_menu(&mut self) {
        self.country_open = !self.country_open;
        self.country_query.clear();
    }

    pub fn set_country_query(&mut self, query: String) {
        self.country_query = query;
        self.country_open = true;
    }

    /// A pick closes the menu: the dropdown is **controlled**, so nothing closes it unless
    /// the state says so.
    pub fn pick_country(&mut self, index: usize) {
        self.country = Some(index);
        self.country_open = false;
        self.country_query.clear();
    }

    pub fn pick_plan(&mut self, index: usize) {
        self.plan = Plan::from_index(index);
    }

    pub fn toggle_interest(&mut self, index: usize) {
        if let Some(at) = self.interests.iter().position(|&i| i == index) {
            self.interests.remove(at);
        } else if index < INTERESTS.len() {
            self.interests.push(index);
        }
    }

    pub fn set_newsletter(&mut self, on: bool) {
        self.newsletter = on;
    }

    pub fn set_terms(&mut self, accepted: bool) {
        self.terms = accepted;
    }

    // --- The date-of-birth dialog ----------------------------------------------------------

    /// Opens the calendar on the chosen date's month, or twenty years back from `today`.
    pub fn open_calendar(&mut self, today: Option<Date>) {
        let shown = match (self.birth_date, today) {
            (Some(date), _) => (date.year, date.month),
            (None, Some(today)) => (today.year - 20, today.month),
            (None, None) => (2000, 1),
        };
        self.calendar = Some(shown);
    }

    pub fn close_calendar(&mut self) {
        self.calendar = None;
    }

    /// Moves the calendar by `months`, across year boundaries.
    pub fn shift_calendar(&mut self, months: i32) {
        if let Some((year, month)) = self.calendar {
            let index = year * 12 + (month as i32 - 1) + months;
            self.calendar = Some((index.div_euclid(12), index.rem_euclid(12) as u32 + 1));
        }
    }

    /// A day picked in the calendar: that is the date, and the dialog closes.
    pub fn pick_day(&mut self, day: u32) {
        if let Some((year, month)) = self.calendar.take() {
            self.birth_date = Some(Date::new(year, month, day));
        }
    }

    // --- Sample data -----------------------------------------------------------------------

    /// Writes a complete, valid profile into the form. The text goes through the
    /// controllers, so the fields show it at once.
    pub fn fill_sample(&mut self) {
        let sample = Profile::sample();
        let password = "Analytical-Engine-1843";
        self.name.set_text(sample.full_name);
        self.email.set_text(sample.email);
        self.password.set_text(password);
        self.confirm.set_text(password);
        self.phone.set_text(sample.phone);
        self.bio
            .set_text(sample.bio.chars().take(BIO_MAX).collect::<String>());
        self.birth_date = Some(sample.birth_date);
        self.country = COUNTRIES.iter().position(|&c| c == sample.country);
        self.plan = sample.plan;
        self.interests = sample
            .interests
            .iter()
            .filter_map(|name| INTERESTS.iter().position(|i| i == name))
            .collect();
        self.newsletter = sample.newsletter;
        self.terms = true;
        self.taken_email = None;
    }

    // --- Submitting ------------------------------------------------------------------------

    /// The press on "Create account". From now on errors show; if the form is valid, the
    /// request starts and the profile to send comes back.
    pub fn begin_submit(&mut self, today: Option<Date>) -> Option<Profile> {
        if self.submitting {
            return None;
        }
        self.submitted = true;
        let profile = to_profile(&self.input(), today)?;
        self.submitting = true;
        Some(profile)
    }

    /// The server's answer. A created account comes back to be shown; a refusal becomes an
    /// error on the field it is about.
    pub fn finish_submit(&mut self, answer: Result<Profile, SignupError>) -> Option<Profile> {
        self.submitting = false;
        match answer {
            Ok(profile) => Some(profile),
            Err(SignupError::EmailTaken(email)) => {
                self.taken_email = Some(email);
                None
            }
        }
    }
}
