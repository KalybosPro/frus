//! `DropdownButton` and `DropdownOption` — a button showing the current choice, whose
//! options float below it. For picking one value from a short, closed list.

use frus::{
    Color, Container, DropdownButton, DropdownOption, Row, State, StateContext, StatefulWidget,
    Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(DropdownDemo.into_widget())
}

const FRUITS: [&str; 4] = ["Apple", "Banana", "Cherry", "Grape"];
const COLORS: [(&str, &str); 4] = [
    ("Red", "#EF4444"),
    ("Green", "#22C55E"),
    ("Blue", "#3B82F6"),
    ("Purple", "#A855F7"),
];

struct DropdownDemo;

struct DropdownDemoState {
    fruit: usize,
    fruit_open: bool,
    color: usize,
    color_open: bool,
}

impl StatefulWidget for DropdownDemo {
    type State = DropdownDemoState;

    fn create_state(&self) -> DropdownDemoState {
        DropdownDemoState {
            fruit: 0,
            fruit_open: false,
            color: 2,
            color_open: false,
        }
    }
}

impl State for DropdownDemoState {
    type Widget = DropdownDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // Controlled twice over: the application holds the choice **and** whether the
        // list is open. The header press toggles it; a pick stores the index and closes.
        let fruits = DropdownButton::new(
            FRUITS[self.fruit],
            cx.callback(|s: &mut DropdownDemoState| s.fruit_open = !s.fruit_open),
        )
        .selected(self.fruit)
        .width(200.0)
        .options(
            self.fruit_open,
            &FRUITS,
            cx.handler(|s: &mut DropdownDemoState, i: usize| {
                s.fruit = i;
                s.fruit_open = false;
            }),
        );

        // Options that are not just words: a swatch beside the name. The last one is
        // shown but cannot be picked.
        let options = COLORS
            .iter()
            .enumerate()
            .map(|(i, (name, hex))| {
                let option = DropdownOption::widget(
                    Row::new()
                        .spacing(8.0)
                        .child(
                            Container::new()
                                .width(14.0)
                                .height(14.0)
                                .radius(7.0)
                                .color(Color::hex(hex)),
                        )
                        .child(Text::new(*name)),
                );
                option.enabled(i != COLORS.len() - 1)
            })
            .collect();
        let colors = DropdownButton::new(
            COLORS[self.color].0,
            cx.callback(|s: &mut DropdownDemoState| s.color_open = !s.color_open),
        )
        .selected(self.color)
        .width(200.0)
        .menu_radius(12.0)
        .options_widgets(
            self.color_open,
            options,
            cx.handler(|s: &mut DropdownDemoState, i: usize| {
                s.color = i;
                s.color_open = false;
            }),
        );

        sections(vec![
            Box::new(section("Words", fruits)),
            Box::new(section("Custom rows, one unavailable", colors)),
            Box::new(Text::new(format!(
                "{} · {}",
                FRUITS[self.fruit], COLORS[self.color].0
            ))),
        ])
    }
}
