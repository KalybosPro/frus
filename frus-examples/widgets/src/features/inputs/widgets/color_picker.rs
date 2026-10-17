//! `ColorPicker` — a grid of colour swatches; the chosen one wears a ring.

use frus::{Color, ColorPicker, Container, Row, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(ColorPickerDemo.into_widget())
}

const PALETTE: [&str; 12] = [
    "#EF4444", "#F97316", "#F59E0B", "#84CC16", "#22C55E", "#14B8A6", "#06B6D4", "#3B82F6",
    "#6366F1", "#A855F7", "#EC4899", "#64748B",
];

struct ColorPickerDemo;

struct ColorPickerDemoState {
    color: Option<Color>,
}

impl StatefulWidget for ColorPickerDemo {
    type State = ColorPickerDemoState;

    fn create_state(&self) -> ColorPickerDemoState {
        ColorPickerDemoState {
            color: Some(Color::hex(PALETTE[7])),
        }
    }
}

impl State for ColorPickerDemoState {
    type Widget = ColorPickerDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The selection, the number of columns, and what to do with a pick; then one
        // `swatch` per colour, in order.
        let picker = PALETTE.iter().fold(
            ColorPicker::new(
                self.color,
                6,
                cx.handler(|s: &mut ColorPickerDemoState, c: Color| s.color = Some(c)),
            ),
            |picker, hex| picker.swatch(Color::hex(hex)),
        );

        let preview = match self.color {
            Some(color) => Row::new()
                .spacing(12.0)
                .child(
                    Container::new()
                        .width(48.0)
                        .height(24.0)
                        .radius(6.0)
                        .color(color),
                )
                .child(Text::new("Your label colour")),
            None => Row::new().child(Text::new("No colour")),
        };

        sections(vec![
            Box::new(section("Six columns", picker)),
            Box::new(preview),
        ])
    }
}
