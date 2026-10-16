//! `MouseRegion` — hears the mouse entering, moving over and leaving a widget, and sets
//! the cursor shown there.
//!
//! On a touch screen a finger is "over" a region only while it touches: an enter when it
//! goes down, an exit when it lifts, and no moves.

use frus::{
    Alignment, Container, Cursor, MouseRegion, Point, State, StateContext, StatefulWidget, Text,
    Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(HoverDemo.into_widget())
}

struct HoverDemo;

struct HoverDemoState {
    inside: bool,
    position: Option<Point>,
}

impl StatefulWidget for HoverDemo {
    type State = HoverDemoState;

    fn create_state(&self) -> HoverDemoState {
        HoverDemoState {
            inside: false,
            position: None,
        }
    }
}

/// The cursors worth knowing, with the place each belongs.
const CURSORS: [(Cursor, &str); 8] = [
    (Cursor::Pointer, "Pointer: a link"),
    (Cursor::Text, "Text: an editable line"),
    (Cursor::Move, "Move"),
    (Cursor::Grab, "Grab: a handle"),
    (Cursor::Forbidden, "Forbidden"),
    (Cursor::Wait, "Wait"),
    (Cursor::Help, "Help"),
    (Cursor::ZoomIn, "Zoom in"),
];

impl State for HoverDemoState {
    type Widget = HoverDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let scheme = &cx.theme().scheme;

        // Each callback receives the pointer's position in the region's own coordinates.
        let card = MouseRegion::new(
            Container::new()
                .width(280.0)
                .height(120.0)
                .radius(12.0)
                .color(if self.inside {
                    scheme.primary_container
                } else {
                    scheme.surface_container_high
                })
                .alignment(Alignment::CENTER)
                .child(Text::new(match self.position {
                    Some(p) if self.inside => format!("Pointer at ({:.0}, {:.0})", p.x, p.y),
                    _ => "Move the mouse over me".to_string(),
                })),
        )
        .on_enter(cx.handler(|s: &mut Self, at: Point| {
            s.inside = true;
            s.position = Some(at);
        }))
        .on_hover(cx.handler(|s: &mut Self, at: Point| s.position = Some(at)))
        .on_exit(cx.handler(|s: &mut Self, _at: Point| s.inside = false))
        .cursor(Cursor::Precise);

        let cursors =
            CURSORS
                .iter()
                .fold(Wrap::new().gap(8.0).run_gap(8.0), |row, (cursor, name)| {
                    row.child(
                        MouseRegion::new(
                            Container::new()
                                .padding(10.0)
                                .radius(8.0)
                                .border(1.0, scheme.outline_variant)
                                .child(Text::new(*name).size(13.0)),
                        )
                        .cursor(*cursor),
                    )
                });

        sections(vec![
            Box::new(section("Enter, hover and exit", card)),
            Box::new(section("Cursors (hover each)", cursors)),
        ])
    }
}
