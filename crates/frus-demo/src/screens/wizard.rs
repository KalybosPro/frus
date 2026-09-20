//! The multi-step form screen: validation per step, and the wizard's own
//! navigation.
//!
//! The four fields are held by `TextEditingController`s the screen keeps — one form across the
//! steps, so what was typed on the first is still there on the last — and the rest of what it
//! remembers, which step it is on and whether the errors show yet, is its `State`.

use crate::prelude::*;
use frus_widgets::host;
use frus_widgets::{column, TextEditingController};

/// The wizard's form: a **pure** validation of the current state (milestones 180–181). The order
/// declares `password` before `confirm` (the cross-field `matches` validation).
pub(crate) fn wizard_form<'a>(
    name: &'a str,
    email: &'a str,
    password: &'a str,
    confirm: &'a str,
) -> Form {
    Form::new()
        .field("name", name, Rule::required("Name is required"))
        .field(
            "email",
            email,
            Rule::all([
                Rule::required("Email is required"),
                Rule::email("Enter a valid email address"),
            ]),
        )
        .field(
            "password",
            password,
            Rule::min_len(8, "Password must be at least 8 characters"),
        )
        .matches("confirm", confirm, "password", "Passwords do not match")
}

/// Which step (0 = Account, 1 = Security) the field `key` lives on — so that clicking an error
/// summary bullet jumps to the right step (milestones 181 + 183).
pub(crate) fn wizard_step_of(key: &str) -> usize {
    match key {
        "name" | "email" => 0,
        _ => 1,
    }
}

/// A wizard field's index (its focus key) — for `keyed` and the host's focus request.
pub(crate) fn wizard_field_of(key: &str) -> u8 {
    match key {
        "name" => 0,
        "email" => 1,
        "password" => 2,
        _ => 3,
    }
}

/// Is step `step` **valid**? (so "Next" is only allowed once the step is filled in.)
pub(crate) fn wizard_step_valid(form: &Form, step: usize) -> bool {
    match step {
        0 => form.error("name").is_none() && form.error("email").is_none(),
        1 => form.error("password").is_none() && form.error("confirm").is_none(),
        _ => form.is_valid(),
    }
}

/// What each wizard field is for, so that a password manager can fill the form and save the
/// account it creates (milestone 512). The email address is the account's name as well,
/// and both passwords are new ones: offered for saving, never filled from what is saved.
pub(crate) fn wizard_hints(field: u8) -> &'static [frus_widgets::AutofillHint] {
    use frus_widgets::AutofillHint as Hint;
    match field {
        0 => &[Hint::Name],
        1 => &[Hint::Username, Hint::Email],
        _ => &[Hint::NewPassword],
    }
}

/// Which keyboard each wizard field opens (milestone 514). Nothing said it before, so every
/// field opened a sentence keyboard: the email address came back with a capital and a space
/// after each suggestion taken. A password shown in the clear still asks for a secret's
/// keyboard — revealing it is for the reader to check it, not for the keyboard to learn it.
pub(crate) fn wizard_keyboard(field: u8, obscure: bool) -> frus_widgets::KeyboardType {
    use frus_widgets::KeyboardType as Keys;
    match field {
        0 => Keys::Name,
        1 => Keys::Email,
        _ if obscure => Keys::Password,
        _ => Keys::VisiblePassword,
    }
}

/// One wizard field: its error is shown **only after** submission, its value is **masked** for a
/// password, and it carries a **focus key** (`keyed`) so the summary can jump to it.
// Nine, and they are the field's whole description. A struct here would be a
// parameter list wearing a hat.
#[allow(clippy::too_many_arguments)]
pub(crate) fn wizard_input(
    form: &Form,
    submitted: bool,
    label: &str,
    value: &TextEditingController,
    key: &str,
    field: u8,
    obscure: bool,
    eye: Option<(bool, Callback)>,
) -> impl Widget + 'static {
    let mut input = TextField::new(value.text())
        .size(16.0)
        .label(label)
        .obscure(obscure)
        .keyboard_type(wizard_keyboard(field, obscure))
        .controller(value)
        .autofill(wizard_hints(field).iter().copied());
    // `eye = Some(revealed)`: an eye icon **inside the field** toggles the masking (milestone 198).
    if let Some((revealed, toggle)) = eye {
        let icon = if revealed {
            Icons::VISIBILITY_OFF
        } else {
            Icons::VISIBILITY
        };
        input = input.suffix_icon(icon).on_suffix(toggle);
    }
    if submitted {
        if let Some(err) = form.error(key) {
            input = input.error(err);
        }
    }
    // A **ceiling**, not a measured width: the field fills the column it is in and stops
    // at 360, so a line of input never stretches across a desktop. It used to be
    // `(width - 48.0).clamp(240.0, 360.0)` — the screen's width less the padding, counted
    // by hand, which is exactly the arithmetic milestone 392 went looking for.
    ConstrainedBox::new(keyed(("wizard", field), input)).max_width(360.0)
}

