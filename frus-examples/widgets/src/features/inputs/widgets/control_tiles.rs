//! `CheckboxListTile`, `RadioListTile` and `SwitchListTile` — list rows whose **whole
//! width** works one control. For settings screens: a tap anywhere on the row toggles,
//! not only on the 20-pixel box.

use frus::{
    Align, CheckboxListTile, Color, Column, ConstrainedBox, ControlAffinity, Icon, Icons,
    RadioListTile, State, StateContext, StatefulWidget, SwitchListTile, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(TilesDemo.into_widget())
}

const THEMES: [&str; 3] = ["System", "Light", "Dark"];

struct TilesDemo;

struct TilesDemoState {
    wifi: bool,
    bluetooth: bool,
    sync: bool,
    backup: bool,
    theme: usize,
}

impl StatefulWidget for TilesDemo {
    type State = TilesDemoState;

    fn create_state(&self) -> TilesDemoState {
        TilesDemoState {
            wifi: true,
            bluetooth: false,
            sync: true,
            backup: false,
            theme: 0,
        }
    }
}

impl State for TilesDemoState {
    type Widget = TilesDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The rows take the message to send, not the new value: flip the field in it.
        let switches = Column::new()
            .cross_axis_alignment(Align::Stretch)
            .child(
                SwitchListTile::new(
                    self.wifi,
                    cx.callback(|s: &mut TilesDemoState| s.wifi = !s.wifi),
                )
                .title("Wi-Fi")
                .subtitle(if self.wifi { "Connected" } else { "Off" })
                .secondary(Icon::new(Icons::WIFI)),
            )
            .child(
                SwitchListTile::new(
                    self.bluetooth,
                    cx.callback(|s: &mut TilesDemoState| s.bluetooth = !s.bluetooth),
                )
                .title("Bluetooth")
                .secondary(Icon::new(Icons::BLUETOOTH))
                .active_color(Color::hex("#2563EB")),
            );

        // The control goes at the end by default; `Leading` puts it in front, where a
        // checkbox in a list usually sits.
        let checkboxes = Column::new()
            .cross_axis_alignment(Align::Stretch)
            .child(
                CheckboxListTile::new(
                    self.sync,
                    cx.callback(|s: &mut TilesDemoState| s.sync = !s.sync),
                )
                .title("Sync contacts")
                .subtitle("Every hour, over Wi-Fi")
                .control_affinity(ControlAffinity::Leading),
            )
            .child(
                CheckboxListTile::new(
                    self.backup,
                    cx.callback(|s: &mut TilesDemoState| s.backup = !s.backup),
                )
                .title("Back up photos")
                .control_affinity(ControlAffinity::Leading)
                .enabled(self.sync),
            );

        // Radio rows: each is "selected or not"; the state keeps which one is.
        let radios = THEMES.iter().enumerate().fold(
            Column::new().cross_axis_alignment(Align::Stretch),
            |column, (i, name)| {
                column.child(
                    RadioListTile::new(
                        self.theme == i,
                        cx.callback(move |s: &mut TilesDemoState| s.theme = i),
                    )
                    .title(*name)
                    .control_affinity(ControlAffinity::Leading),
                )
            },
        );

        // A list row stretches to the width it is given; `ConstrainedBox` bounds it.
        sections(vec![
            Box::new(section(
                "SwitchListTile",
                ConstrainedBox::new(switches).max_width(400.0),
            )),
            Box::new(section(
                "CheckboxListTile (the second needs the first)",
                ConstrainedBox::new(checkboxes).max_width(400.0),
            )),
            Box::new(section(
                "RadioListTile",
                ConstrainedBox::new(radios).max_width(400.0),
            )),
        ])
    }
}
