//! `ToggleButtons` — a bank of buttons sharing their edges, each one on or off on its
//! own. Unlike a segmented button, any number can be on at once (bold + italic).

use frus::{
    Color, FontWeight, Icon, Icons, State, StateContext, StatefulWidget, Text, ToggleAxis,
    ToggleButtons, Widget, WidgetState, WidgetStateProperty,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(ToggleDemo.into_widget())
}

struct ToggleDemo;

struct ToggleDemoState {
    /// Bold, italic, underline: three independent answers.
    format: Vec<bool>,
    /// Text alignment: the application keeps exactly one on.
    align: usize,
}

impl StatefulWidget for ToggleDemo {
    type State = ToggleDemoState;

    fn create_state(&self) -> ToggleDemoState {
        ToggleDemoState {
            format: vec![true, false, false],
            align: 0,
        }
    }
}

impl State for ToggleDemoState {
    type Widget = ToggleDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let mut sample = Text::new("The quick brown fox");
        if self.format[0] {
            sample = sample.weight(FontWeight::Bold);
        }
        if self.format[1] {
            sample = sample.italic();
        }
        if self.format[2] {
            sample = sample.underline();
        }

        sections(vec![
            // The widget tells you which button was pressed; what that means is yours to
            // decide. Here each press flips its own flag.
            Box::new(section(
                "Many on at once",
                ToggleButtons::new(
                    self.format.clone(),
                    cx.handler(|s: &mut ToggleDemoState, i: usize| s.format[i] = !s.format[i]),
                )
                .child(Icon::new(Icons::FORMAT_BOLD))
                .child(Icon::new(Icons::FORMAT_ITALIC))
                .child(Icon::new(Icons::FORMAT_UNDERLINED)),
            )),
            Box::new(sample),
            // One-of-many: the handler turns the others off.
            Box::new(section(
                "Exactly one on, custom colours",
                ToggleButtons::new(
                    (0..3).map(|i| i == self.align).collect(),
                    cx.handler(|s: &mut ToggleDemoState, i: usize| s.align = i),
                )
                .child(Icon::new(Icons::FORMAT_ALIGN_LEFT))
                .child(Icon::new(Icons::FORMAT_ALIGN_CENTER))
                .child(Icon::new(Icons::FORMAT_ALIGN_RIGHT))
                .selected_color(Color::hex("#FFFFFF"))
                .fill_color(
                    WidgetStateProperty::new().when(WidgetState::Selected, Color::hex("#4F46E5")),
                )
                .selected_border_color(Color::hex("#4F46E5"))
                .border_radius(8.0),
            )),
            // Stacked vertically.
            Box::new(section(
                "Vertical",
                ToggleButtons::new(
                    self.format.clone(),
                    cx.handler(|s: &mut ToggleDemoState, i: usize| s.format[i] = !s.format[i]),
                )
                .direction(ToggleAxis::Vertical)
                .child(Text::new("Bold"))
                .child(Text::new("Italic"))
                .child(Text::new("Underline")),
            )),
        ])
    }
}
