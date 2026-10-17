//! `Checkbox` — a yes/no choice, with an optional label beside it. `Checkbox::maybe` is
//! the three-state version: on, off, or partly on (a "select all" over a list).

use frus::{
    Align, Checkbox, Color, Column, Insets, Padding, State, StateContext, StatefulWidget, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(CheckboxDemo.into_widget())
}

pub const TOPPINGS: [&str; 3] = ["Cheese", "Mushrooms", "Olives"];

pub struct CheckboxDemo;

pub struct CheckboxDemoState {
    terms: bool,
    toppings: [bool; 3],
    newsletter: bool,
}

impl CheckboxDemoState {
    /// The parent's value follows its children: all, none, or some (`None`).
    pub fn all_toppings(&self) -> Option<bool> {
        match self.toppings.iter().filter(|on| **on).count() {
            0 => Some(false),
            n if n == self.toppings.len() => Some(true),
            _ => None,
        }
    }

    /// Ticking the parent ticks every child; anything else clears them.
    pub fn set_all(&mut self, value: Option<bool>) {
        self.toppings = [value == Some(true); 3];
    }
}

impl StatefulWidget for CheckboxDemo {
    type State = CheckboxDemoState;

    fn create_state(&self) -> CheckboxDemoState {
        CheckboxDemoState {
            terms: false,
            toppings: [true, false, false],
            newsletter: true,
        }
    }
}

impl State for CheckboxDemoState {
    type Widget = CheckboxDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // `on_toggle` reports the new value; the label is part of the tap target.
        let terms = Checkbox::new(self.terms)
            .label("I accept the terms")
            .on_toggle(cx.handler(|s: &mut CheckboxDemoState, on: bool| s.terms = on));

        // A parent in three states over its children.
        let toppings = TOPPINGS.iter().enumerate().fold(
            Column::new()
                .spacing(4.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Checkbox::maybe(self.all_toppings())
                        .label("All toppings")
                        .on_change(
                            cx.handler(|s: &mut CheckboxDemoState, v: Option<bool>| s.set_all(v)),
                        ),
                ),
            |column, (i, name)| {
                // Indented under the parent, so the hierarchy reads at a glance.
                column.child(Padding::new(
                    Insets::new(0.0, 0.0, 0.0, 32.0),
                    Checkbox::new(self.toppings[i]).label(*name).on_toggle(
                        cx.handler(move |s: &mut CheckboxDemoState, on: bool| s.toppings[i] = on),
                    ),
                ))
            },
        );

        sections(vec![
            Box::new(section("Simple", terms)),
            Box::new(section("Tristate parent", toppings)),
            // `error(true)` paints it in the error colour: the terms must be accepted.
            Box::new(section(
                "Error, disabled and custom colours",
                Column::new()
                    .spacing(4.0)
                    .cross_axis_alignment(Align::Start)
                    .child(
                        Checkbox::new(self.terms)
                            .label("Required")
                            .error(!self.terms)
                            .on_toggle(
                                cx.handler(|s: &mut CheckboxDemoState, on: bool| s.terms = on),
                            ),
                    )
                    .child(Checkbox::new(true).label("Disabled").enabled(false))
                    .child(
                        Checkbox::new(self.newsletter)
                            .label("Newsletter")
                            .fill_color(Color::hex("#16A34A"))
                            .check_color(Color::hex("#FFFFFF"))
                            .radius(10.0)
                            .on_toggle(
                                cx.handler(|s: &mut CheckboxDemoState, on: bool| s.newsletter = on),
                            ),
                    ),
            )),
        ])
    }
}
