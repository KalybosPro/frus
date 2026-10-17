//! `NavigationRail` — the main destinations down the side of a tablet or desktop
//! window: glyphs alone, glyphs with labels, or extended with the labels beside them.

use frus::{
    Align, Center, Color, Container, Expanded, FloatingActionButton, IconButton, Icons, RailLabels,
    Row, SegmentedButton, State, StateContext, StatefulWidget, Switch, Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(RailDemo.into_widget())
}

struct RailDemo;

struct RailDemoState {
    selected: usize,
    /// Index into `LABELS`.
    labels: usize,
    extended: bool,
}

const PLACES: [&str; 4] = ["Inbox", "Starred", "Sent", "Drafts"];
const LABELS: [(&str, RailLabels); 3] = [
    ("None", RailLabels::None),
    ("Selected", RailLabels::Selected),
    ("All", RailLabels::All),
];

impl StatefulWidget for RailDemo {
    type State = RailDemoState;

    fn create_state(&self) -> RailDemoState {
        RailDemoState {
            selected: 0,
            labels: 2,
            extended: false,
        }
    }
}

impl State for RailDemoState {
    type Widget = RailDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // A rail has a slot above its destinations (`leading`, often the main action) and
        // one below (`trailing`). An extended rail labels every destination by itself,
        // so `labels` only matters while it is narrow.
        let rail = frus::NavigationRail::new(
            self.selected,
            cx.handler(|s: &mut Self, i: usize| s.selected = i),
        )
        .leading(FloatingActionButton::new(Icons::EDIT).small())
        .item(Icons::INBOX, "Inbox")
        .badge(4)
        .item(Icons::STAR, "Starred")
        .item(Icons::SEND, "Sent")
        .item(Icons::DRAFTS, "Drafts")
        .trailing(IconButton::new(Icons::SETTINGS).label("Settings"))
        .labels(LABELS[self.labels].1)
        .extended(self.extended);

        let window = Container::new()
            .width(320.0)
            .height(440.0)
            .border(1.0, Color::hex("#A1A1AA"))
            .clip()
            .child(
                Row::new()
                    .cross_axis_alignment(Align::Stretch)
                    .child(rail)
                    .child(Expanded::new(Center::new(
                        Text::new(PLACES[self.selected]).size(20.0),
                    ))),
            );

        let picker = LABELS.iter().fold(
            SegmentedButton::new(
                self.labels,
                cx.handler(|s: &mut Self, i: usize| s.labels = i),
            ),
            |picker, (name, _)| picker.segment(*name),
        );

        sections(vec![
            Box::new(section("Labels", picker)),
            Box::new(section(
                "Extended",
                Switch::new(self.extended)
                    .on_toggle(cx.handler(|s: &mut Self, on: bool| s.extended = on)),
            )),
            Box::new(window),
        ])
    }
}
