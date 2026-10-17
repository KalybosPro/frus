//! `use_interval` — a timer owned by a component: it ticks for as long as builds keep
//! asking for it, and stops by itself when a build does not (or the component goes).
//!
//! Unlike the other hooks it **may** be called conditionally: `if running { .. }` is
//! exactly how a timer is switched on and off.

use std::time::Duration;

use frus::{component, Align, Button, Column, FontWeight, Row, Text, Variant, Widget};

use crate::common::shared::{section, sections};

/// The tick, in tenths of a second.
const TICK: Duration = Duration::from_millis(100);

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let tenths = cx.use_state(|| 0_u64);
        let running = cx.use_state(|| false);

        // Only while running: the next build without this call stops the timer.
        if running.get() {
            let tick = tenths.clone();
            cx.use_interval(TICK, move || tick.update(|t| *t += 1));
        }

        let t = tenths.get();
        let (toggle, reset) = (running.clone(), tenths.clone());
        let stop = running.clone();
        let display = Text::new(format!("{}:{:02}.{}", t / 600, (t / 10) % 60, t % 10))
            .size(40.0)
            .weight(FontWeight::Bold)
            .color(cx.theme().primary);

        sections(vec![Box::new(section(
            "A stopwatch",
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(display)
                .child(
                    Row::new()
                        .spacing(8.0)
                        .child(
                            Button::new(if running.get() { "Pause" } else { "Start" })
                                .on_press(move || toggle.update(|on| *on = !*on)),
                        )
                        .child(Button::new("Reset").variant(Variant::Outlined).on_press(
                            move || {
                                stop.set(false);
                                reset.set(0);
                            },
                        )),
                ),
        ))])
    }))
}
