//! `AutofillGroup` and `AutofillHint` — say what each field **holds** (a user name, a new
//! password, an SMS code) so that password managers and the system can fill it in, and
//! group a form's fields so that a username and its password are saved together.

use frus::{
    Align, AutofillGroup, AutofillHint, Button, Column, KeyboardType, State, StateContext,
    StatefulWidget, Text, TextField, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(AutofillDemo.into_widget())
}

struct AutofillDemo;

#[derive(Default)]
struct AutofillDemoState {
    email: String,
    password: String,
    code: String,
    signed_in: bool,
}

impl StatefulWidget for AutofillDemo {
    type State = AutofillDemoState;

    fn create_state(&self) -> AutofillDemoState {
        AutofillDemoState::default()
    }
}

impl State for AutofillDemoState {
    type Widget = AutofillDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // A field can be several things at once, best description first: this box takes
        // an email address that is also the user name.
        let form = Column::new()
            .spacing(12.0)
            .cross_axis_alignment(Align::Start)
            .child(
                TextField::new(&self.email)
                    .label("Email")
                    .keyboard_type(KeyboardType::Email)
                    .autofill([AutofillHint::Username, AutofillHint::Email])
                    .width(300.0)
                    .on_input(cx.handler(|s: &mut AutofillDemoState, t: String| s.email = t)),
            )
            .child(
                TextField::new(&self.password)
                    .label("Password")
                    .obscure(true)
                    .autofill([AutofillHint::Password])
                    .width(300.0)
                    .on_input(cx.handler(|s: &mut AutofillDemoState, t: String| s.password = t)),
            )
            .child(
                Button::new("Sign in")
                    .on_press(cx.callback(|s: &mut AutofillDemoState| s.signed_in = true)),
            );

        sections(vec![
            // The group is what tells the system these two fields are one credential.
            Box::new(section("Sign-in form", AutofillGroup::new(form))),
            // A one-time code: on a phone the code from an SMS is offered above the
            // keyboard.
            Box::new(section(
                "One-time code",
                TextField::new(&self.code)
                    .label("Code from the SMS")
                    .keyboard_type(KeyboardType::Number)
                    .autofill([AutofillHint::OneTimeCode])
                    .max_length(6)
                    .width(300.0)
                    .on_input(cx.handler(|s: &mut AutofillDemoState, t: String| s.code = t)),
            )),
            Box::new(Text::new(if self.signed_in {
                "Signed in (a password manager would offer to save this pair)"
            } else {
                "Hints change nothing visible: they speak to the platform's autofill"
            })),
        ])
    }
}
