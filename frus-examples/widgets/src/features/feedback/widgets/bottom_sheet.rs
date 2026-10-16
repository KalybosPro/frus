//! `BottomSheet` — a modal panel that slides up from the bottom of the window, over a
//! scrim, for a few contextual actions or a short form. A tap on the scrim closes it.
//!
//! In a page with a `Scaffold`, `Scaffold::bottom_sheet(panel, open, toggle)` does the
//! same in one call; here the sheet is built around the demo itself.

use frus::{
    Align, BottomSheet, Button, Column, FontWeight, Icon, Icons, Insets, ListTile, Padding, State,
    StateContext, StatefulWidget, Text, Widget,
};

use crate::common::shared::section;

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(BottomSheetDemo.into_widget())
}

struct BottomSheetDemo;

struct BottomSheetDemoState {
    /// The sheet is controlled: open while this says so.
    open: bool,
    last: &'static str,
}

impl BottomSheetDemoState {
    fn choose(&mut self, what: &'static str) {
        self.last = what;
        self.open = false;
    }
}

impl StatefulWidget for BottomSheetDemo {
    type State = BottomSheetDemoState;

    fn create_state(&self) -> BottomSheetDemoState {
        BottomSheetDemoState {
            open: false,
            last: "nothing yet",
        }
    }
}

impl State for BottomSheetDemoState {
    type Widget = BottomSheetDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let choose =
            |what: &'static str| cx.callback(move |s: &mut BottomSheetDemoState| s.choose(what));

        // What the sheet holds: a heading and a few actions as list rows.
        let actions = Column::new()
            .cross_axis_alignment(Align::Stretch)
            .child(
                Text::new("Share photo")
                    .size(16.0)
                    .weight(FontWeight::SemiBold),
            )
            .child(
                ListTile::new()
                    .leading(Icon::new(Icons::LINK))
                    .title("Copy link")
                    .on_tap(choose("Copy link")),
            )
            .child(
                ListTile::new()
                    .leading(Icon::new(Icons::EMAIL))
                    .title("Send by email")
                    .on_tap(choose("Send by email")),
            )
            .child(
                ListTile::new()
                    .leading(Icon::new(Icons::DOWNLOAD))
                    .title("Save to device")
                    .on_tap(choose("Save to device")),
            );
        let actions = Padding::new(Insets::new(16.0, 8.0, 24.0, 8.0), actions);

        // What stays behind the sheet.
        let body = section(
            "Contextual actions",
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Button::new("Share")
                        .icon(Icon::new(Icons::SHARE))
                        .on_press(cx.callback(|s: &mut BottomSheetDemoState| s.open = true)),
                )
                .child(Text::new(format!("Chosen: {}", self.last))),
        );

        // `sheet` sets the content and `body` finishes the widget, so say everything
        // else before `body`.
        let sheet = BottomSheet::new(self.open)
            .on_dismiss(cx.callback(|s: &mut BottomSheetDemoState| s.open = false))
            .sheet(actions)
            .body(body);

        Box::new(sheet)
    }
}
