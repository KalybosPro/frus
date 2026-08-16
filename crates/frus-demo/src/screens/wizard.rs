//! The multi-step form screen: validation per step, and the wizard's own
//! navigation.

use crate::prelude::*;
use frus_widgets::column;

/// The wizard's form: a **pure** validation of the current state (milestones 180–181). The order
/// declares `password` before `confirm` (the cross-field `matches` validation).
pub(crate) fn wizard_form(app: &TodoApp) -> Form {
    Form::new()
        .field(
            "name",
            app.wizard_name.as_str(),
            Rule::required("Name is required"),
        )
        .field(
            "email",
            app.wizard_email.as_str(),
            Rule::all([
                Rule::required("Email is required"),
                Rule::email("Enter a valid email address"),
            ]),
        )
        .field(
            "password",
            app.wizard_pass.as_str(),
            Rule::min_len(8, "Password must be at least 8 characters"),
        )
        .matches(
            "confirm",
            app.wizard_confirm.as_str(),
            "password",
            "Passwords do not match",
        )
}

/// Which step (0 = Account, 1 = Security) the field `key` lives on — so that clicking an error
/// summary bullet jumps to the right step (milestones 181 + 183).
pub(crate) fn wizard_step_of(key: &str) -> usize {
    match key {
        "name" | "email" => 0,
        _ => 1,
    }
}

/// A wizard field's index (its focus key) — for `keyed`/`Command::focus`.
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

/// One wizard field: its error is shown **only after** submission, its value is **masked** for a
/// password, and it carries a **focus key** (`keyed`) so the summary can jump to it.
// Nine, and they are the field's whole description. A struct here would be a
// parameter list wearing a hat.
#[allow(clippy::too_many_arguments)]
pub(crate) fn wizard_input(
    form: &Form,
    submitted: bool,
    label: &str,
    value: &str,
    key: &str,
    field: u8,
    obscure: bool,
    eye: Option<bool>,
    field_width: f32,
) -> impl Widget<Msg> + 'static {
    let mut input = TextInput::new(value)
        .width(field_width)
        .size(16.0)
        .label(label)
        .obscure(obscure)
        .on_input(move |s| Msg::WizardInput(field, s));
    // `eye = Some(revealed)`: an eye icon **inside the field** toggles the masking (milestone 198).
    if let Some(revealed) = eye {
        let icon = if revealed {
            IconName::EyeOff
        } else {
            IconName::Eye
        };
        input = input.suffix_icon(icon).on_suffix(Msg::WizardToggleReveal);
    }
    if submitted {
        if let Some(err) = form.error(key) {
            input = input.error(err);
        }
    }
    keyed(("wizard", field), input)
}

/// The **sign-up wizard** screen: proof that the recent building blocks fit together — a
/// clickable [`Steps`] indicator (milestone 183), a validated [`Form`] (180) with a **clickable**
/// error summary (181), and a success notification (185/188).
pub(crate) fn wizard_screen(
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Box<dyn Widget<Msg>> {
    let form = wizard_form(app);
    let submitted = app.wizard_submitted;
    // A **responsive** field width: it fits the width (minus the 24×2 padding), capped at 360 px
    // so it does not stretch on a large screen.
    let field_w = (width - 48.0).clamp(240.0, 360.0);

    // Steps are marked "done" by **validity** (milestone 195), not merely by position — which
    // matches "Next" being gated by that same validity.
    let steps = Steps::new(["Account", "Security", "Review"])
        .current(app.wizard_step)
        .completed([
            wizard_step_valid(&form, 0),
            wizard_step_valid(&form, 1),
            form.is_valid(),
        ])
        .on_tap(Msg::WizardStep);

    // The current step's content.
    let content: Box<dyn Widget<Msg>> = match app.wizard_step {
        0 => Box::new(
            Flex::column()
                .gap(14.0)
                .child(wizard_input(
                    &form,
                    submitted,
                    "Full name",
                    &app.wizard_name,
                    "name",
                    0,
                    false,
                    None,
                    field_w,
                ))
                .child(wizard_input(
                    &form,
                    submitted,
                    "Email",
                    &app.wizard_email,
                    "email",
                    1,
                    false,
                    None,
                    field_w,
                )),
        ),
        1 => {
            // Passwords are masked unless revealed: the eye icon **inside the field** toggles it (198).
            let obscure = !app.wizard_reveal;
            let eye = Some(app.wizard_reveal);
            Box::new(
                Flex::column()
                    .gap(14.0)
                    .child(wizard_input(
                        &form,
                        submitted,
                        "Password",
                        &app.wizard_pass,
                        "password",
                        2,
                        obscure,
                        eye,
                        field_w,
                    ))
                    .child(wizard_input(
                        &form,
                        submitted,
                        "Confirm password",
                        &app.wizard_confirm,
                        "confirm",
                        3,
                        obscure,
                        eye,
                        field_w,
                    )),
            )
        }
        _ => {
            let mut review = Flex::column().gap(14.0);
            // A clickable summary: each bullet jumps to the faulty field's step **and** focuses
            // it (milestones 181 + 183 + programmatic focus).
            if submitted && !form.is_valid() {
                let links = form.errors().into_iter().map(|(key, message)| {
                    (
                        message.to_string(),
                        Msg::WizardFocus(wizard_step_of(key), wizard_field_of(key)),
                    )
                });
                review = review.child(ErrorSummary::links(links));
            }
            review = review.child(
                text(format!(
                    "Creating account for {} <{}>",
                    if app.wizard_name.is_empty() {
                        "—"
                    } else {
                        app.wizard_name.as_str()
                    },
                    if app.wizard_email.is_empty() {
                        "—"
                    } else {
                        app.wizard_email.as_str()
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
    if app.wizard_step > 0 {
        nav = nav.child(
            button("Back", Msg::WizardBack)
                .variant(Variant::Outlined)
                .size(16.0),
        );
    }
    if app.wizard_step < 2 {
        // "Next" only becomes active once the current step is valid (milestone 191: a disabled Button).
        nav = nav.child(
            button("Next", Msg::WizardNext)
                .variant(Variant::Filled)
                .size(16.0)
                .enabled(wizard_step_valid(&form, app.wizard_step)),
        );
    } else {
        nav = nav.child(
            button("Create account", Msg::WizardSubmit)
                .variant(Variant::Filled)
                .size(16.0),
        );
    }

    // A Scaffold, for what a form wants from one (milestone 288): Back / Next go in
    // the **persistent footer**, so they stay put while the steps scroll and are not
    // hunted for at the end of a long form; and the body is shortened by the keyboard
    // rather than covered by it, which is the default and is what a form needs.
    let inner = column![steps, content].gap(24.0).padding(24.0);
    Scaffold::new(width, height)
        .background(theme.background)
        .app_bar(NavBar::new("Sign-up wizard").on_back(Msg::Pop))
        .body(inner)
        .persistent_footer(nav)
        .build()
}
