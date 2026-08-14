//! What the batch planner costs, against what it saves.
//!
//! Milestones 294 and 295 traded a fixed pass per kind of primitive for a plan that
//! puts each one on a level above whatever it covers. The saving was measured — a
//! twelve-row list is 3 draw calls rather than one per widget — but the *cost* of
//! planning never was. Both numbers are here, so the trade can be read whole.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use frus_bench::{build, task_list};

fn batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_calls");
    for rows in [12usize, 60, 200] {
        let tree = task_list(rows);
        let ui = build(&tree);
        let scene = ui.scene();
        let primitives = scene.primitives().len();
        let calls = frus_gpu::draw_calls(scene);
        // Printed once per input: the plan's *output* is the point of the plan, and a
        // bench that only reported nanoseconds would hide it changing.
        println!("  {rows} rows: {primitives} primitives → {calls} draw calls");
        group.bench_with_input(BenchmarkId::new("task_list", rows), &rows, |b, _| {
            b.iter(|| frus_gpu::draw_calls(scene));
        });
    }
    group.finish();
}

criterion_group!(benches, batch);
criterion_main!(benches);
