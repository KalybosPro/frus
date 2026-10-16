//! `FocusableActionDetector` and `KeyboardListener` — a custom control that answers the
//! keyboard for itself.
//!
//! `FocusableActionDetector` is a focus stop with its own key table: click it (or Tab to
//! it) and its keys work while it has the focus. `KeyboardListener` hands every keystroke
//! inside it to a closure — the escape hatch when a table cannot say what you need.

use frus::{
    Align, Alignment, Callback, Column, Container, Focus, FocusableActionDetector, Intent,
    KeyStroke, KeyboardListener, Positioned, ShortcutKey, Stack, StackFit, State, StateContext,
    StatefulWidget, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(Board.into_widget())
}

const GRID: i32 = 5;
const CELL: f32 = 48.0;
const HOME: Intent = Intent("move-home");

struct Board;

struct BoardState {
    /// The marker's cell, column then row.
    at: (i32, i32),
    last_key: String,
}

impl StatefulWidget for Board {
    type State = BoardState;

    fn create_state(&self) -> BoardState {
        BoardState {
            at: (2, 2),
            last_key: "none yet".to_string(),
        }
    }
}

impl BoardState {
    fn step(&mut self, dx: i32, dy: i32) {
        self.at.0 = (self.at.0 + dx).clamp(0, GRID - 1);
        self.at.1 = (self.at.1 + dy).clamp(0, GRID - 1);
    }
}

impl State for BoardState {
    type Widget = Board;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let scheme = &cx.theme().scheme;
        let side = CELL * GRID as f32;

        let board = Stack::new()
            .width(side)
            .height(side)
            .fit(StackFit::Expand)
            .layer(
                Container::new()
                    .radius(8.0)
                    .color(scheme.surface_container_high)
                    .alignment(Alignment::TOP_LEFT)
                    .padding(6.0)
                    .child(Text::new("Click, then use the arrows").size(12.0)),
            )
            .layer(
                Positioned::new(
                    Container::new()
                        .width(CELL - 8.0)
                        .height(CELL - 8.0)
                        .radius(CELL / 2.0)
                        .color(scheme.primary),
                )
                .left(self.at.0 as f32 * CELL + 4.0)
                .top(self.at.1 as f32 * CELL + 4.0),
            );

        let arrow = |key, dx, dy| (KeyStroke::new(key), cx.callback(move |s| s.step(dx, dy)));
        let detector = [
            arrow(ShortcutKey::Left, -1, 0),
            arrow(ShortcutKey::Right, 1, 0),
            arrow(ShortcutKey::Up, 0, -1),
            arrow(ShortcutKey::Down, 0, 1),
        ]
        .into_iter()
        // `on_key`: a keystroke straight to a handler, while this control has the focus.
        .fold(
            FocusableActionDetector::new(board),
            |detector, (stroke, run)| detector.on_key(stroke, run),
        )
        // Or through an intent, as `Shortcuts` and `Actions` do, in one widget.
        .bind(KeyStroke::new(ShortcutKey::Home), HOME)
        .action(HOME, cx.callback(|s| s.at = (0, 0)));

        // Every keystroke inside goes to the closure first. Answering `Some` takes the key;
        // `None` lets it carry on to the shortcuts around it.
        let heard = cx.handler(|s: &mut Self, stroke: KeyStroke| {
            let mut name = format!("{:?}", stroke.key);
            if stroke.shift {
                name = format!("Shift+{name}");
            }
            if stroke.ctrl {
                name = format!("Ctrl+{name}");
            }
            s.last_key = name;
        });
        let listener = KeyboardListener::new(
            // A listener hears the keys only while the focus is inside it, so it wraps a
            // focus stop.
            Focus::new(
                Container::new()
                    .width(side)
                    .height(56.0)
                    .radius(8.0)
                    .border(1.0, scheme.outline_variant)
                    .alignment(Alignment::CENTER)
                    .child(Text::new("Click here, then press any key")),
            ),
            move |stroke| -> Option<Callback> { Some(heard(stroke)) },
        );

        sections(vec![
            Box::new(section("Arrows move, Home goes to the corner", detector)),
            Box::new(section(
                "KeyboardListener",
                Column::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Start)
                    .child(listener)
                    .child(Text::new(format!("Last key: {}", self.last_key)).color(scheme.primary)),
            )),
        ])
    }
}
