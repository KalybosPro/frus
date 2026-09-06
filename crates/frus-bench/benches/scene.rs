//! What it costs to turn a widget tree into a frame's worth of work: layout through
//! taffy, then the walk that produces the scene and the hit-test registries.
//!
//! `view` rebuilds the whole tree every frame, and the roadmap's memoization item has
//! been deferred with "it has not been a bottleneck yet". This is the bench that lets
//! that sentence be checked rather than repeated.

use criterion::{criterion_group, BenchmarkId, Criterion};
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

/// **On a thread of its own, with a stack that fits the deepest tree here.**
///
/// The nested case goes 256 levels down on purpose, and building a tree costs stack per
/// level: about seven kilobytes across the layout walk and the scene walk once milestone
/// 474 took two eleven-kilobyte `Theme` copies out of each of them. 256 levels is around
/// 1.8 MB, and a Windows main thread gets one megabyte by default — so this bench, and
/// only this bench, overflowed there while passing everywhere else. A thread says what it
/// needs; the numbers are the same on any stack.
fn main() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let mut criterion = Criterion::default().configure_from_args();
            scene(&mut criterion);
            criterion.final_summary();
        })
        .expect("spawning the bench thread")
        .join()
        .expect("the bench thread");
}
