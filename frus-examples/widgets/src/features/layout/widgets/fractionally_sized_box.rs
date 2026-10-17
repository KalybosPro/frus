//! `FractionallySizedBox` — a box sized as a fraction of the room its parent offers.

use frus::{
    component, Alignment, Color, Column, Container, Curve, FractionallySizedBox, Slider, Text,
    Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // Fractions of a fixed frame: 75 % of the width, 50 % of the height.
        Box::new(section(
            "Width and height factors",
            Container::new()
                .width(280.0)
                .height(120.0)
                .color(Color::hex("#EEF2FF"))
                .child(
                    FractionallySizedBox::new()
                        .width_factor(0.75)
                        .height_factor(0.5)
                        .child(fill("75 % × 50 %")),
                ),
        )),
        // Live: drag the slider and the bar takes that share of the width. `animated`
        // makes it glide to each new size instead of jumping — a progress bar.
        Box::new(section("Animated, driven by state", live_bar())),
    ])
}

/// A component keeps the slider's value between frames (`use_state`); every change
/// rebuilds it with the new factor.
fn live_bar() -> impl Widget {
    component(|cx| {
        let share = cx.use_state(|| 0.4_f32);
        let current = share.get();
        Box::new(
            Column::new()
                .spacing(8.0)
                .child(
                    Container::new()
                        .width(280.0)
                        .height(32.0)
                        .radius(6.0)
                        .color(Color::hex("#EEF2FF"))
                        .child(
                            FractionallySizedBox::new()
                                .width_factor(current)
                                .animated(0.25, Curve::ease_out())
                                .child(fill(&format!("{:.0} %", current * 100.0))),
                        ),
                )
                .child(
                    Slider::new(current)
                        .width(280.0)
                        .on_change(move |value| share.set(value)),
                ),
        )
    })
}

fn fill(label: &str) -> Container {
    Container::new()
        .radius(6.0)
        .color(Color::hex("#6366F1"))
        .alignment(Alignment::CENTER)
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}
