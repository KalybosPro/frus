//! `RadioGroup` and `Radio` — one choice out of a few, all visible at once. `RadioGroup`
//! lays out a labelled set; `Radio` is a single button for layouts of your own.

use frus::{Color, Radio, RadioGroup, Row, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(RadioDemo.into_widget())
}

const SHIPPING: [&str; 3] = ["Standard (5 days)", "Express (2 days)", "Pick up in store"];
const SIZES: [&str; 3] = ["S", "M", "L"];

struct RadioDemo;

struct RadioDemoState {
    shipping: usize,
    size: usize,
}

impl StatefulWidget for RadioDemo {
    type State = RadioDemoState;

    fn create_state(&self) -> RadioDemoState {
        RadioDemoState {
            shipping: 0,
            size: 1,
        }
    }
}

impl State for RadioDemoState {
    type Widget = RadioDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The group reports the index of the option picked.
        let group = SHIPPING.iter().fold(
            RadioGroup::new(
                self.shipping,
                cx.handler(|s: &mut RadioDemoState, i: usize| s.shipping = i),
            ),
            |group, option| group.option(*option),
        );

        // Single radios, laid out in a row: each is told whether it is the selected
        // one and what to send when chosen.
        let sizes = SIZES
            .iter()
            .enumerate()
            .fold(Row::new().spacing(16.0), |row, (i, size)| {
                row.child(
                    Radio::new(self.size == i)
                        .label(*size)
                        .selected_color(Color::hex("#DB2777"))
                        .on_select(cx.callback(move |s: &mut RadioDemoState| s.size = i)),
                )
            });

        sections(vec![
            Box::new(section("RadioGroup", group)),
            Box::new(Text::new(format!("Shipping: {}", SHIPPING[self.shipping]))),
            Box::new(section("Radio, in a row, custom colour", sizes)),
            Box::new(section(
                "Disabled",
                RadioGroup::new(0, cx.handler(|_: &mut RadioDemoState, _: usize| {}))
                    .option("Only option")
                    .option("Not available")
                    .enabled(false),
            )),
        ])
    }
}
