//! `SimpleDialog` and `SimpleDialogOption` — a modal list of choices, answered by
//! picking one row. No buttons: choosing **is** the answer.

use frus::{
    Align, Button, Column, Icon, Icons, Row, SimpleDialog, SimpleDialogOption, State, StateContext,
    StatefulWidget, Text, Widget,
};

use crate::common::shared::section;

pub fn example() -> Box<dyn Widget> {
    Box::new(SimpleDialogDemo.into_widget())
}

const ACCOUNTS: [&str; 3] = ["ada@example.com", "grace@example.com", "alan@example.com"];

struct SimpleDialogDemo;

struct SimpleDialogDemoState {
    open: bool,
    account: usize,
}

impl StatefulWidget for SimpleDialogDemo {
    type State = SimpleDialogDemoState;

    fn create_state(&self) -> SimpleDialogDemoState {
        SimpleDialogDemoState {
            open: false,
            account: 0,
        }
    }
}

impl State for SimpleDialogDemoState {
    type Widget = SimpleDialogDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let body = section(
            "Choose from a list",
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(Text::new(format!(
                    "Signed in as {}",
                    ACCOUNTS[self.account]
                )))
                .child(
                    Button::new("Switch account")
                        .on_press(cx.callback(|s: &mut SimpleDialogDemoState| s.open = true)),
                ),
        );

        // Each option sends its own message; `build()` turns it into the full-width row.
        let dialog = ACCOUNTS.iter().enumerate().fold(
            SimpleDialog::new(self.open).title("Switch account"),
            |dialog, (i, account)| {
                dialog.option(
                    SimpleDialogOption::with_child(
                        Row::new()
                            .spacing(16.0)
                            .cross_axis_alignment(Align::Center)
                            .child(Icon::new(Icons::PERSON))
                            .child(Text::new(*account)),
                    )
                    .on_press(cx.callback(move |s: &mut SimpleDialogDemoState| {
                        s.account = i;
                        s.open = false;
                    }))
                    .build(),
                )
            },
        );
        // A plain text row works too.
        let dialog = dialog
            .option(
                SimpleDialogOption::new(
                    "Cancel",
                    cx.callback(|s: &mut SimpleDialogDemoState| s.open = false),
                )
                .build(),
            )
            .on_dismiss(cx.callback(|s: &mut SimpleDialogDemoState| s.open = false))
            .body(body);

        Box::new(dialog)
    }
}
