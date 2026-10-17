use frus::form::Form;
use frus::{
    host, keyed, Align, AppBar, AutofillGroup, AutofillHint, Callback, Capitalization, Checkbox,
    Column, Dialog, DropdownMenu, ErrorSummary, Expanded, IconButton, Icons, KeyboardType,
    RadioGroup, Row, Scaffold, State, StateContext, StatefulWidget, StatelessWidget, Switch, Text,
    TextAlign, TextField, TextInputAction, Widget,
};

use crate::common::routes::SUMMARY;
use crate::common::shared::{centered_page, content_width, with_error};
use crate::features::signup::controllers::clock::today;
use crate::features::signup::controllers::signup_service::register;
use crate::features::signup::controllers::validation::{field, BIO_MAX};
use crate::features::signup::states::signupstate::SignupState;
use crate::features::signup::widgets::{
    birth_date_calendar::BirthDateCalendar, form_section::form_section,
    interest_chips::InterestChips, password_strength::PasswordStrengthMeter,
    submit_button::SubmitButton,
};
use crate::models::profile::{Date, Plan, COUNTRIES};

/// The sign-up page's configuration: whether it opens with its errors already showing.
pub struct SignupPage {
    show_errors: bool,
}

impl SignupPage {
    pub fn new(show_errors: bool) -> Self {
        Self { show_errors }
    }
}

impl StatefulWidget for SignupPage {
    type State = SignupState;

    fn create_state(&self) -> SignupState {
        SignupState::new(self.show_errors)
    }
}

impl State for SignupState {
    type Widget = SignupPage;

    /// The whole form, described from the state.
    ///
    /// Validation is not a step that runs somewhere: the report is **computed here, from
    /// the values**, on every build, and each field reads its error from it. A field is
    /// never "marked invalid" and then forgotten — it is invalid exactly while its value is.
    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let today = today();
        let report = self.visible_report(today);
        let error = |key: &str| report.as_ref().and_then(|form| form.error(key));

        let account = form_section(
            cx,
            "Account",
            vec![
                self.name_field(error(field::NAME)),
                self.email_field(error(field::EMAIL)),
                self.password_field(cx, error(field::PASSWORD)),
                self.confirm_field(error(field::CONFIRM)),
            ],
        );
        let about = form_section(
            cx,
            "About you",
            vec![
                self.phone_field(error(field::PHONE)),
                self.birth_date_field(cx, today, error(field::BIRTH_DATE)),
                self.country_field(cx, error(field::COUNTRY)),
                self.bio_field(error(field::BIO)),
            ],
        );
        let preferences = form_section(
            cx,
            "Preferences",
            vec![
                self.plan_field(cx),
                self.interests_field(cx, error(field::INTERESTS)),
                self.newsletter_field(cx),
            ],
        );

        // `Stretch`: every child as wide as the column. (A column centres its children
        // across by default.)
        let mut form = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Stretch)
            .child(
                Text::styled("Create your account", cx.theme().text.headline_small)
                    .align(TextAlign::Center),
            )
            .child(
                Text::new("It takes a minute. Fields marked * are required.")
                    .color(cx.theme().muted)
                    .align(TextAlign::Center),
            );
        // The summary sits at the top, where the eye lands after a failed submit; each line
        // is a link that moves the focus to its field.
        if let Some(summary) = report.as_ref().and_then(error_summary) {
            form = form.child(summary);
        }
        let form = form
            .child(account)
            .child(about)
            .child(preferences)
            .child(self.terms_field(cx, error(field::TERMS)))
            .child(SubmitButton::new(self.submitting, self.submit(cx, today)).into_widget());

        // One `AutofillGroup` around the fields: it tells the platform's password manager
        // that the email and the new password belong to the same account, so it can offer
        // to save them together.
        let body = centered_page(AutofillGroup::new(form));

        let app_bar = AppBar::new()
            .title(Text::new("Sign up"))
            .action(
                IconButton::new(Icons::AUTO_FIX_HIGH)
                    .label("Fill with sample data")
                    .enabled(!self.submitting)
                    .on_press(cx.callback(SignupState::fill_sample)),
            )
            .build();

        let page = Scaffold::new()
            .background(cx.theme().background)
            .app_bar(app_bar)
            .body(body)
            .build();

        // The date-of-birth dialog. Like every overlay here it is **controlled**: it shows
        // while the state says so, and the page underneath is its body.
        let mut dialog = Dialog::new(self.calendar.is_some())
            .on_dismiss(cx.callback(SignupState::close_calendar));
        if let Some(shown) = self.calendar {
            // Built here, with `build(cx)`, rather than handed over as a component with
            // `into_widget()`: the dialog's content is an overlay, and the walk that builds
            // components does not reach overlays yet.
            dialog = dialog.child(
                BirthDateCalendar::new(
                    shown,
                    self.birth_date,
                    today,
                    cx.handler(|s: &mut SignupState, day: u32| s.pick_day(day)),
                    cx.handler(|s: &mut SignupState, months: i32| s.shift_calendar(months)),
                    cx.callback(SignupState::close_calendar),
                )
                .build(cx),
            );
        }
        Box::new(dialog.body(page))
    }
}

