//! `IndexedStack` — several children in one box, only one of them shown: the panels of
//! a tab set, the steps of a wizard.
//!
//! The box is as big as the biggest child, so switching panels does not make the page
//! jump.

use frus::{
    component, Align, Alignment, Color, Column, Container, IndexedStack, SegmentedButton, Text,
    Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![Box::new(section("Switching panels", panels()))])
}

/// The selected index is state: a component keeps it between frames, and the segmented
/// button changes it.
fn panels() -> impl Widget {
    component(|cx| {
        let selected = cx.use_state(|| 0_usize);
        let index = selected.get();
        // A segmented button reports the segment that was tapped; the closure turns that
        // into the work to do — store it.
        let on_select = move |segment: usize| {
            let selected = selected.clone();
            (move || selected.set(segment)).into()
        };
        Box::new(
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(12.0)
                .child(
                    SegmentedButton::new(index, on_select)
                        .segment("Inbox")
                        .segment("Drafts")
                        .segment("Archive"),
                )
                .child(
                    IndexedStack::new(index)
                        .child(panel("Inbox", "3 unread messages", "#6366F1", 80.0))
                        .child(panel("Drafts", "Nothing saved yet", "#F59E0B", 140.0))
                        .child(panel("Archive", "128 messages", "#10B981", 110.0)),
                ),
        )
    })
}

/// Panels of different heights: the stack keeps the tallest one's height for all of them.
fn panel(title: &str, body: &str, color: &str, height: f32) -> Container {
    Container::new()
        .width(280.0)
        .height(height)
        .padding(12.0)
        .radius(8.0)
        .color(Color::hex(color))
        .alignment(Alignment::TOP_LEFT)
        .child(
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(4.0)
                .child(Text::new(title).size(18.0).color(Color::hex("#FFFFFF")))
                .child(Text::new(body).color(Color::hex("#FFFFFF"))),
        )
}
