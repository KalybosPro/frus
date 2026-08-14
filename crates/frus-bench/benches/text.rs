//! What it costs to measure text.
//!
//! Every widget that sizes itself around a string pays this, and milestone 289 showed
//! how much rides on it: rounding a measurement changed where every glyph on the
//! screen landed. Shaping is also the one part of a frame that is not obviously cheap,
//! so it is worth knowing what it costs before anything is built on the assumption
//! that it is.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use frus_core::FontWeight;

const WORD: &str = "Open";
const LINE: &str = "Task number 42 — due on Friday";
const PARAGRAPH: &str = "A portable Rust interface framework. The widget tree is \
    ordinary Rust values, built by `view` every frame, laid out through taffy and \
    painted into a scene the renderer batches by what covers what.";

fn text(c: &mut Criterion) {
    let mut group = c.benchmark_group("measure");
    for (name, s) in [("word", WORD), ("line", LINE), ("paragraph", PARAGRAPH)] {
        group.bench_with_input(BenchmarkId::from_parameter(name), &s, |b, s| {
            b.iter(|| frus_text::measure(s, 15.0));
        });
    }
    group.finish();

    // Wrapping is the expensive one: the shaper has to place the run before it knows
    // where the breaks fall.
    let mut group = c.benchmark_group("measure_wrapped");
    for width in [120.0f32, 240.0, 480.0] {
        group.bench_with_input(
            BenchmarkId::new("paragraph", width as u32),
            &width,
            |b, &width| {
                b.iter(|| {
                    frus_text::measure_wrapped(
                        PARAGRAPH,
                        15.0,
                        FontWeight::Regular,
                        false,
                        Some(width),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, text);
criterion_main!(benches);
