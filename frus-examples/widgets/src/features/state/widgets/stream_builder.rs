//! `use_stream` and `StreamBuilder` — work that gives **several** values as it goes: a
//! progress, a feed, messages. The work is handed a `StreamSink` and `add`s each value;
//! the component shows the latest one, then knows when the work is done.

use frus::{
    component, Align, AsyncSnapshot, Button, Column, ConnectionState, LinearProgressIndicator,
    StreamBuilder, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

/// The work is split in this many steps, each reported.
pub const STEPS: u32 = 40;
/// How far each step searches.
pub const PER_STEP: u64 = 4_000;

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        // The run number is the dependency: bumping it starts the work again.
        let run = cx.use_state(|| 0_u32);
        let snapshot = cx.use_stream(run.get(), |sink| async move {
            let mut found = 0;
            for step in 0..STEPS {
                let start = u64::from(step) * PER_STEP;
                found += (start..start + PER_STEP).filter(|&n| is_prime(n)).count();
                sink.add(Progress {
                    done: step + 1,
                    found,
                });
            }
        });

        let again = run.clone();
        sections(vec![
            Box::new(section(
                "cx.use_stream: search primes, reporting each step",
                Column::new()
                    .spacing(12.0)
                    .cross_axis_alignment(Align::Start)
                    .child(show(&snapshot))
                    .child(
                        Button::new("Run again")
                            .variant(Variant::Tonal)
                            .on_press(move || again.update(|n| *n += 1)),
                    ),
            )),
            // The same as a widget. Here the stream is a countdown.
            Box::new(section(
                "StreamBuilder",
                StreamBuilder::new(
                    (),
                    |_: &(), sink| async move {
                        for left in (0..=5).rev() {
                            sink.add(left);
                        }
                    },
                    |_, snapshot: AsyncSnapshot<i32>| {
                        Box::new(Text::new(match (snapshot.state, snapshot.data) {
                            (ConnectionState::Done, _) => "Lift-off".to_string(),
                            (_, Some(left)) => format!("{left}…"),
                            (_, None) => "Waiting".to_string(),
                        }))
                    },
                ),
            )),
        ])
    }))
}

#[derive(Clone, Copy)]
pub struct Progress {
    done: u32,
    found: usize,
}

pub fn show(snapshot: &AsyncSnapshot<Progress>) -> Box<dyn Widget> {
    let progress = snapshot.data.map_or(0.0, |p| p.done as f32 / STEPS as f32);
    let label = match (snapshot.state, snapshot.data) {
        (ConnectionState::Waiting, _) | (_, None) => "Starting…".to_string(),
        (ConnectionState::Active, Some(p)) => {
            format!("Step {}/{STEPS}: {} primes so far", p.done, p.found)
        }
        (ConnectionState::Done, Some(p)) => {
            format!(
                "Done: {} primes below {}",
                p.found,
                u64::from(STEPS) * PER_STEP
            )
        }
    };
    Box::new(
        Column::new()
            .spacing(8.0)
            .cross_axis_alignment(Align::Start)
            .child(LinearProgressIndicator::new(progress).width(260.0))
            .child(Text::new(label)),
    )
}

/// Trial division: slow on purpose, so the steps can be seen.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n.is_multiple_of(d) {
            return false;
        }
        d += 1;
    }
    true
}
