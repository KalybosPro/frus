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

pub(crate) fn tour_screen(app: &TodoApp, theme: &Theme) -> Box<dyn Widget<Msg>> {
    // The window this screen fills, read from the surface description in force:
    // nothing hands it down any more.
    let Size { width, height } = surface();
    let last = TOUR_PAGES.len() - 1;
    let page = app.tour_page.min(last);
    let palette = theme.clone();
    let pages = PageView::new(TOUR_PAGES.len(), move |index| {
        tour_panel(index, palette.clone())
    })
    .width(width)
    .flex(1.0)
    .page(page)
    .on_page_changed(Msg::TourPage);

    // **A rail that fills as the panels go by.** The quantity is a *fraction* and not a
    // width: nobody here knows how wide the footer comes out, and a window resized in the
    // middle of the movement is answered by the layout rather than by a number that was
    // right when it was written (milestone 495).
    let rail = Container::new()
        .width(width - 40.0)
        .height(4.0)
        .radius(2.0)
        .color(theme.border)
        .child(
            AnimatedFractionallySizedBox::new(
                0.25,
                Curve::ease_out(),
                // A fractional box sizes **itself** as a share of its parent and leaves
                // its child to fill it, so the child has to say it fills: a bare box
                // here is a rail nought pixels tall.
                Container::new()
                    .flex(1.0)
                    .height(4.0)
                    .radius(2.0)
                    .color(theme.primary),
            )
            .width_factor((page + 1) as f32 / TOUR_PAGES.len() as f32),
        );

    let picker = Pagination::new(page + 1, TOUR_PAGES.len(), |p| Msg::TourPage(p - 1));
    let position = text(format!("Panel {} of {}", page + 1, TOUR_PAGES.len()))
        .size(13.0)
        .color(theme.muted);
    // The dots (milestone 480) and the pager say the same thing and are not the same
    // thing: the dots are a **read-out** — twelve pixels is not a target — and the pager
    // is the control. Side by side is the point, and the dots cross with the swipe
    // because they are on the same fractional index the page view settles on.
    let dots = TabPageSelector::new(TOUR_PAGES.len(), page);
    let footer = Container::new().width(width).padding(20.0).child(
        column![rail, dots, picker, position]
            .gap(10.0)
            .align(Align::Center),
    );

    // **The way out leaves once there is nothing left to skip.** Both ends of the
    // movement pin the same edge — `top`, at 12 and then above the panel entirely — which
    // is what makes it a movement at all: a pin that is set at one end and unset at the
    // other changes what the layer *is*, and takes effect at once rather than travelling
    // (milestone 495).
    let skip = AnimatedPositioned::new(
        0.25,
        Curve::ease_out(),
        button("Skip", Msg::TourPage(last))
            .variant(Variant::Text)
            .size(14.0),
    )
    .top(if page == last { -56.0 } else { 12.0 })
    .right(12.0)
    .height(40.0);
    let panels = Stack::new().layer(pages).layer(skip).flex(1.0);

    let screen = column![
        NavigationBar::new("Guided tour").on_back(Msg::Pop),
        panels,
        footer
    ]
    .flex(1.0);
    Box::new(
        Container::new()
            .width(width)
            .height(height)
            .color(theme.background)
            // The background runs **under** the bars; the content does not. `SafeArea`
            // reads the intrusions from the surface description, so a screen with no
            // `Scaffold` to do it for it still keeps clear of the notch.
            .child(SafeArea::new(screen)),
    )
}