/// The **sign-up wizard** screen: proof that the recent building blocks fit together — a
/// clickable [`Steps`] indicator (milestone 183), a validated [`Form`] (180) with a **clickable**
/// error summary (181), and a success notification (185/188).
pub(crate) struct WizardPage {
    pub(crate) demo: Rc<Demo>,
}

/// What the wizard keeps besides its fields.
#[derive(Default)]
pub(crate) struct WizardState {
    /// The wizard's current step (0 = Account, 1 = Security, 2 = Review).
    pub(crate) step: usize,
    /// Has the wizard been submitted at least once? (the errors only show afterwards.)
    pub(crate) submitted: bool,
    /// Are the wizard's passwords **revealed** (unmasked)?
    pub(crate) reveal: bool,
}

impl WizardState {
    /// Jumps to a step (a `Steps` marker was clicked).
    pub(crate) fn go_to(&mut self, step: usize) {
        self.step = step.min(2);
    }

    /// The previous step.
    pub(crate) fn back(&mut self) {
        self.step = self.step.saturating_sub(1);
    }

    /// The next step.
    pub(crate) fn next(&mut self) {
        self.step = (self.step + 1).min(2);
    }

    /// Submits: valid, the wizard starts over (the caller clears the fields and says so);
    /// otherwise the errors show and the review step lists them. Whether it went through.
    pub(crate) fn submit(&mut self, valid: bool) -> bool {
        if valid {
            self.step = 0;
            self.submitted = false;
        } else {
            self.submitted = true;
            self.step = 2;
        }
        valid
    }
}

impl StatefulWidget for WizardPage {
    type State = WizardState;

    fn create_state(&self) -> WizardState {
        WizardState::default()
    }
}

impl State for WizardState {
    type Widget = WizardPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme().clone();
        let demo = cx.widget().demo.clone();
        // One controller per field, kept for as long as the screen is: it is one form across
        // its steps.
        let name = cx.use_text_controller("");
        let email = cx.use_text_controller("");
        let pass = cx.use_text_controller("");
        let confirm = cx.use_text_controller("");
        let (name_text, email_text, pass_text, confirm_text) =
            (name.text(), email.text(), pass.text(), confirm.text());
        let form = wizard_form(&name_text, &email_text, &pass_text, &confirm_text);
        let submitted = self.submitted;
        // Steps are marked "done" by **validity** (milestone 195), not merely by position — which
        // matches "Next" being gated by that same validity.
        let steps = Steps::new(["Account", "Security", "Review"])
            .current(self.step)
            .completed([
                wizard_step_valid(&form, 0),
                wizard_step_valid(&form, 1),
                form.is_valid(),
            ])
            .on_tap(cx.handler(|s, step: usize| s.go_to(step)));

        // The current step's content.
        let content: Box<dyn Widget> = match self.step {
            0 => Box::new(
                Flex::column()
                    .gap(14.0)
                    .child(wizard_input(
                        &form,
                        submitted,
                        "Full name",
                        &name,
                        "name",
                        0,
                        false,
                        None,
                    ))
                    .child(wizard_input(
                        &form, submitted, "Email", &email, "email", 1, false, None,
                    )),
            ),
            1 => {
                // Passwords are masked unless revealed: the eye icon **inside the field**
                // toggles it (198).
                let obscure = !self.reveal;
                let eye = || Some((self.reveal, cx.callback(|s| s.reveal = !s.reveal)));
                Box::new(
                    Flex::column()
                        .gap(14.0)
                        .child(wizard_input(
                            &form,
                            submitted,
                            "Password",
                            &pass,
                            "password",
                            2,
                            obscure,
                            eye(),
                        ))
                        .child(wizard_input(
                            &form,
                            submitted,
                            "Confirm password",
                            &confirm,
                            "confirm",
                            3,
                            obscure,
                            eye(),
                        )),
                )
            }
            _ => {
                let mut review = Flex::column().gap(14.0);
                // A clickable summary: each bullet jumps to the faulty field's step **and**
                // focuses it (milestones 181 + 183 + programmatic focus).
                if submitted && !form.is_valid() {
                    let links = form.errors().into_iter().map(|(key, message)| {
                        let (step, field) = (wizard_step_of(key), wizard_field_of(key));
                        let jump = cx.callback(move |s| {
                            s.go_to(step);
                            host::focus(("wizard", field));
                        });
                        (message.to_string(), jump)
                    });
                    review = review.child(ErrorSummary::links(links));
                }
                review = review.child(
                    text(format!(
                        "Creating account for {} <{}>",
                        if name_text.is_empty() {
                            "—"
                        } else {
                            name_text.as_str()
                        },
                        if email_text.is_empty() {
                            "—"
                        } else {
                            email_text.as_str()
                        },
                    ))
                    .size(16.0)
                    .wrap(),
                );
                Box::new(review)
            }
        };

