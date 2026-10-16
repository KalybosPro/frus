use frus::{Align, Column, FontWeight, StatelessWidget, Text};

use crate::common::colors::{black, grey};

pub struct Greetings;

impl StatelessWidget for Greetings {
    fn build(&self, _cx: &frus::BuildContext) -> Box<dyn frus::Widget> {
        Box::new(
            Column::new()
                .spacing(4.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Text::new("Tuesday, September 24")
                        .size(12.0)
                        .color(grey())
                        .weight(FontWeight::Regular),
                )
                .child(
                    Text::new("Good morning, Kalybos")
                        .size(30.0)
                        .color(black())
                        .weight(FontWeight::Bold),
                )
                .child(
                    Text::new("Here’s what needs your attention.")
                        .size(12.0)
                        .color(grey())
                        .weight(FontWeight::Regular),
                ),
        )
    }
}