/// The error summary, or nothing while every field is valid.
fn error_summary(report: &Form) -> Option<Box<dyn Widget>> {
    let errors = report.errors();
    if errors.is_empty() {
        return None;
    }
    let links = errors
        .into_iter()
        .map(|(key, message)| (message.to_string(), Callback::new(move || host::focus(key))));
    Some(Box::new(ErrorSummary::links(links)))
}

/// The building of each field, one method apiece, so `build` reads as the form's outline.
///
/// Every text field is wrapped in `keyed(field::X, ..)`: the key is what `host::focus`
/// names to move the focus there.
impl SignupState {
    fn name_field(&self, error: Option<&str>) -> Box<dyn Widget> {
        let mut field = TextField::new("")
            .controller(&self.name)
            .label("Full name *")
            .prefix_icon(Icons::PERSON)
            .capitalization(Capitalization::Words)
            .action(TextInputAction::Next)
            .autofill([AutofillHint::Name]);
        if let Some(message) = error {
            field = field.error(message);
        }
        Box::new(keyed(field::NAME, field))
    }

    fn email_field(&self, error: Option<&str>) -> Box<dyn Widget> {
        let mut field = TextField::new("")
            .controller(&self.email)
            .label("Email *")
            .placeholder("name@example.com")
            .prefix_icon(Icons::EMAIL)
            // The keyboard with an @ on it, and a hint the password manager files the
            // account under.
            .keyboard_type(KeyboardType::Email)
            .action(TextInputAction::Next)
            .autofill([AutofillHint::Email, AutofillHint::NewUsername]);
        if let Some(message) = error {
            field = field.error(message);
        }
        Box::new(keyed(field::EMAIL, field))
    }

