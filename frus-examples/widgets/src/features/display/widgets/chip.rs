//! `Chip` — a compact element for an attribute, a filter, a choice or an entry. One
//! widget; what makes it one kind or another is what it can do: be selected, carry an
//! icon, be removed, be pressed.

use frus::{Align, Chip, Column, Icons, Text, Widget, Wrap};

use crate::common::shared::{section, sections};

const TOPPINGS: [&str; 4] = ["Cheese", "Olives", "Basil", "Mushrooms"];
const SIZES: [&str; 3] = ["Small", "Medium", "Large"];

pub(crate) fn example() -> Box<dyn Widget> {
    // `frus::component` turns a closure into a widget that can keep state with hooks:
    // each `use_state` value survives rebuilds, and setting it rebuilds the component.
    Box::new(frus::component(|cx| {
        let toppings = cx.use_state(|| vec![true, false, false, false]);
        let size = cx.use_state(|| 1usize);
        let guests = cx.use_state(|| vec!["Ada", "Alan", "Grace"]);
        let presses = cx.use_state(|| 0u32);

        // Filter chips: each one toggles independently, and a checkmark shows the ones on.
        let filters = TOPPINGS.iter().enumerate().fold(
            Wrap::new().gap(8.0).run_gap(8.0),
            |row, (i, name)| {
                let toppings_handle = toppings.clone();
                row.child(
                    Chip::new(*name)
                        .selected(toppings.with(|on| on[i]))
                        .on_press(move || toppings_handle.update(|on| on[i] = !on[i])),
                )
            },
        );

        // Choice chips: exactly one is selected. No checkmark, the fill says it.
        let choices =
            SIZES
                .iter()
                .enumerate()
                .fold(Wrap::new().gap(8.0).run_gap(8.0), |row, (i, name)| {
                    let size_handle = size.clone();
                    row.child(
                        Chip::new(*name)
                            .selected(size.get() == i)
                            .show_checkmark(false)
                            .on_press(move || size_handle.set(i)),
                    )
                });

        // Input chips: entries the user added, each with a leading icon and a delete cross.
        let entries =
            guests
                .get()
                .into_iter()
                .fold(Wrap::new().gap(8.0).run_gap(8.0), |row, name| {
                    let guests_handle = guests.clone();
                    row.child(Chip::new(name).leading(Icons::PERSON).on_remove(move || {
                        guests_handle.update(|list| list.retain(|g| *g != name))
                    }))
                });

        // Action chips: a pressable shortcut, like a small button.
        let presses_handle = presses.clone();
        let actions = Wrap::new()
            .gap(8.0)
            .run_gap(8.0)
            .child(
                Chip::new("Add to calendar")
                    .leading(Icons::EVENT)
                    .on_press(move || presses_handle.update(|n| *n += 1)),
            )
            // A disabled chip stays visible, dimmed, and ignores presses.
            .child(
                Chip::new("Unavailable")
                    .leading(Icons::BLOCK)
                    .enabled(false),
            );

        let chosen: Vec<&str> = TOPPINGS
            .iter()
            .zip(toppings.get())
            .filter_map(|(name, on)| on.then_some(*name))
            .collect();

        sections(vec![
            Box::new(section(
                "Filter chips",
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(filters)
                    .child(Text::new(format!("Toppings: {}", chosen.join(", ")))),
            )),
            Box::new(section(
                "Choice chips",
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(choices)
                    .child(Text::new(format!("Size: {}", SIZES[size.get()]))),
            )),
            Box::new(section(
                "Input chips (removable)",
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(entries)
                    .child(Text::new(format!("{} guests left", guests.with(Vec::len)))),
            )),
            Box::new(section(
                "Action chips",
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(actions)
                    .child(Text::new(format!("Pressed {} times", presses.get()))),
            )),
        ])
    }))
}
