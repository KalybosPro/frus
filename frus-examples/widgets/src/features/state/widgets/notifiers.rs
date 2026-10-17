//! `ValueNotifier`, `ChangeNotifier` and the builders that follow them —
//! `ValueListenableBuilder` and `ListenableBuilder`.
//!
//! A notifier is state that lives **outside** the widgets — in a model, a service, a
//! controller — and tells whoever cares when it changed. Cloning one gives another
//! handle to the same value, so it can be handed to every widget that needs it.

use std::cell::RefCell;
use std::rc::Rc;

use frus::{
    component, Align, Button, ChangeNotifier, Column, Listenable, ListenableBuilder, Row, Text,
    ValueListenableBuilder, ValueNotifier, Variant, Widget,
};

use crate::common::shared::{section, sections};

/// A small model with a `ChangeNotifier` inside: its own methods change it and say so.
#[derive(Clone, Default)]
struct Basket {
    items: Rc<RefCell<Vec<&'static str>>>,
    changed: ChangeNotifier,
}

impl Basket {
    fn add(&self, item: &'static str) {
        self.items.borrow_mut().push(item);
        self.changed.notify_listeners();
    }

    fn clear(&self) {
        self.items.borrow_mut().clear();
        self.changed.notify_listeners();
    }

    fn summary(&self) -> String {
        let items = self.items.borrow();
        if items.is_empty() {
            "The basket is empty".to_string()
        } else {
            format!("{} items: {}", items.len(), items.join(", "))
        }
    }
}

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        // In an app these would live in a service or a page's state; `use_ref` keeps them
        // for the demo's lifetime.
        let volume = cx.use_ref(|| ValueNotifier::new(3_i32)).get();
        let basket = cx.use_ref(Basket::default).get();

        // Listening by hand: a subscription made in an effect, dropped by its cleanup.
        let notifications = cx.use_state(|| 0_u32);
        {
            let (basket, heard) = (basket.clone(), notifications.clone());
            cx.use_effect((), move || {
                let subscription = basket
                    .changed
                    .add_listener(move || heard.update(|n| *n += 1));
                move || drop(subscription)
            });
        }

        let (down, up) = (volume.clone(), volume.clone());
        let volume_controls = Row::new()
            .spacing(8.0)
            .cross_axis_alignment(Align::Center)
            .child(
                Button::new("−")
                    .variant(Variant::Outlined)
                    .on_press(move || down.update(|v| *v = (*v - 1).max(0))),
            )
            .child(
                Button::new("+")
                    .variant(Variant::Outlined)
                    .on_press(move || up.update(|v| *v = (*v + 1).min(10))),
            )
            // Rebuilt from the notifier's value whenever it changes.
            .child(ValueListenableBuilder::new(
                volume.clone(),
                |cx, level: &i32| {
                    Box::new(
                        Text::new(format!("Volume {level} {}", "▮".repeat(*level as usize)))
                            .color(cx.theme().primary),
                    )
                },
            ));

        let (apples, pears, emptied) = (basket.clone(), basket.clone(), basket.clone());
        let basket_controls = Row::new()
            .spacing(8.0)
            .child(
                Button::new("Add an apple")
                    .variant(Variant::Tonal)
                    .on_press(move || apples.add("apple")),
            )
            .child(
                Button::new("Add a pear")
                    .variant(Variant::Tonal)
                    .on_press(move || pears.add("pear")),
            )
            .child(
                Button::new("Empty")
                    .variant(Variant::Text)
                    .on_press(move || emptied.clear()),
            );

        sections(vec![
            Box::new(section(
                "ValueNotifier + ValueListenableBuilder",
                volume_controls,
            )),
            Box::new(section(
                "ChangeNotifier in a model + ListenableBuilder",
                Column::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Start)
                    .child(basket_controls)
                    // Follows the model's notifier, and reads the model when it fires.
                    .child({
                        let basket = basket.clone();
                        ListenableBuilder::new(basket.changed.clone(), move |_, _| {
                            Box::new(Text::new(basket.summary()))
                        })
                    }),
            )),
            Box::new(section(
                "add_listener",
                Text::new(format!(
                    "The basket has notified {} times",
                    notifications.get()
                ))
                .size(13.0),
            )),
        ])
    }))
}
