//! `Timeline` — a vertical chronology: events joined by a line, each marked with a dot.

use frus::{FontWeight, TextStyle, Timeline, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // Each event is a title and a line of detail, oldest first.
        Box::new(section(
            "Order history",
            Timeline::new()
                .event("Ordered", "Monday 09:12 — paid by card")
                .event("Packed", "Monday 15:40 — warehouse B")
                .event("Shipped", "Tuesday 08:05 — tracking FR123456")
                .event("Delivered", "Wednesday 11:30 — left with a neighbour"),
        )),
        // The two lines' type is the theme's unless you say otherwise.
        Box::new(section(
            "Custom text styles",
            Timeline::new()
                .title_text_style(TextStyle::new(16.0).weight(FontWeight::Bold))
                .detail_text_style(TextStyle::new(12.0))
                .event("v0.1", "First public release")
                .event("v0.2", "Router and deep links")
                .event("v0.3", "Components and hooks"),
        )),
    ])
}
