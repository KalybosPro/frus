//! The draggable sheet screen (milestone 515): a sheet over a page, dragged between a
//! quarter, half and the whole of it, holding a list long enough to scroll at every one.

use crate::prelude::*;

/// What the sheet lists: enough to scroll however tall the sheet is.
pub(crate) const PLACES: [&str; 20] = [
    "Harbour market",
    "Old lighthouse",
    "Botanical garden",
    "Night bakery",
    "River ferry",
    "Glass museum",
    "Hill observatory",
    "Tram depot café",
    "Salt flats",
    "Clock tower",
    "Paper mill",
    "Pine forest trail",
    "Covered bridge",
    "Jazz cellar",
    "Fish auction",
    "Windmill row",
    "Rooftop cinema",
    "Stone circle",
    "Ice rink",
    "Cable car",
];

/// The screen: a page, and the sheet over it until a flick down puts it away.
pub(crate) fn sheet_screen(app: &TodoApp, theme: &Theme) -> Box<dyn Widget<Msg>> {
    let page = Container::new().padding(24.0).child(
        Flex::column()
            .gap(12.0)
            .child(
                text("A sheet over the page")
                    .size(20.0)
                    .color(theme.on_surface),
            )
            .child(
                text(
                    "Drag it by its handle or by its list. It rests at a quarter, at half and \
                     at full height, and a flick down from the lowest puts it away.",
                )
                .size(15.0)
                .color(theme.muted),
            )
            .child(
                button("Show the sheet", Msg::ShowPlaces)
                    .variant(Variant::Filled)
                    .size(15.0),
            ),
    );
    let mut stack = Stack::new().flex(1.0).layer(page);
    if !app.places_hidden {
        stack = stack.layer(
            frus_widgets::DraggableScrollableSheet::new(places_panel(theme))
                .min(0.25)
                .snap_sizes([0.5])
                .on_dismiss(Msg::PlacesDismissed),
        );
    }
    Scaffold::new()
        .background(theme.background)
        .app_bar(NavigationBar::new("Draggable sheet").on_back(Msg::Pop))
        .body(stack)
        .build()
}

/// What the sheet holds: a handle, a heading and the list. The sheet draws nothing of its
/// own, so the surface and its rounding are the content's.
fn places_panel(theme: &Theme) -> Container<Msg> {
    let list = PLACES
        .iter()
        .enumerate()
        .fold(Flex::column(), |list, (index, place)| {
            list.child(
                Container::new().padding(16.0).child(
                    text(format!("{}. {place}", index + 1))
                        .size(16.0)
                        .color(theme.on_surface),
                ),
            )
        });
    let handle = Flex::row().justify(Justify::Center).padding(12.0).child(
        Container::new()
            .width(36.0)
            .height(4.0)
            .radius(2.0)
            .color(theme.muted),
    );
    Container::new()
        .color(theme.scheme.surface_container_low)
        .radius(20.0)
        .child(
            Flex::column()
                .flex(1.0)
                .child(handle)
                .child(
                    Container::new()
                        .padding(16.0)
                        .child(text("Nearby places").size(20.0).color(theme.on_surface)),
                )
                // The sheet reaches the bottom of the window, under the system's navigation
                // bar, and draws there — as the reference's does. What must stay clear of
                // the bar is the content, so the list's end is padded by the bottom inset:
                // seen on a phone, the last place sat under the buttons, out of reach.
                .child(
                    SingleChildScrollView::new()
                        .flex(1.0)
                        .padding_each(0.0, 0.0, MediaQuery::of().padding.bottom, 0.0)
                        .child(list),
                ),
        )
}
