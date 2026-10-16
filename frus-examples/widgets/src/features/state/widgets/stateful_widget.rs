//! `StatefulWidget` and its `State` — the lifecycle: `create_state` and `init_state` once,
//! `did_update_widget` whenever the parent hands down a new configuration, `build` on
//! every rebuild, and `dispose` when the widget leaves the screen.
//!
//! The widget is the **configuration**, made again on every rebuild; the state is what
//! the framework keeps between them.

use frus::{
    component, Align, Button, Column, Container, Row, State, StateContext, StatefulWidget, Text,
    UseState, Variant, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let log = cx.use_state(Vec::<String>::new);
        let colour = cx.use_state(|| 0_usize);
        let shown = cx.use_state(|| true);
        let scheme = &cx.theme().scheme;

        let (next, flip, clear) = (colour.clone(), shown.clone(), log.clone());
        // A `Wrap`, not a `Row`: three buttons are wider than a phone.
        let controls = Wrap::new()
            .gap(8.0)
            .run_gap(8.0)
            .child(
                // A new configuration for the same widget: `did_update_widget`.
                Button::new("Change its label")
                    .variant(Variant::Tonal)
                    .on_press(move || next.update(|i| *i = (*i + 1) % LABELS.len())),
            )
            .child(
                // Out of the tree and back: `dispose`, then a fresh `init_state`.
                Button::new(if shown.get() {
                    "Remove it"
                } else {
                    "Put it back"
                })
                .variant(Variant::Outlined)
                .on_press(move || flip.update(|on| *on = !*on)),
            )
            .child(
                Button::new("Clear log")
                    .variant(Variant::Text)
                    .on_press(move || clear.set(Vec::new())),
            );

        let child: Box<dyn Widget> = if shown.get() {
            Box::new(
                Tally {
                    label: LABELS[colour.get()],
                    log: log.clone(),
                }
                .into_widget(),
            )
        } else {
            Box::new(Text::new("(removed)").size(13.0))
        };

        let lines = log.get().into_iter().rev().take(8).fold(
            Column::new()
                .spacing(2.0)
                .cross_axis_alignment(Align::Start),
            |column, line| column.child(Text::new(line).size(13.0)),
        );

        sections(vec![
            Box::new(section("Controls", controls)),
            Box::new(section("The stateful widget", child)),
            Box::new(section(
                "Lifecycle log, newest first",
                Container::new()
                    .width(300.0)
                    .padding(12.0)
                    .radius(8.0)
                    .color(scheme.surface_container)
                    .child(lines),
            )),
        ])
    }))
}

const LABELS: [&str; 3] = ["Apples", "Pears", "Plums"];

/// The configuration: a label, and where to write the lifecycle down.
struct Tally {
    label: &'static str,
    log: UseState<Vec<String>>,
}

/// What survives rebuilds: the count. It also keeps the log, because `dispose` is given
/// nothing else.
struct TallyState {
    count: u32,
    log: UseState<Vec<String>>,
}

impl TallyState {
    fn write(&self, line: String) {
        self.log.update(|log| log.push(line));
    }
}

impl StatefulWidget for Tally {
    type State = TallyState;

    fn create_state(&self) -> TallyState {
        TallyState {
            count: 0,
            log: self.log.clone(),
        }
    }
}

impl State for TallyState {
    type Widget = Tally;

    fn init_state(&mut self, cx: &StateContext<Self>) {
        self.write(format!("init_state ({})", cx.widget().label));
    }

    fn did_update_widget(&mut self, old: &Tally, cx: &StateContext<Self>) {
        // Called on every rebuild of the parent; compare to see what actually changed.
        let new = cx.widget().label;
        if old.label != new {
            self.write(format!("did_update_widget: {} → {new}", old.label));
        }
    }

    fn dispose(&mut self) {
        self.write(format!("dispose (count was {})", self.count));
    }

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let scheme = &cx.theme().scheme;
        Box::new(
            Row::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Center)
                .child(
                    Container::new()
                        .padding(12.0)
                        .radius(8.0)
                        .color(scheme.secondary_container)
                        .child(
                            Text::new(format!("{}: {}", cx.widget().label, self.count))
                                .color(scheme.on_secondary_container),
                        ),
                )
                .child(
                    Button::new("+1")
                        .variant(Variant::Tonal)
                        .on_press(cx.callback(|s| s.count += 1)),
                ),
        )
    }
}
