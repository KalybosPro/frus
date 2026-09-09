//! The licence screen: everything this application links, and under what terms — under a
//! header that gives way to the list as the list moves.

use crate::prelude::*;
use frus_widgets::{column, row, CollapsingHeader, HeaderState};

/// The room the header takes at the top of the page, and so the room the list reserves.
/// **One number**: the header reads it off the list rather than being told it twice.
const HEADER_HEIGHT: f32 = 168.0;
/// What it shrinks to and stays at — a toolbar, which is what is left when a header has
/// nothing but its title and the way back.
const TOOLBAR_HEIGHT: f32 = 60.0;

/// The "Licences" screen: the [`LicensePage`] over the notices registered at start-up
/// from `assets/licenses.txt`, which `scripts/gen_licenses.py` generates from this
/// application's **own** dependency graph.
///
/// The page is a column and does not scroll itself — four hundred packages are taller
/// than any screen, and a widget that guessed its own height would guess wrong in a panel
/// (milestone 263's rule, and the reason `Kanban` takes a height too). The screen puts it
/// in the scroll, which is where a screen's content belongs anyway.
///
/// Since milestone 494 the bar over it is a [`CollapsingHeader`]: four hundred packages
/// are the longest list in this application, so it is the one where a header that keeps
/// its whole height for ever costs the most.
pub(crate) fn licenses_screen(app: &TodoApp, theme: &Theme) -> Container<Msg> {
    let Size { width, height } = surface();
    let page = LicensePage::new(app.licence_open, Msg::OpenLicence)
        .application("frus demo")
        .version(format!("version {}", env!("CARGO_PKG_VERSION")))
        .legalese("The demo itself is MIT OR Apache-2.0. What follows is everything it links.")
        .build();
    let body = SingleChildScrollView::new()
        .width(width)
        .flex(1.0)
        // The header's room, **inside** the viewport and scrolling with the content — so
        // the first package clears the header at rest and passes under it on the way up.
        // This is also where the header reads its expanded height from.
        .padding_each(HEADER_HEIGHT, 0.0, 0.0, 0.0)
        .child(Container::new().padding(20.0).child(page));
    let t = theme.clone();
    let bar = CollapsingHeader::new(body, move |state| header(state, &t))
        .collapsed_height(TOOLBAR_HEIGHT)
        .build();
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(SafeArea::new(column![bar].flex(1.0)))
}

/// The header itself, at whatever height it has been given.
///
/// Everything that moves is interpolated on `state.fraction` rather than on the height,
/// so the two constants above can change without this having to: `0` is fully open and
/// `1` is a toolbar, whatever those are worth in pixels.
fn header(state: HeaderState, theme: &Theme) -> Container<Msg> {
    let f = state.fraction;
    // The title ends at the size a bar's title has and starts half again as big. Read off
    // the theme's own step rather than a number typed here, so a theme that changes its
    // type scale takes the header with it.
    let title_size = 20.0 + (1.0 - f) * 14.0;
    // The subtitle is the first thing to go: it is the part nobody needs twice.
    let subtitle = Color {
        a: (1.0 - f * 2.0).clamp(0.0, 1.0),
        ..theme.muted
    };
    // A line under the bar once it is over content, and not before — the shadow a sheet
    // of paper casts only once something has gone under it.
    let edge = Color {
        a: f,
        ..theme.border
    };
    let title = column![
        text("Licences").size(title_size),
        text("Everything this application links")
            .size(13.0)
            .color(subtitle),
    ]
    .gap(2.0);
    Container::new()
        .width(state.width)
        .height(state.height)
        .color(theme.surface)
        .child(column![
            // The gap that absorbs the shrinking, so the title **rises** with it rather
            // than the header closing over the title from below.
            Expanded::new(Container::new()),
            Container::new().padding_each(0.0, 16.0, 10.0, 8.0).child(
                row![
                    button("←", Msg::Pop).variant(Variant::Text),
                    Expanded::new(title),
                ]
                .gap(4.0)
            ),
            // Flush to the bottom edge, which is why it is a row of the column rather
            // than a border on the box: a border would be inside the padding.
            Container::new().height(1.0).color(edge),
        ])
}
