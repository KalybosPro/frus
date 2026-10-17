//! `RangeSlider` — two thumbs on one track, for picking an interval (a price range, a
//! time window).

use frus::{Color, RangeSlider, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(RangeDemo.into_widget())
}

struct RangeDemo;

struct RangeDemoState {
    price: (f32, f32),
    hours: (f32, f32),
}

impl StatefulWidget for RangeDemo {
    type State = RangeDemoState;

    fn create_state(&self) -> RangeDemoState {
        RangeDemoState {
            price: (20.0, 80.0),
            hours: (9.0, 17.0),
        }
    }
}

impl State for RangeDemoState {
    type Widget = RangeDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        sections(vec![
            // The handler gets both ends at once. `cx.handler` takes one value, so this
            // one changes the state through a handle: a closure that returns nothing is
            // run when the change is delivered.
            Box::new(section(
                "Continuous",
                RangeSlider::new(self.price.0, self.price.1)
                    .range(0.0, 100.0)
                    .width(300.0)
                    .on_change({
                        let handle = cx.handle();
                        move |start, end| handle.set_state(|s| s.price = (start, end))
                    }),
            )),
            Box::new(Text::new(format!(
                "Price: ${:.0} – ${:.0}",
                self.price.0, self.price.1
            ))),
            Box::new(section(
                "Whole hours, labelled, custom colour",
                RangeSlider::new(self.hours.0, self.hours.1)
                    .range(0.0, 24.0)
                    .divisions(24)
                    .value_label(|v| format!("{v:02.0}:00"))
                    .active_color(Color::hex("#0891B2"))
                    .width(300.0)
                    .on_change({
                        let handle = cx.handle();
                        move |start, end| handle.set_state(|s| s.hours = (start, end))
                    }),
            )),
            Box::new(Text::new(format!(
                "Open {:02.0}:00 to {:02.0}:00",
                self.hours.0, self.hours.1
            ))),
        ])
    }
}
