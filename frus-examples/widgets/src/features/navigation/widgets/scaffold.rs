//! `Scaffold` — the shell of a screen. You fill its slots (app bar, body, navigation,
//! drawer, floating action button, persistent footer) and it lays them out, keeping
//! clear of the system bars and the keyboard.

use frus::{
    Align, AppBar, Button, Center, Color, Column, Container, FabLocation, FloatingActionButton,
    Icons, ListTile, MediaQuery, Row, Scaffold, SegmentedButton, Size, State, StateContext,
    StatefulWidget, Switch, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(ScaffoldDemo.into_widget())
}

struct ScaffoldDemo;

#[derive(Default)]
struct ScaffoldDemoState {
    /// Index into `LOCATIONS`.
    fab_at: usize,
    drawer_open: bool,
    footer: bool,
    tab: usize,
    count: u32,
}

const LOCATIONS: [(&str, FabLocation); 3] = [
    ("End", FabLocation::EndFloat),
    ("Center", FabLocation::CenterFloat),
    ("Start", FabLocation::StartFloat),
];

impl StatefulWidget for ScaffoldDemo {
    type State = ScaffoldDemoState;

    fn create_state(&self) -> ScaffoldDemoState {
        ScaffoldDemoState::default()
    }
}

impl State for ScaffoldDemoState {
    type Widget = ScaffoldDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme();
        let toggle_drawer = cx.callback(|s: &mut Self| s.drawer_open = !s.drawer_open);

        // The screen is built under a description of a 320 × 480 surface with no system
        // bars: the scaffold and its slots read the screen (`MediaQuery::of`) when they
        // are made, so a shell shown in part of a page — a preview, a device frame — is
        // made inside `scope`. A real screen writes none of this.
        let screen = MediaQuery::new(Size::new(320.0, 480.0)).scope(|| {
            // No leading button: a scaffold with a drawer gives its app bar the button that
            // opens it.
            let app_bar = AppBar::new().title(Text::new("Notes")).build();

            let body = Center::new(
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(Text::new(["Notes", "Reminders"][self.tab]).size(20.0))
                    .child(Text::new(format!("{} added", self.count)).color(theme.muted)),
            );

            let drawer = Column::new()
                .cross_axis_alignment(Align::Start)
                .child(
                    Container::new()
                        .padding(16.0)
                        .child(Text::new("Folders").size(18.0)),
                )
                .child(
                    ListTile::new()
                        .title("Personal")
                        .on_tap(toggle_drawer.clone()),
                )
                .child(ListTile::new().title("Work").on_tap(toggle_drawer.clone()));

            let mut shell = Scaffold::new()
                // Without a background the scaffold is transparent.
                .background(theme.background)
                .app_bar(app_bar)
                .body(body)
                // Destinations make a bottom navigation bar.
                .nav(self.tab, cx.handler(|s: &mut Self, i: usize| s.tab = i))
                .destination(Icons::NOTE, "Notes")
                .destination(Icons::ALARM, "Reminders")
                .drawer(drawer, self.drawer_open, toggle_drawer)
                .fab(
                    FloatingActionButton::new(Icons::ADD)
                        .on_press(cx.callback(|s: &mut Self| s.count += 1)),
                )
                .fab_location(LOCATIONS[self.fab_at].1);
            if self.footer {
                // Always visible above the navigation: the screen's committing actions.
                shell = shell.persistent_footer(
                    Row::new()
                        .spacing(8.0)
                        .main_axis_size(frus::MainAxisSize::Min)
                        .child(Button::new("Discard").variant(Variant::Text))
                        .child(Button::new("Save")),
                );
            }
            shell.build()
        });

        let fab_picker = LOCATIONS.iter().fold(
            SegmentedButton::new(
                self.fab_at,
                cx.handler(|s: &mut Self, i: usize| s.fab_at = i),
            ),
            |picker, (name, _)| picker.segment(*name),
        );

        sections(vec![
            Box::new(section("Floating action button location", fab_picker)),
            Box::new(section(
                "Persistent footer",
                Switch::new(self.footer)
                    .on_toggle(cx.handler(|s: &mut Self, on: bool| s.footer = on)),
            )),
            Box::new(section(
                "The screen (the menu button opens the drawer)",
                Container::new()
                    // The 1 px border sits inside the box, so the box is the screen plus 2 px.
                    .width(322.0)
                    .height(482.0)
                    .border(1.0, Color::hex("#A1A1AA"))
                    .clip()
                    .child(screen),
            )),
        ])
    }
}
