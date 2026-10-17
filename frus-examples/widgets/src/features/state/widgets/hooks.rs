//! Hooks — `use_state`, `use_ref` and `use_memo`: state kept by a plain function
//! component (`frus::component(|cx| ..)`), with no struct to declare.
//!
//! Hooks are matched to their values by the **order** they are called in, so call them
//! unconditionally and in the same order on every build: never inside an `if`, a loop or
//! a closure.

use frus::{component, Align, Button, Column, Component, Row, Slider, Text, Variant, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        Box::new(section(
            "use_state: a value that rebuilds when set",
            counter(),
        )),
        Box::new(section("use_ref: a value that does not", builds())),
        Box::new(section(
            "use_memo: work redone only when its input changes",
            primes(),
        )),
    ])
}

pub fn counter() -> Component {
    component(|cx| {
        // Made by the closure the first time, then kept for as long as the component is
        // on screen. `set` and `update` ask for a rebuild.
        let count = cx.use_state(|| 0);
        let (up, reset) = (count.clone(), count.clone());
        Box::new(
            Row::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Center)
                .child(Button::new("Add one").on_press(move || up.update(|n| *n += 1)))
                .child(
                    Button::new("Reset")
                        .variant(Variant::Text)
                        .on_press(move || reset.set(0)),
                )
                .child(Text::new(format!("count = {}", count.get()))),
        )
    })
}

pub fn builds() -> Component {
    component(|cx| {
        // A `use_ref` changes without rebuilding: right for bookkeeping the screen does not
        // show directly — a timer handle, a previous value, a counter of builds.
        let built = cx.use_ref(|| 0_u32);
        built.update(|n| *n += 1);
        let pending = cx.use_ref(|| 0_u32);
        let rebuilds = cx.use_state(|| ());

        let (add, show) = (pending.clone(), rebuilds.clone());
        Box::new(
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Row::new()
                        .spacing(8.0)
                        .child(
                            Button::new("Add to the ref")
                                .variant(Variant::Tonal)
                                .on_press(move || add.update(|n| *n += 1)),
                        )
                        .child(
                            // Setting a state — even to the same value — rebuilds, and
                            // the ref's current value shows.
                            Button::new("Rebuild")
                                .variant(Variant::Outlined)
                                .on_press(move || show.set(())),
                        ),
                )
                .child(Text::new(format!(
                    "ref = {} (shown as of the last build) · {} builds of this component",
                    pending.get(),
                    built.get()
                ))),
        )
    })
}

pub fn primes() -> Component {
    component(|cx| {
        let limit = cx.use_state(|| 10_000.0_f32);
        let unrelated = cx.use_state(|| 0_u32);
        let n = limit.get() as usize;

        // Recomputed only when `n` changes: pressing "Unrelated" rebuilds, but the count
        // comes from the memo.
        let runs = cx.use_ref(|| 0_u32);
        let counted = {
            let runs = runs.clone();
            cx.use_memo(n, move || {
                runs.update(|r| *r += 1);
                count_primes(n)
            })
        };

        let (set, bump) = (limit.clone(), unrelated.clone());
        Box::new(
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Slider::new(limit.get())
                        .range(1_000.0, 50_000.0)
                        .width(260.0)
                        .on_change(move |value| set.set(value.round())),
                )
                .child(Text::new(format!("{counted} primes below {n}")))
                .child(
                    Row::new()
                        .spacing(12.0)
                        .cross_axis_alignment(Align::Center)
                        .child(
                            Button::new("Unrelated rebuild")
                                .variant(Variant::Outlined)
                                .on_press(move || bump.update(|n| *n += 1)),
                        )
                        .child(
                            Text::new(format!(
                                "{} rebuilds, {} computations",
                                unrelated.get(),
                                runs.get()
                            ))
                            .size(13.0),
                        ),
                ),
        )
    })
}

/// How many primes are below `n` — a sieve, cheap but not free.
pub fn count_primes(n: usize) -> usize {
    if n < 3 {
        return 0;
    }
    let mut composite = vec![false; n];
    let mut count = 0;
    for i in 2..n {
        if !composite[i] {
            count += 1;
            let mut multiple = i * i;
            while multiple < n {
                composite[multiple] = true;
                multiple += i;
            }
        }
    }
    count
}
