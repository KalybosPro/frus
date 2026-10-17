//! `SafeArea` — keeps its child clear of the parts of the screen the system has taken:
//! the status bar, a notch or camera cut-out, the gesture handle, the keyboard.
//!
//! On desktop those insets are zero and `SafeArea` changes nothing, so the same screen
//! code is right everywhere. Wrap the body of a page that draws edge to edge in one.

use frus::{Color, Container, Edges, Insets, SafeArea, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // `build` hands you the screen's current metrics: here, the insets it is keeping
        // clear on this device. On a desktop window they are all zero.
        Box::new(section(
            "The insets on this device",
            SafeArea::build(|media| {
                let p = media.padding;
                Text::new(format!(
                    "top {:.0}, right {:.0}, bottom {:.0}, left {:.0}",
                    p.top, p.right, p.bottom, p.left
                ))
            }),
        )),
        // Every edge, plus a minimum padding of 12 px so the content never touches the
        // frame even where the system takes nothing.
        Box::new(section(
            "All edges, with a minimum",
            framed(
                SafeArea::new(content("Kept clear of every system bar"))
                    .minimum(Insets::uniform(12.0)),
            ),
        )),
        // A list that should run under the gesture handle drops the bottom edge; a form
        // adds `avoid_keyboard` to move up when the on-screen keyboard opens.
        Box::new(section(
            "Without the bottom edge, avoiding the keyboard",
            framed(
                SafeArea::new(content("Runs under the handle, rises above the keyboard"))
                    .edges(Edges::ALL.without_bottom())
                    .avoid_keyboard(),
            ),
        )),
    ])
}

fn framed(child: impl Widget + 'static) -> Container {
    Container::new()
        .width(300.0)
        .color(Color::hex("#E0E7FF"))
        .child(child)
}

fn content(text: &str) -> Container {
    Container::new()
        .padding(10.0)
        .color(Color::hex("#6366F1"))
        .child(Text::new(text).color(Color::hex("#FFFFFF")).wrap())
}
