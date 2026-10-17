//! `ExpansionPanelList` — a column of panels drawn as one surface that splits apart as a
//! panel opens: a settings screen, an FAQ, the steps of a checkout.

use frus::{Callback, Container, ExpansionPanel, ExpansionPanelList, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let open = cx.use_state(|| vec![0usize]);
        let radio = cx.use_state(|| Some(1usize));

        // `new`: any number open. The message says which panel and whether it is now
        // open; the app keeps the set.
        let toggle = open.clone();
        let free = ExpansionPanelList::new(&open.get(), move |index: usize, now_open: bool| {
            let toggle = toggle.clone();
            Callback::new(move || {
                toggle.update(|set| {
                    set.retain(|&i| i != index);
                    if now_open {
                        set.push(index);
                    }
                })
            })
        })
        .panel(ExpansionPanel::new(
            "Delivery",
            Text::new("Two parcels, one signature."),
        ))
        .panel(
            ExpansionPanel::new("Payment", Text::new("Visa ending 4242.")).subtitle("Card on file"),
        )
        .panel(ExpansionPanel::new(
            "Gift options",
            Text::new("Wrapping and a note."),
        ));

        // `radio`: at most one open. Its message carries the resulting state — `Some(i)`
        // or `None` — so the app just stores it.
        let set = radio.clone();
        let exclusive = ExpansionPanelList::radio(radio.get(), move |state: Option<usize>| {
            let set = set.clone();
            Callback::new(move || set.set(state))
        })
        .panel(ExpansionPanel::new(
            "Account",
            Text::new("Name, e-mail, password."),
        ))
        .panel(ExpansionPanel::new(
            "Notifications",
            Text::new("Push, e-mail, digest."),
        ))
        .panel(ExpansionPanel::new(
            "Privacy",
            Text::new("Who can see your activity."),
        ));

        sections(vec![
            Box::new(section(
                "Several open",
                Container::new().width(320.0).child(free),
            )),
            Box::new(section(
                "One at a time",
                Container::new().width(320.0).child(exclusive),
            )),
        ])
    }))
}
