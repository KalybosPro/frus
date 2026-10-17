//! `Alert` — a message box that sits **in the flow** of the page and stays: info,
//! success, warning or error, with an optional title. No actions, no overlay — for
//! context next to what it is about.

use frus::{Alert, Align, Color, Column, TextStyle, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // The kind sets the accent colour and the icon.
        Box::new(section(
            "Kinds",
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Stretch)
                .child(Alert::new("Your trial ends in 3 days."))
                .child(Alert::new("Payment received.").success())
                .child(Alert::new("Your password expires soon.").warning())
                .child(Alert::new("The file could not be uploaded.").error()),
        )),
        // A title above the message, for something that needs a heading.
        Box::new(section(
            "With a title",
            Alert::new("Two fields are missing. Fill them in to continue.")
                .title("Form incomplete")
                .error(),
        )),
        // The text styles can be changed like every other widget's.
        Box::new(section(
            "Custom text style",
            Alert::new("Larger, darker text.")
                .title("Note")
                .title_text_style(TextStyle::new(18.0).color(Color::hex("#1E3A8A")))
                .content_text_style(TextStyle::new(15.0)),
        )),
    ])
}
