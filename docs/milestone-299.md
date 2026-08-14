# Milestone 299 — What a frame actually costs

The roadmap has carried this for a long time: *"Benchmarks. There is no performance
harness at all. Frame time, layout cost, scene build, and text shaping all need one
before any optimization claim can be honest."* Meanwhile milestones 294 and 295 made
claims about draw calls, and the memoization item has been deferred three times with
"it has not been a bottleneck yet".

None of that was measured. It is now, and the answer is not the one the roadmap
assumed.

## The harness

`crates/frus-bench` — a crate that exists to measure, `publish = false`, so criterion
stays out of the shipped crates' dependency trees. Its `src/lib.rs` holds the screens,
in one place, so that a number from one bench means the same as a number from the next
and no bench invents its own idea of "a realistic screen".

Three benches: `scene` (`build_ui`), `text` (measuring and wrapping), `batch`
(`draw_calls`). `[profile.bench]` inherits the release profile — fat LTO, one codegen
unit — so the numbers are the shipped build's, not a debug build's.

```sh
cargo bench -p frus-bench
cargo bench -p frus-bench --bench scene
```

## The baseline

Measured on the maintainer's machine under WSL2. The absolute numbers are that
machine's; the *ratios* are the framework's.

**Building a frame** — layout through taffy, then the walk that makes the scene:

| rows | `build_ui` | the same tree with no strings | text's share |
|---|---|---|---|
| 12 | 382 µs | 93 µs | **76%** |
| 60 | 1.62 ms | 433 µs | **73%** |
| 200 | 5.38 ms | 1.47 ms | **73%** |

The second column is the same widget tree with every string replaced by a box of the
size that string would have taken: same widget count, same layout work, no shaping.

**Measuring text**, which is where that share goes:

| | |
|---|---|
| a word (`Open`) | 2.2 µs |
| a line (30 chars) | 16.3 µs |
| a paragraph (170 chars) | 92 µs |
| the same paragraph, wrapped to 240 px | 104 µs |

**Planning the batches:**

| primitives | `draw_calls` | result |
|---|---|---|
| 80 | 4.7 µs | 3 draw calls |
| 392 | 66 µs | 5 draw calls |
| 1302 | 597 µs | 5 draw calls |

## What the numbers say

**Three-quarters of building a frame is measuring text, and none of it is cached.**
`frus_text::measure` re-shapes the string through cosmic-text on every call, every
frame, for strings that have not changed. A twelve-row screen spends ~290 µs of its
382 µs re-answering questions it answered 16 ms ago.

This is a correction to the roadmap, which names the bottleneck as `view` rebuilding
the whole tree and proposes rebuild memoization for it. Rebuilding is real but it is
the small half: the wordless tree does all the same rebuilding and costs a quarter as
much. **A measurement cache keyed on `(text, size, weight, italic, max_width)` is the
cheaper fix and does not need a new architecture.** Rebuild memoization is still worth
having; it is no longer the first thing to reach for.

**The cost is linear in the widget count.** 12 → 60 → 200 rows costs 382 µs → 1.62 ms
→ 5.38 ms: 16.7× the rows for 14.1× the time. Nothing quadratic is hiding in the walk.

**The batch planner is not linear.** 80 → 1302 primitives costs 4.7 µs → 597 µs, which
is 16× the primitives for 127× the time — roughly O(n²), because each primitive scans
the levels for something it overlaps. At 1302 primitives that is 0.6 ms, about 4% of a
60 Hz frame, so it is not urgent; but it grows the wrong way and the milestone-294 note
should not have implied the plan was free. Recorded on the roadmap.

**A screenful is comfortable; a long list is not.** At 60 Hz the budget is 16.7 ms. A
twelve-row screen builds in 2.3% of it. Two hundred rows takes 32%, of which 23 points
are text measurement. That is the argument for the cache in one line.

## Verification

- `cargo bench -p frus-bench` — 15 benchmarks, all reporting.
- The `batch` bench prints its plan's *output* beside the timing (`80 primitives → 3
  draw calls`), so a change that made planning faster by making it wrong would show.
- `cargo bench -p frus-bench -- --test` runs every bench once and is wired into the CI
  test job, because a benchmark suite nobody compiles rots exactly like an advisory
  check nobody reads (milestones 294, 298).