        // The navigation bar: Back / Next, or Create on the last step.
        let mut nav = Flex::row().gap(12.0);
        if self.step > 0 {
            nav = nav.child(
                button("Back", cx.callback(|s| s.back()))
                    .variant(Variant::Outlined)
                    .size(16.0),
            );
        }
        if self.step < 2 {
            // "Next" only becomes active once the current step is valid (milestone 191: a
            // disabled Button).
            nav = nav.child(
                button("Next", cx.callback(|s| s.next()))
                    .variant(Variant::Filled)
                    .size(16.0)
                    .enabled(wizard_step_valid(&form, self.step)),
            );
        } else {
            let submit = {
                let handle = cx.handle();
                let fields = [name.clone(), email.clone(), pass.clone(), confirm.clone()];
                on(move || {
                    let form = wizard_form(
                        &fields[0].text(),
                        &fields[1].text(),
                        &fields[2].text(),
                        &fields[3].text(),
                    );
                    let valid = form.is_valid();
                    handle.set_state(|s| {
                        s.submit(valid);
                    });
                    if valid {
                        // Success: resets the wizard and notifies (a Snackbar, with an animated
                        // exit).
                        for field in &fields {
                            field.clear();
                        }
                        demo.toast("Account created");
                    }
                })
            };
            nav = nav.child(
                button("Create account", submit)
                    .variant(Variant::Filled)
                    .size(16.0),
            );
        }

        // A Scaffold, for what a form wants from one (milestone 288): Back / Next go in
        // the **persistent footer**, so they stay put while the steps scroll and are not
        // hunted for at the end of a long form; and the body is shortened by the keyboard
        // rather than covered by it, which is the default and is what a form needs.
        //
        // The form itself is what scrolls, and it says so: the Scaffold places the body and
        // does not wrap it (milestone 321). That matters most here — the footer must stay
        // pinned while the steps move, which is exactly the split between the two slots.
        // The steps are one form, for autofill: a group around whichever step is showing, at
        // the same place in the tree whatever the step, so the account's name typed on the
        // first and its password on the second are saved together (milestone 512).
        let content = frus_widgets::AutofillGroup::new(content);
        let inner = column![steps, content].gap(24.0).padding(24.0);
        let router = cx.router();
        Scaffold::new()
            .background(theme.background)
            .app_bar(NavigationBar::new("Sign-up wizard").on_back(on(move || {
                router.pop();
            })))
            .body(SingleChildScrollView::new().flex(1.0).child(inner))
            .persistent_footer(nav)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_reads_all_four_fields_and_wants_the_passwords_to_match() {
        let form = wizard_form("Ada", "ada@example.com", "longenough", "longenough");
        assert!(form.is_valid());
        let form = wizard_form("", "nope", "short", "different");
        assert!(!form.is_valid());
        assert_eq!(form.error("name"), Some("Name is required"));
        assert!(form.error("email").is_some());
        assert!(form.error("password").is_some());
        assert_eq!(form.error("confirm"), Some("Passwords do not match"));
    }

    #[test]
    fn a_step_is_valid_when_its_own_fields_are() {
        let form = wizard_form("Ada", "ada@example.com", "short", "short");
        assert!(wizard_step_valid(&form, 0));
        assert!(!wizard_step_valid(&form, 1), "the password is too short");
        assert!(!wizard_step_valid(&form, 2));
    }

    #[test]
    fn a_field_knows_its_step_and_its_focus_key() {
        assert_eq!(wizard_step_of("email"), 0);
        assert_eq!(wizard_step_of("confirm"), 1);
        assert_eq!(wizard_field_of("name"), 0);
        assert_eq!(wizard_field_of("confirm"), 3);
    }

    #[test]
    fn the_steps_move_within_their_bounds_and_a_bad_submit_lands_on_the_review() {
        let mut s = WizardState::default();
        s.back();
        assert_eq!(s.step, 0);
        s.next();
        s.next();
        s.next();
        assert_eq!(s.step, 2);
        s.go_to(9);
        assert_eq!(s.step, 2);
        s.go_to(0);
        assert!(!s.submit(false));
        assert!(s.submitted, "the errors show from now on");
        assert_eq!(s.step, 2, "on the review, where they are listed");
        assert!(s.submit(true));
        assert!(!s.submitted);
        assert_eq!(s.step, 0, "and a success starts over");
    }
}
