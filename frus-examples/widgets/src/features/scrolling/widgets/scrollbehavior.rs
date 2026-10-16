//! Scroll behaviour — how scrollables feel at their edges and whether they show
//! scrollbars. The app's default follows the platform; `ScrollConfiguration` changes it
//! for a subtree, and each scrollable can override its own physics and scrollbar.

use frus::{
    Color, Column, Container, ListTile, ListView, Row, ScrollConfiguration, ScrollPhysics,
    Scrollbars, Text, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // Per scrollable: the physics at the edges. Bouncing goes past the edge and
        // springs back; clamping stops dead; `NEVER` does not let the user scroll at all
        // (programmatic scrolling still works).
        Box::new(section(
            "Physics",
            Wrap::new()
                .gap(12.0)
                .run_gap(12.0)
                .child(labelled(
                    "Bouncing",
                    list().physics(ScrollPhysics::BOUNCING),
                ))
                .child(labelled(
                    "Clamping",
                    list().physics(ScrollPhysics::CLAMPING),
                ))
                .child(labelled("Never", list().physics(ScrollPhysics::NEVER))),
        )),
        // For a whole subtree: take the behaviour in force and change one thing. Here,
        // no scrollbars and no overscroll indicator for everything inside.
        Box::new(section(
            "ScrollConfiguration for a subtree",
            ScrollConfiguration::new(
                ScrollConfiguration::of()
                    .with_scrollbars(false)
                    .with_overscroll(false),
                Row::new()
                    .spacing(12.0)
                    .main_axis_size(frus::MainAxisSize::Min)
                    .child(labelled("No scrollbar", list()))
                    .child(labelled("Nor this one", list())),
            ),
        )),
        // A single scroll view can insist on a scrollbar where the platform shows none
        // (on a touch screen, say) and keep it visible.
        Box::new(section(
            "Scrollbar always on",
            frus::SingleChildScrollView::new()
                .scrollbars(Scrollbars::Always)
                .thumb_visibility(true)
                .width(150.0)
                .height(160.0)
                .child((1..=20).fold(Column::new(), |column, i| {
                    column.child(Text::new(format!("Line {i}")))
                })),
        )),
    ])
}

/// A short list to try the behaviours on.
fn list() -> ListView {
    ListView::new(30, 40.0, |i| {
        ListTile::new().dense().title(format!("Item {}", i + 1))
    })
    .width(150.0)
    .height(160.0)
}

fn labelled(label: &str, list: impl Widget + 'static) -> Column {
    Column::new()
        .spacing(4.0)
        .main_axis_size(frus::MainAxisSize::Min)
        .child(Text::new(label).size(12.0))
        .child(
            Container::new()
                .border(1.0, Color::hex("#A1A1AA"))
                .child(list),
        )
}
