//! What it costs to turn a widget tree into a frame's worth of work: layout through
//! taffy, then the walk that produces the scene and the hit-test registries.
//!
//! `view` rebuilds the whole tree every frame, and the roadmap's memoization item has
//! been deferred with "it has not been a bottleneck yet". This is the bench that lets
//! that sentence be checked rather than repeated.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use frus_bench::{build, nested, task_list, task_list_wordless};

fn scene(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_ui");
    // 12 rows is a screenful; 200 is a list nobody would draw at once but a virtualised
    // one might build. The gap between them is what says whether the cost is linear.
    for rows in [12usize, 60, 200] {
        let tree = task_list(rows);
        group.bench_with_input(BenchmarkId::new("task_list", rows), &rows, |b, _| {
            b.iter(|| build(&tree));
        });
    }
    group.finish();

    // The same trees with every string replaced by a box of the size it would have
    // taken: same widget count, same layout, no shaping. The gap is what measuring
    // text costs, and there is no measurement cache behind it.
    let mut group = c.benchmark_group("build_ui/wordless");
    for rows in [12usize, 60, 200] {
        let tree = task_list_wordless(rows);
        group.bench_with_input(BenchmarkId::new("task_list", rows), &rows, |b, _| {
            b.iter(|| build(&tree));
        });
    }
    group.finish();

    let mut group = c.benchmark_group("build_ui/nested");
    for depth in [8usize, 64, 256] {
        let tree = nested(depth);
        group.bench_with_input(BenchmarkId::new("depth", depth), &depth, |b, _| {
            b.iter(|| build(&tree));
        });
    }
    group.finish();
}

criterion_group!(benches, scene);
criterion_main!(benches);
