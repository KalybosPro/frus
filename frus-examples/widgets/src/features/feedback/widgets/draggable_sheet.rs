//! `DraggableScrollableSheet` — a panel along the bottom that follows the finger between
//! the heights it may rest at, and hands the gesture over to the list inside it (the
//! "nearby places" sheet of a map). It is not modal: the page behind stays usable.

use frus::{
    host, keyed, Align, Button, Column, Container, Curve, DraggableScrollableSheet, Flex,
    FontWeight, Justify, Row, SheetTo, SingleChildScrollView, Stack, State, StateContext,
    StatefulWidget, Text, Variant, Widget, Wrap,
};

use crate::common::shared::{section, sections};

/// The preview is phone-sized: it fits a phone's page and reads like a screen.
const PREVIEW_WIDTH: f32 = 320.0;

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(SheetDemo.into_widget())
}

/// The key that lets code move the sheet (`host::sheet_to`).
const PLACES_SHEET: &str = "places-sheet";

const PLACES: [&str; 10] = [
    "Bakery", "Bookshop", "Café", "Cinema", "Library", "Market", "Museum", "Park", "Pharmacy",
    "Station",
];

struct SheetDemo;

struct SheetDemoState {
    /// Lowered all the way, the sheet sends `on_dismiss` and the page removes it.
    hidden: bool,
}

impl StatefulWidget for SheetDemo {
    type State = SheetDemoState;

    fn create_state(&self) -> SheetDemoState {
        SheetDemoState { hidden: false }
    }
}

impl State for SheetDemoState {
    type Widget = SheetDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();

        // The sheet draws nothing itself: the surface, its corners and the drag handle
        // are the content's.
        let list = PLACES.iter().fold(
            Column::new().cross_axis_alignment(Align::Start),
            |list, place| list.child(Container::new().padding(12.0).child(Text::new(*place))),
        );
        let handle = Row::new().main_axis_alignment(Justify::Center).child(
            Container::new()
                .width(36.0)
                .height(4.0)
                .radius(2.0)
                .color(theme.muted)
                .margin(10.0),
        );
        let panel = Container::new()
            .color(theme.scheme.surface_container_high)
            .radius(20.0)
            // The list scrolls inside the sheet: the column fills the panel and the
            // scroll view takes what the handle and the title leave.
            .child(
                Flex::column()
                    .flex(1.0)
                    .align(Align::Stretch)
                    .child(handle)
                    .child(
                        Container::new().padding(12.0).child(
                            Text::new("Nearby places")
                                .size(18.0)
                                .weight(FontWeight::SemiBold),
                        ),
                    )
                    .child(SingleChildScrollView::new().flex(1.0).child(list)),
            );

        // The sheet fills the box it is in and sits along its bottom. Its heights are
        // shares of that box: it rests at a quarter, a half or all of it.
        let mut map = Stack::new().width(PREVIEW_WIDTH).height(320.0).layer(
            Container::new()
                .width(PREVIEW_WIDTH)
                .height(320.0)
                .radius(12.0)
                .color(theme.scheme.surface_container)
                .padding(16.0)
                .child(Text::new("The page behind the sheet").color(theme.muted)),
        );
        if !self.hidden {
            map = map.layer(keyed(
                PLACES_SHEET,
                DraggableScrollableSheet::new(panel)
                    .initial(0.5)
                    .min(0.25)
                    .snap_sizes([0.5])
                    .on_dismiss(cx.callback(|s: &mut SheetDemoState| s.hidden = true)),
            ));
        }

        let controls = Wrap::new()
            .gap(8.0)
            .run_gap(8.0)
            .child(
                Button::new("Show the sheet")
                    .enabled(self.hidden)
                    .on_press(cx.callback(|s: &mut SheetDemoState| s.hidden = false)),
            )
            // Moving the sheet from code is an effect, not state: where it rests
            // afterwards is the sheet's own, as if a finger had put it there.
            .child(
                Button::new("Raise it")
                    .variant(Variant::Outlined)
                    .on_press(|| {
                        host::sheet_to(PLACES_SHEET, SheetTo::size(1.0).animate(0.3, Curve::ease()))
                    }),
            );

        sections(vec![Box::new(section(
            "Drag the handle or the list",
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Stretch)
                .child(controls)
                .child(map),
        ))])
    }
}
