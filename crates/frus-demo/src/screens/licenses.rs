//! The licence screen: everything this application links, and under what terms.

use crate::prelude::*;
use frus_widgets::column;

/// The "Licences" screen: the [`LicensePage`] over the notices registered at start-up
/// from `assets/licenses.txt`, which `scripts/gen_licenses.py` generates from this
/// application's **own** dependency graph.
///
/// The page is a column and does not scroll itself — four hundred packages are taller
/// than any screen, and a widget that guessed its own height would guess wrong in a panel
/// (milestone 263's rule, and the reason `Kanban` takes a height too). The screen puts it
/// in the scroll, which is where a screen's content belongs anyway.
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
        .child(Container::new().padding(20.0).child(page));
    let screen = column![NavigationBar::new("Licences").on_back(Msg::Pop), body].flex(1.0);
    Container::new()
        .width(width)
        .height(height)
        .color(theme.background)
        .child(SafeArea::new(screen))
}
