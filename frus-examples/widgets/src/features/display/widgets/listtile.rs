//! `ListTile` — one row of a list: something in front, one to three lines of text, and
//! something behind. The shape of a settings line, a contact, a menu entry.

use frus::{
    Align, CircleAvatar, Column, Container, Divider, Icon, Icons, ListTile, Switch, Text, Widget,
};

use crate::common::shared::{section, sections};

const ROOMS: [(&str, &str); 3] = [
    ("Living room", "3 lights on"),
    ("Kitchen", "All off"),
    ("Bedroom", "1 light on"),
];

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let selected = cx.use_state(|| 0usize);
        let wifi = cx.use_state(|| true);

        // `selected` tints the tile and its text with the theme's accent; `on_tap` makes
        // the whole row pressable.
        let rooms = ROOMS.iter().enumerate().fold(
            Column::new().cross_axis_alignment(Align::Stretch),
            |list, (i, (room, status))| {
                let select = selected.clone();
                list.child(
                    ListTile::new()
                        .leading(Icon::new(Icons::LIGHTBULB))
                        .title(*room)
                        .subtitle(*status)
                        .selected(selected.get() == i)
                        .on_tap(move || select.set(i)),
                )
            },
        );

        let toggle = wifi.clone();
        sections(vec![
            // One line: a title, with an icon in front and a chevron behind.
            Box::new(section(
                "One line",
                list(
                    Column::new()
                        .cross_axis_alignment(Align::Stretch)
                        .child(
                            ListTile::new()
                                .leading(Icon::new(Icons::PERSON))
                                .title("Account")
                                .trailing(Icon::new(Icons::CHEVRON_RIGHT)),
                        )
                        .child(Divider::new().height(1.0))
                        .child(
                            ListTile::new()
                                .leading(Icon::new(Icons::NOTIFICATIONS))
                                .title("Notifications")
                                .trailing(Icon::new(Icons::CHEVRON_RIGHT)),
                        ),
                ),
            )),
            // Two lines: a subtitle under the title, and any widget behind — here a live
            // switch, which keeps its own tap.
            Box::new(section(
                "Two lines, with a control",
                list(
                    ListTile::new()
                        .leading(CircleAvatar::initials("Home Network"))
                        .title("Wi-Fi")
                        .subtitle(if wifi.get() { "Connected" } else { "Off" })
                        .trailing(Switch::new(wifi.get()).on_toggle(move |on: bool| toggle.set(on))),
                ),
            )),
            // Three lines: a subtitle long enough to wrap onto a second line.
            Box::new(section(
                "Three lines",
                list(
                    ListTile::new()
                        .leading(CircleAvatar::initials("Grace Hopper"))
                        .title("Grace Hopper")
                        .subtitle("Re: compiler meeting — I moved it to Thursday so that everyone can make it.")
                        .three_line()
                        .trailing(Text::new("9:41")),
                ),
            )),
            Box::new(section("Selectable", list(rooms))),
            // Dense tiles are shorter, for a compact list; disabled ones dim and ignore taps.
            Box::new(section(
                "Dense and disabled",
                list(
                    Column::new()
                        .cross_axis_alignment(Align::Stretch)
                        .child(
                            ListTile::new()
                                .dense()
                                .leading(Icon::new(Icons::FOLDER))
                                .title("Documents")
                                .subtitle("Dense"),
                        )
                        .child(
                            ListTile::new()
                                .leading(Icon::new(Icons::LOCK))
                                .title("Admin settings")
                                .subtitle("You do not have access")
                                .enabled(false),
                        ),
                ),
            )),
        ])
    }))
}

/// Tiles fill the width they are given, so the demo gives them a list-like width.
fn list(child: impl Widget + 'static) -> Container {
    Container::new().width(320.0).child(child)
}