    fn password_field(&self, cx: &StateContext<Self>, error: Option<&str>) -> Box<dyn Widget> {
        let mut field = TextField::new("")
            .controller(&self.password)
            .label("Password *")
            .helper("At least 8 characters, with letters and numbers")
            .prefix_icon(Icons::LOCK)
            .obscure(!self.show_password)
            // A clickable suffix: the eye shows or hides what was typed.
            .suffix_icon(if self.show_password {
                Icons::VISIBILITY_OFF
            } else {
                Icons::VISIBILITY
            })
            .on_suffix(cx.callback(SignupState::toggle_password))
            .action(TextInputAction::Next)
            // `NewPassword`, not `Password`: the manager offers to generate one and save it.
            .autofill([AutofillHint::NewPassword]);
        if let Some(message) = error {
            field = field.error(message);
        }
        Box::new(
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Stretch)
                .child(keyed(field::PASSWORD, field))
                .child(PasswordStrengthMeter::new(self.password.text()).into_widget()),
        )
    }

    fn confirm_field(&self, error: Option<&str>) -> Box<dyn Widget> {
        let mut field = TextField::new("")
            .controller(&self.confirm)
            .label("Confirm password *")
            .prefix_icon(Icons::LOCK)
            .obscure(!self.show_password)
            .action(TextInputAction::Next)
            .autofill([AutofillHint::NewPassword]);
        if let Some(message) = error {
            field = field.error(message);
        }
        Box::new(keyed(field::CONFIRM, field))
    }

    fn phone_field(&self, error: Option<&str>) -> Box<dyn Widget> {
        let mut field = TextField::new("")
            .controller(&self.phone)
            .label("Phone *")
            .placeholder("+1 555 010 9999")
            .prefix_icon(Icons::PHONE)
            // The phone keypad on a phone; on any platform, letters never get in.
            .keyboard_type(KeyboardType::Phone)
            .input_filter(|c| (c.is_ascii_digit() || " +-().".contains(c)).then_some(c))
            .action(TextInputAction::Next)
            .autofill([AutofillHint::TelephoneNumber]);
        if let Some(message) = error {
            field = field.error(message);
        }
        Box::new(keyed(field::PHONE, field))
    }

    /// A read-only field showing the date, whose calendar icon opens the picker dialog.
    fn birth_date_field(
        &self,
        cx: &StateContext<Self>,
        today: Option<Date>,
        error: Option<&str>,
    ) -> Box<dyn Widget> {
        let shown = self.birth_date.map(|d| d.to_string()).unwrap_or_default();
        let mut field = TextField::new(shown)
            .label("Date of birth *")
            .placeholder("Pick a date")
            .helper("You must be at least 13")
            .read_only()
            .suffix_icon(Icons::CALENDAR_TODAY)
            .on_suffix(cx.callback(move |s: &mut SignupState| s.open_calendar(today)));
        if let Some(message) = error {
            field = field.error(message);
        }
        Box::new(keyed(field::BIRTH_DATE, field))
    }

    /// A dropdown that filters as you type. It is controlled: the state holds whether it is
    /// open, what is typed, and which country is picked.
    fn country_field(&self, cx: &StateContext<Self>, error: Option<&str>) -> Box<dyn Widget> {
        let menu = DropdownMenu::new(
            &self.country_query,
            self.country_open,
            cx.handler(|s: &mut SignupState, query: String| s.set_country_query(query)),
            cx.callback(SignupState::toggle_country_menu),
        )
        .label("Country *")
        .placeholder("Choose a country")
        // A dropdown sizes its list to a width in pixels, so it is told the column's.
        // (32 = the section card's padding on both sides.)
        .width(content_width() - 32.0)
        .max_visible(5)
        .selected(self.country)
        .options(
            &COUNTRIES,
            cx.handler(|s: &mut SignupState, index: usize| s.pick_country(index)),
        );
        with_error(cx, keyed(field::COUNTRY, menu), error)
    }

    /// A multi-line field. `max_length` both enforces the limit and shows the `12/160`
    /// counter under the box.
    fn bio_field(&self, error: Option<&str>) -> Box<dyn Widget> {
        let mut field = TextField::new("")
            .controller(&self.bio)
            .label("Bio")
            .placeholder("A line or two about yourself")
            .rows(3)
            .max_length(BIO_MAX)
            .capitalization(Capitalization::Sentences);
        if let Some(message) = error {
            field = field.error(message);
        }
        Box::new(keyed(field::BIO, field))
    }

    fn plan_field(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let group = Plan::ALL.iter().fold(
            RadioGroup::new(
                self.plan.index(),
                cx.handler(|s: &mut SignupState, index: usize| s.pick_plan(index)),
            ),
            |group, plan| group.option(plan.label()),
        );
        Box::new(
            Column::new()
                .spacing(4.0)
                .cross_axis_alignment(Align::Start)
                .child(Text::styled("Plan", cx.theme().text.label_large))
                .child(group),
        )
    }

    fn interests_field(&self, cx: &StateContext<Self>, error: Option<&str>) -> Box<dyn Widget> {
        let chips = InterestChips::new(
            self.interests.clone(),
            cx.handler(|s: &mut SignupState, index: usize| s.toggle_interest(index)),
        );
        let column = Column::new()
            .spacing(8.0)
            .cross_axis_alignment(Align::Start)
            .child(Text::styled("Interests *", cx.theme().text.label_large))
            .child(chips.into_widget());
        with_error(cx, column, error)
    }

    /// A switch with its label: the whole row reads as one setting.
    fn newsletter_field(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();
        Box::new(
            Row::new()
                .spacing(16.0)
                .cross_axis_alignment(Align::Center)
                .child(Expanded::new(
                    Column::new()
                        .spacing(2.0)
                        .cross_axis_alignment(Align::Start)
                        .child(Text::styled("Newsletter", theme.text.body_large))
                        .child(
                            Text::styled("One email a month, no more", theme.text.body_small)
                                .color(theme.muted),
                        ),
                ))
                .child(
                    Switch::new(self.newsletter).on_toggle(
                        cx.handler(|s: &mut SignupState, on: bool| s.set_newsletter(on)),
                    ),
                ),
        )
    }

    /// The terms checkbox: it turns red, as well as getting an error line, when it is the
    /// one thing missing.
    fn terms_field(&self, cx: &StateContext<Self>, error: Option<&str>) -> Box<dyn Widget> {
        let checkbox = Checkbox::new(self.terms)
            .label("I accept the terms of service *")
            .error(error.is_some())
            .on_toggle(cx.handler(|s: &mut SignupState, on: bool| s.set_terms(on)));
        with_error(cx, keyed(field::TERMS, checkbox), error)
    }

    /// What "Create account" does.
    ///
    /// The press changes the state (errors now show, the button now spins) and, when the
    /// form is valid, starts the request. Its answer arrives later, through a **handle** on
    /// the state: a callback made now could not reach the state from a timer.
    fn submit(&self, cx: &StateContext<Self>, today: Option<Date>) -> Callback {
        let handle = cx.handle();
        let router = cx.router().clone();
        cx.callback(move |state: &mut SignupState| {
            let Some(profile) = state.begin_submit(today) else {
                // Invalid: bring the first field in error into focus.
                if let Some(key) = state
                    .visible_report(today)
                    .and_then(|report| report.first_invalid())
                {
                    host::focus(key);
                }
                return;
            };
            let (handle, router) = (handle.clone(), router.clone());
            register(profile, move |answer| {
                let mut created = None;
                handle.set_state(|state| created = state.finish_submit(answer));
                match created {
                    // `push_with_extra` hands the profile to the next page with the
                    // navigation, no global store needed. Back returns to the filled form.
                    Some(profile) => router.push_with_extra(SUMMARY, profile),
                    None => host::focus(field::EMAIL),
                }
            });
        })
    }
}
