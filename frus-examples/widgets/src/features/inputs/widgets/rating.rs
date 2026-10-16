//! `Rating` — a score given by tapping stars.

use frus::{Rating, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(RatingDemo.into_widget())
}

struct RatingDemo;

struct RatingDemoState {
    stars: u32,
    tens: u32,
}

impl StatefulWidget for RatingDemo {
    type State = RatingDemoState;

    fn create_state(&self) -> RatingDemoState {
        RatingDemoState { stars: 3, tens: 7 }
    }
}

impl State for RatingDemoState {
    type Widget = RatingDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        sections(vec![
            // The current value, the number of stars, and what to do with a tap.
            Box::new(section(
                "Out of five",
                Rating::new(
                    self.stars,
                    5,
                    cx.handler(|s: &mut RatingDemoState, v: u32| s.stars = v),
                ),
            )),
            Box::new(Text::new(format!("You gave {} of 5", self.stars))),
            Box::new(section(
                "Out of ten",
                Rating::new(
                    self.tens,
                    10,
                    cx.handler(|s: &mut RatingDemoState, v: u32| s.tens = v),
                ),
            )),
            // A read-only score: shown, not changed.
            Box::new(section(
                "Read-only",
                Rating::new(4, 5, cx.handler(|_: &mut RatingDemoState, _: u32| {})).enabled(false),
            )),
        ])
    }
}
