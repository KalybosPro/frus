//! Programmatic scrolling — reading where a scroll area is (`on_scroll` and
//! `ScrollPosition`) and moving it (`keyed` + `frus::host::scroll_to`): a "back to top"
//! button, jumping to an item, a chat that opens at its newest message.

use frus::{
    keyed, Align, Button, Color, Column, Container, ListTile, ListView, Row, ScrollPosition,
    ScrollTo, Text, Variant, Widget, Wrap,
};

use crate::common::shared::{section, sections};

/// The key that names the list, for requests to find it.
const LIST: &str = "scroll-to-demo-list";
const ROW: f32 = 48.0;
const COUNT: usize = 200;

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let position = cx.use_state(|| None::<ScrollPosition>);

        // Reading: the list reports its position as it moves. `notify_every` limits the
        // messages to one per 24 px (plus where it comes to rest).
        let report = position.clone();
        let list = ListView::new(COUNT, ROW, |i| {
            ListTile::new().title(format!("Row {}", i + 1))
        })
        .height(240.0)
        .width(320.0)
        .on_scroll(move |p: ScrollPosition| report.set(Some(p)))
        .notify_every(24.0);

        // Moving: wrap the area in `keyed(key, …)` and send a request to that key. The
        // request is resolved on the next frame; `instant()` skips the animation.
        let controls = Wrap::new()
            .gap(8.0)
            .run_gap(8.0)
            .child(button("Top", || {
                frus::host::scroll_to(LIST, ScrollTo::start())
            }))
            .child(button("Row 100", || {
                frus::host::scroll_to(LIST, ScrollTo::y(99.0 * ROW))
            }))
            .child(button("Bottom", || {
                frus::host::scroll_to(LIST, ScrollTo::end())
            }))
            .child(button("Row 150, instantly", || {
                frus::host::scroll_to(LIST, ScrollTo::y(149.0 * ROW).instant())
            }));

        let status = match position.get() {
            Some(p) => format!(
                "{:.0}% down · {:.0} px left below",
                p.fraction_y() * 100.0,
                p.remaining_y()
            ),
            None => "Scroll the list".to_string(),
        };

        sections(vec![
            Box::new(section(
                "Move it",
                Column::new()
                    .spacing(12.0)
                    .cross_axis_alignment(Align::Start)
                    .child(controls)
                    .child(
                        Container::new()
                            .border(1.0, Color::hex("#A1A1AA"))
                            .child(keyed(LIST, list)),
                    ),
            )),
            Box::new(section(
                "Read it",
                Row::new()
                    .main_axis_size(frus::MainAxisSize::Min)
                    .child(Text::new(status).color(cx.theme().muted)),
            )),
        ])
    }))
}

fn button(label: &str, on_press: impl Fn() + 'static) -> Button {
    Button::new(label)
        .variant(Variant::Tonal)
        .on_press(on_press)
}
