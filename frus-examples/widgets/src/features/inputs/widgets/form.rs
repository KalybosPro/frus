//! `form::Form`, `form::Rule` and `ErrorSummary` — validation as a plain function of the
//! state. Rules check each value, `Form` collects the answers, and the fields only
//! **show** them: nothing here draws until you hand an error to a `TextField`.

use frus::form::{Form, Rule};
use frus::{
    host, keyed, Align, Button, Callback, Column, ErrorSummary, State, StateContext,
    StatefulWidget, Text, TextField, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(SignUpForm.into_widget())
}

/// The whole validation of the form: pure, so it is tested below without a window.
fn validate(name: &str, email: &str, password: &str, confirm: &str) -> Form {
    Form::new()
        .field("name", name, Rule::required("Enter your name"))
        .field(
            "email",
            email,
            // `all`: the first rule that fails gives the message.
            Rule::all([
                Rule::required("Enter your email"),
                Rule::email("Enter a valid email address"),
            ]),
        )
        .field(
            "password",
            password,
            Rule::all([
                Rule::min_len(8, "At least 8 characters"),
                // Any closure is a rule: `None` passes, `Some(message)` fails.
                Rule::new(|v| {
                    (!v.chars().any(|c| c.is_ascii_digit())).then(|| "Include a digit".to_string())
                }),
            ]),
        )
        // A cross-field rule: it reads a field declared before it.
        .matches("confirm", confirm, "password", "The passwords differ")
}

struct SignUpForm;

#[derive(Default)]
struct SignUpState {
    name: String,
    email: String,
    password: String,
    confirm: String,
    /// Errors show only after a first attempt: a form that is red before anyone typed
    /// scolds the reader for nothing.
    submitted: bool,
    sent: bool,
}

impl SignUpState {
    fn form(&self) -> Form {
        validate(&self.name, &self.email, &self.password, &self.confirm)
    }

    fn submit(&mut self) {
        self.submitted = true;
        let form = self.form();
        self.sent = form.is_valid();
        // Bring the first wrong field forward. A field is reached by the key it is
        // `keyed` with in `build`.
        if let Some(first) = form.first_invalid() {
            host::focus(("signup", first));
        }
    }
}

impl StatefulWidget for SignUpForm {
    type State = SignUpState;

    fn create_state(&self) -> SignUpState {
        SignUpState::default()
    }
}

impl State for SignUpState {
    type Widget = SignUpForm;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let form = self.form();
        let error = |key: &str| {
            if self.submitted {
                form.error(key)
            } else {
                None
            }
        };

        // One field, decorated with its error if it has one, and keyed so that it can be
        // focused from code.
        let field = |key: &'static str,
                     label: &str,
                     value: &str,
                     obscure: bool,
                     on_input: Box<dyn Fn(String) -> Callback>| {
            let mut field = TextField::new(value)
                .label(label)
                .obscure(obscure)
                .width(300.0)
                .on_input(on_input);
            if let Some(message) = error(key) {
                field = field.error(message);
            }
            keyed(("signup", key), field)
        };

        let mut column = Column::new()
            .spacing(12.0)
            .cross_axis_alignment(Align::Start);

        // The summary lists every error; each bullet focuses its field.
        if self.submitted && !form.is_valid() {
            let links = form.errors().into_iter().map(|(key, message)| {
                (
                    message.to_string(),
                    cx.callback(move |_: &mut SignUpState| host::focus(("signup", key))),
                )
            });
            column = column.child(ErrorSummary::links(links));
        }

        column = column
            .child(field(
                "name",
                "Name",
                &self.name,
                false,
                Box::new(cx.handler(|s: &mut SignUpState, t: String| s.name = t)),
            ))
            .child(field(
                "email",
                "Email",
                &self.email,
                false,
                Box::new(cx.handler(|s: &mut SignUpState, t: String| s.email = t)),
            ))
            .child(field(
                "password",
                "Password",
                &self.password,
                true,
                Box::new(cx.handler(|s: &mut SignUpState, t: String| s.password = t)),
            ))
            .child(field(
                "confirm",
                "Confirm password",
                &self.confirm,
                true,
                Box::new(cx.handler(|s: &mut SignUpState, t: String| s.confirm = t)),
            ))
            .child(Button::new("Create account").on_press(cx.callback(SignUpState::submit)));

        if self.sent {
            column = column.child(Text::new(format!("Welcome, {}!", self.name)));
        }

        sections(vec![
            Box::new(section("A sign-up form", column)),
            // `ErrorSummary::new` is the same card with inert bullets.
            Box::new(section(
                "ErrorSummary on its own",
                ErrorSummary::new(["Enter your name", "At least 8 characters"]),
            )),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_complete_form_is_valid() {
        let form = validate("Ada", "ada@example.com", "secret123", "secret123");
        assert!(form.is_valid());
    }

    #[test]
    fn the_first_failing_rule_gives_the_message() {
        let form = validate("", "ada@", "short", "other");
        assert_eq!(form.error("name"), Some("Enter your name"));
        assert_eq!(form.error("email"), Some("Enter a valid email address"));
        assert_eq!(form.error("password"), Some("At least 8 characters"));
        assert_eq!(form.error("confirm"), Some("The passwords differ"));
        assert_eq!(form.first_invalid(), Some("name"));
    }

    #[test]
    fn a_password_needs_a_digit() {
        let form = validate("Ada", "ada@example.com", "longenough", "longenough");
        assert_eq!(form.error("password"), Some("Include a digit"));
    }
}
