//! `Dialog` — the modal surface on its own: rounded, raised, held off the window's edges,
//! with **any content** in it. For what is not a title-and-buttons question: a small form,
//! a picture, a progress report.

use frus::{
    Align, Button, Column, Dialog, FontWeight, Insets, Justify, Padding, Row, State, StateContext,
    StatefulWidget, Text, TextField, Variant, Widget,
};

use crate::common::shared::section;

pub fn example() -> Box<dyn Widget> {
    Box::new(DialogDemo.into_widget())
}

pub struct DialogDemo;

pub struct DialogDemoState {
    pub open: bool,
    /// What is being typed in the dialog, kept apart from the saved value so that
    /// Cancel can throw it away.
    pub draft: String,
    pub name: String,
}

impl DialogDemoState {
    pub fn open(&mut self) {
        self.draft = self.name.clone();
        self.open = true;
    }

    pub fn save(&mut self) {
        self.name = self.draft.trim().to_string();
        self.open = false;
    }

    pub fn cancel(&mut self) {
        self.open = false;
    }
}

impl StatefulWidget for DialogDemo {
    type State = DialogDemoState;

    fn create_state(&self) -> DialogDemoState {
        DialogDemoState {
            open: false,
            draft: String::new(),
            name: "My playlist".into(),
        }
    }
}

impl State for DialogDemoState {
    type Widget = DialogDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let body = section(
            "A form in a dialog",
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(Text::new(format!("Playlist: {}", self.name)))
                .child(Button::new("Rename").on_press(cx.callback(DialogDemoState::open))),
        );

        // The content is ours to lay out: a heading, a field, a row of buttons.
        let content = Column::new()
            .spacing(16.0)
            .cross_axis_alignment(Align::Stretch)
            .child(
                Text::new("Rename playlist")
                    .size(20.0)
                    .weight(FontWeight::SemiBold),
            )
            .child(
                TextField::new(&self.draft)
                    .label("Name")
                    .on_input(cx.handler(|s: &mut DialogDemoState, t: String| s.draft = t))
                    .on_submit(cx.callback(DialogDemoState::save)),
            )
            .child(
                Row::new()
                    .spacing(8.0)
                    .main_axis_alignment(Justify::End)
                    .child(
                        Button::new("Cancel")
                            .variant(Variant::Text)
                            .on_press(cx.callback(DialogDemoState::cancel)),
                    )
                    .child(
                        Button::new("Save")
                            .enabled(!self.draft.trim().is_empty())
                            .on_press(cx.callback(DialogDemoState::save)),
                    ),
            );

        // Corners, widths and the room around it override the defaults (a 28-radius
        // surface at least 280 wide); `background` and `elevation` work the same way.
        let dialog = Dialog::new(self.open)
            .child(Padding::new(24.0, content))
            .radius(16.0)
            .max_width(360.0)
            .inset_padding(Insets::symmetric(16.0, 24.0))
            .on_dismiss(cx.callback(DialogDemoState::cancel))
            .body(body);

        Box::new(dialog)
    }
}
