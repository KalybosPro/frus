//! `AboutDialog`, `AboutListTile` and `LicensePage` — the "about this app" box and the
//! page listing the licences of everything the app ships. Every distributed app owes
//! that page; `frus::licenses::add_all(include_str!(..))` fills it from a file generated
//! out of the app's real dependency graph.

use frus::{
    AboutDialog, AboutListTile, Align, Column, ConstrainedBox, Icon, Icons, LicenseNotice,
    LicensePage, Package, State, StateContext, StatefulWidget, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(AboutDemo.into_widget())
}

/// Notices of our own, so the page has something to show. A real app registers the
/// generated file at start-up instead, and `LicensePage::new` reads that registry.
fn sample_notices() -> Vec<LicenseNotice> {
    vec![
        LicenseNotice {
            packages: vec![
                Package::new("frus", "0.3.0"),
                Package::new("frus-widgets", "0.3.0"),
            ],
            text: "MIT License\n\nPermission is hereby granted, free of charge, to any person \
                   obtaining a copy of this software…"
                .into(),
        },
        LicenseNotice {
            packages: vec![Package::new("serde", "1.0.228")],
            text: "Apache License, Version 2.0…".into(),
        },
    ]
}

struct AboutDemo;

struct AboutDemoState {
    about_open: bool,
    showing_licences: bool,
    /// Which package's licence the page has open; `None` is the list.
    package: Option<usize>,
}

impl StatefulWidget for AboutDemo {
    type State = AboutDemoState;

    fn create_state(&self) -> AboutDemoState {
        AboutDemoState {
            about_open: false,
            showing_licences: false,
            package: None,
        }
    }
}

impl State for AboutDemoState {
    type Widget = AboutDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let mut column = Column::new()
            .spacing(24.0)
            .cross_axis_alignment(Align::Stretch);

        // The row a settings screen or a drawer ends with. It only says "open"; the
        // application opens the box.
        column = column.child(section(
            "AboutListTile",
            AboutListTile::new(cx.callback(|s: &mut AboutDemoState| s.about_open = true))
                .application("frus widgets")
                .subtitle("Version 0.1.0")
                .build(),
        ));

        // The page is a column and does not scroll itself: in an app it goes in the
        // page's scroll view, usually on its own route.
        if self.showing_licences {
            column = column.child(section(
                "LicensePage",
                LicensePage::new(
                    self.package,
                    cx.handler(|s: &mut AboutDemoState, which: Option<usize>| s.package = which),
                )
                .notices(sample_notices())
                .application("frus widgets")
                .version("0.1.0")
                .legalese("The gallery itself is MIT OR Apache-2.0.")
                .build(),
            ));
        }

        let body = ConstrainedBox::new(column).max_width(480.0);

        // An alert dialog underneath: name, version, legal line, a way to the licences and
        // a way out. `on_close` also closes it on a tap outside.
        let about = AboutDialog::new(self.about_open)
            .application("frus widgets")
            .version("Version 0.1.0")
            .legalese("A gallery of every frus widget.")
            .icon(Icon::new(Icons::INFO))
            .on_licences(cx.callback(|s: &mut AboutDemoState| {
                s.about_open = false;
                s.showing_licences = true;
            }))
            .on_close(cx.callback(|s: &mut AboutDemoState| s.about_open = false))
            .body(body);

        sections(vec![Box::new(about)])
    }
}
