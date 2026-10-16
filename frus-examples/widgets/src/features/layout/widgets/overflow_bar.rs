//! `OverflowBar` — a row of buttons that becomes a column when they stop fitting.
//!
//! Unlike `Wrap`, it folds all of its children or none: never a second line with one
//! button stranded on it. It is the bar for dialog actions, which must survive a larger
//! system font size.

use frus::{Align, Button, Color, Container, Justify, OverflowBar, Variant, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // Short labels with room enough: one line, pushed to the end as dialog actions
        // are.
        Box::new(section(
            "Fits on one line",
            framed(320.0, actions(["Cancel", "Keep", "Delete"])),
        )),
        // Longer labels in a narrower box: the buttons fold into a column, aligned at
        // the end, with `overflow_spacing` between them — never a squashed row.
        Box::new(section(
            "Too narrow: folds into a column",
            framed(
                240.0,
                actions(["Cancel", "Keep a copy", "Delete permanently"]),
            ),
        )),
    ])
}

fn actions([cancel, keep, delete]: [&str; 3]) -> OverflowBar {
    // A button with no `on_press` is disabled; these do nothing but stay enabled.
    OverflowBar::new()
        .spacing(8.0)
        .alignment(Justify::End)
        .overflow_spacing(4.0)
        .overflow_alignment(Align::End)
        .child(Button::new(cancel).variant(Variant::Text).on_press(|| ()))
        .child(Button::new(keep).variant(Variant::Outlined).on_press(|| ()))
        .child(Button::new(delete).on_press(|| ()))
}

fn framed(width: f32, bar: OverflowBar) -> Container {
    Container::new()
        .width(width)
        .padding(12.0)
        .radius(8.0)
        .border(1.0, Color::hex("#C7D2FE"))
        .child(bar)
}
