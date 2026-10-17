//! `AnimatedSwitcher` — when the value it shows changes, the old child leaves as the new
//! one arrives: a fade by default, or any transition you hand it.
//!
//! It is given the **value** a child is made from, not the child: values that compare
//! equal are the same child and nothing moves; a different value is a switch.

use frus::{
    component, Align, AnimatedSwitcher, Button, Column, Curve, FadeTransition, FontWeight,
    IconButton, Icons, Row, ScaleTransition, SlideTransition, Text, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let count = cx.use_state(|| 0_i32);
        let n = count.get();
        let primary = cx.theme().primary;

        // One number, three ways to change it on screen.
        let number = move |value: &i32| {
            Text::new(value.to_string())
                .size(40.0)
                .weight(FontWeight::Bold)
                .color(primary)
        };

        let (down, up) = (count.clone(), count.clone());
        let controls = Row::new()
            .spacing(8.0)
            .cross_axis_alignment(Align::Center)
            .child(
                IconButton::new(Icons::REMOVE)
                    .label("Decrement")
                    .on_press(move || down.update(|n| *n -= 1)),
            )
            .child(
                IconButton::new(Icons::ADD)
                    .label("Increment")
                    .on_press(move || up.update(|n| *n += 1)),
            );

        let shown = |title: &str, switcher: AnimatedSwitcher| {
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Center)
                .child(Text::new(title).size(12.0))
                .child(switcher)
        };

        sections(vec![
            Box::new(section("Change the value", controls)),
            Box::new(section(
                "Three transitions",
                Wrap::new()
                    .gap(40.0)
                    .run_gap(16.0)
                    // The default: the old child fades out as the new one fades in.
                    .child(shown("Fade", AnimatedSwitcher::new(0.3, n, number)))
                    // `transition` is handed the child and its progress (1 = in place,
                    // 0 = gone), for the arriving child and the leaving one alike.
                    .child(shown(
                        "Scale + fade",
                        AnimatedSwitcher::new(0.3, n, number)
                            .switch_in_curve(Curve::ease_out())
                            .transition(|child, t| {
                                ScaleTransition::new(0.5 + 0.5 * t, FadeTransition::new(t, child))
                            }),
                    ))
                    // Any explicit transition fits: here the digits slide up into place.
                    .child(shown(
                        "Slide",
                        AnimatedSwitcher::new(0.3, n, number).transition(|child, t| {
                            SlideTransition::new(0.0, 1.0 - t, FadeTransition::new(t, child))
                        }),
                    )),
            )),
            Box::new(section(
                "Reset",
                Button::new("Back to zero").on_press(move || count.set(0)),
            )),
        ])
    }))
}
