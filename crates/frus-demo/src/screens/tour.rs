//! The paged walkthrough screen.

use crate::prelude::*;
use frus_widgets::column;

/// The walkthrough's panels: a glyph, a title, and a line of body text.
pub(crate) const TOUR_PAGES: [(&str, &str, &str); 4] = [
    (
        "\u{1F44B}",
        "Welcome",
        "Swipe sideways, or use the picker below. Both drive the same page.",
    ),
    (
        "\u{1F446}",
        "One panel at a time",
        "A release never rests between two panels: it springs to one of them.",
    ),
    (
        "\u{26A1}",
        "A flick is enough",
        "You need not drag a panel all the way across; a short flick turns it.",
    ),
    (
        "\u{2713}",
        "That is the tour",
        "The picker follows the finger as soon as the page reads as changed.",
    ),
];

/// One panel of the walkthrough. Built on demand — a page that is off screen does
/// not exist — so it takes the theme by value rather than borrowing the frame's.
pub(crate) fn tour_panel(index: usize, theme: Theme) -> Container<Msg> {
    let (glyph, title, body) = TOUR_PAGES[index];
    // Every other panel takes the surface colour, so a swipe is visible even at the
    // moment the two panels are half and half.
    let background = if index.is_multiple_of(2) {
        theme.surface
    } else {
        theme.background
    };
    Container::new().color(background).padding(32.0).child(
        column![
            text(glyph).size(56.0),
            text(title).size(24.0).weight(FontWeight::Bold),
            text(body).size(15.0).color(theme.muted).wrap(),
        ]
        .gap(16.0)
        .align(Align::Center)
        .justify(Justify::Center),
    )
}

pub(crate) fn tour_screen(
    app: &TodoApp,
    theme: &Theme,
    width: f32,
    height: f32,
) -> Box<dyn Widget<Msg>> {
    let last = TOUR_PAGES.len() - 1;
    let page = app.tour_page.min(last);
    let palette = *theme;
    let pages = PageView::new(TOUR_PAGES.len(), move |index| tour_panel(index, palette))
        .width(width)
        .flex(1.0)
        .page(page)
        .on_page_changed(Msg::TourPage);

    let picker = Pagination::new(page + 1, TOUR_PAGES.len(), |p| Msg::TourPage(p - 1));
    let position = text(format!("Panel {} of {}", page + 1, TOUR_PAGES.len()))
        .size(13.0)
        .color(theme.muted);
    let footer = Container::new()
        .width(width)
        .padding(20.0)
        .child(column![picker, position].gap(10.0).align(Align::Center));

    let screen = column![
        NavigationBar::new("Guided tour").on_back(Msg::Pop),
        pages,
        footer
    ]
    .width(width)
    .height(height);
    Box::new(
        Container::new()
            .width(width)
            .height(height)
            .color(theme.background)
            .child(screen),
    )
}
