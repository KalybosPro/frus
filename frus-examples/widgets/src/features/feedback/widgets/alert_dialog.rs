//! `AlertDialog` — a modal box that asks a question and waits for the answer: an
//! optional icon, a title, some content and the buttons that answer it.
//!
//! There is no "show dialog" call. A dialog is **controlled** like every overlay: it is
//! built around a body (`.body(..)`), the application keeps whether it is open, and the
//! buttons change that flag.

use frus::{
    AlertDialog, Align, Button, Column, Icon, Icons, State, StateContext, StatefulWidget, Text,
    Variant, Widget, Wrap,
};

use crate::common::shared::section;

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(AlertDialogDemo.into_widget())
}

struct AlertDialogDemo;

#[derive(Clone, Copy, PartialEq)]
enum Open {
    None,
    Discard,
    Delete,
}

struct AlertDialogDemoState {
    open: Open,
    answer: &'static str,
}

impl AlertDialogDemoState {
    /// Every button of a dialog closes it, and says what was answered.
    fn answer(&mut self, answer: &'static str) {
        self.answer = answer;
        self.open = Open::None;
    }
}

impl StatefulWidget for AlertDialogDemo {
    type State = AlertDialogDemoState;

    fn create_state(&self) -> AlertDialogDemoState {
        AlertDialogDemoState {
            open: Open::None,
            answer: "nothing yet",
        }
    }
}

impl State for AlertDialogDemoState {
    type Widget = AlertDialogDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let answer =
            |what: &'static str| cx.callback(move |s: &mut AlertDialogDemoState| s.answer(what));

        // What the dialogs open over: the buttons that open them.
        let body = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Start)
            .child(section(
                "Open a dialog",
                Wrap::new()
                    .gap(8.0)
                    .run_gap(8.0)
                    .child(Button::new("Discard draft").on_press(
                        cx.callback(|s: &mut AlertDialogDemoState| s.open = Open::Discard),
                    ))
                    .child(
                        Button::new("Delete account")
                            .variant(Variant::Danger)
                            .on_press(
                                cx.callback(|s: &mut AlertDialogDemoState| s.open = Open::Delete),
                            ),
                    ),
            ))
            .child(Text::new(format!("Answer: {}", self.answer)));

        // A title, a sentence, two text buttons. A tap on the scrim around it sends
        // `on_dismiss`, so this one can be left without answering.
        let discard = AlertDialog::new(self.open == Open::Discard)
            .title("Discard draft?")
            .content("Your message has not been sent.")
            .action(
                Button::new("Keep editing")
                    .variant(Variant::Text)
                    .on_press(answer("kept")),
            )
            .action(
                Button::new("Discard")
                    .variant(Variant::Text)
                    .on_press(answer("discarded")),
            )
            .on_dismiss(answer("dismissed"))
            .body(body);

        // An icon centres the title. No `on_dismiss`: the scrim is inert, and the
        // question must be answered with a button.
        let delete = AlertDialog::new(self.open == Open::Delete)
            .icon(Icon::new(Icons::WARNING))
            .title("Delete your account?")
            .content("Everything you stored will be removed. This cannot be undone.")
            .action(
                Button::new("Cancel")
                    .variant(Variant::Text)
                    .on_press(answer("cancelled")),
            )
            .action(
                Button::new("Delete")
                    .variant(Variant::Danger)
                    .on_press(answer("deleted")),
            )
            .body(discard);

        Box::new(delete)
    }
}
