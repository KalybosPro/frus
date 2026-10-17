//! `RefreshIndicator` — pull a list down past its top to ask for fresh data. The app
//! decides when the answer has arrived: the spinner turns for as long as it says
//! `refreshing(true)`.

use std::time::Duration;

use frus::{Color, Container, ListTile, ListView, RefreshIndicator, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let loading = cx.use_state(|| false);
        let version = cx.use_state(|| 1u32);

        // `on_refresh` is sent when the finger lets go past the threshold. Here a timer
        // stands in for a network call; a real app would start a request, then clear
        // `loading` when it answers.
        let (start, done, bump) = (loading.clone(), loading.clone(), version.clone());
        let on_refresh = move || {
            start.set(true);
            let (done, bump) = (done.clone(), bump.clone());
            frus::host::after(Duration::from_millis(1500), move || {
                bump.update(|v| *v += 1);
                done.set(false);
            });
        };

        let shown = version.get();
        let list = ListView::new(20, 48.0, move |i| {
            ListTile::new().title(format!("Headline {} (fetch #{shown})", i + 1))
        })
        .width(320.0)
        .height(280.0);

        sections(vec![
            Box::new(section(
                "Pull down to refresh",
                Container::new().border(1.0, Color::hex("#A1A1AA")).child(
                    RefreshIndicator::new(list)
                        .on_refresh(on_refresh)
                        .refreshing(loading.get()),
                ),
            )),
            Box::new(
                Text::new(if loading.get() {
                    "Refreshing…"
                } else {
                    "Up to date"
                })
                .color(cx.theme().muted),
            ),
        ])
    }))
}
