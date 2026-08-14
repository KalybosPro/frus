# Milestone 300 — Not measuring the same string twice

Milestone 299 built the first performance harness and it overturned the roadmap's
stated bottleneck. The roadmap said rebuilding the widget tree; the measurement said
**text**:

| rows | `build_ui` | the same tree with no strings | text's share |
|---|---|---|---|
| 12 | 382 µs | 93 µs | **76%** |
| 60 | 1.62 ms | 433 µs | **73%** |
| 200 | 5.38 ms | 1.47 ms | **73%** |

A twelve-row screen was spending ~290 µs of its 382 µs re-shaping strings through
cosmic-text that had not changed since the frame 16 ms earlier. That is the whole of
this milestone: stop doing that.

## The cache

Keyed on everything that can change the answer, and nothing else:

```rust
#[derive(PartialEq, Eq, Hash)]
struct MeasureKey {
    text: String,
    size: u32,          // the float's bits
    weight: u16,
    italic: bool,
    max_width: Option<u32>,
}
```

The weight and the style are the **resolved** ones — what the face database can
actually serve — so a Medium asked for on a family that only ships Regular hits the
same entry as the Regular, which is the same shaping either way. Floats are keyed by
their bits: two sizes that are bit-identical measure identically, and no other pair
needs to share an entry.

## Eviction without timestamps

Two generations, `current` and `previous`:

```rust
struct MeasureCache {
    current: HashMap<MeasureKey, Size>,
    previous: HashMap<MeasureKey, Size>,
}
```

A lookup checks `current`, then `previous` — and a hit in `previous` is **promoted**
into `current`. When `current` fills to `CACHE_CAP`, it becomes `previous` and the old
`previous` is dropped.

So a string still being drawn is touched every frame and never falls out, while a
string that has gone — last second's clock, yesterday's search box, a list scrolled
past — leaves with the generation it was in. No timestamps, no LRU bookkeeping, no
per-entry cost. The bound is roughly `2 × 2048` measurements, a few hundred kilobytes
against ~290 µs a frame.

## Invalidation is not eviction

`forget_measurements()` empties the cache outright, and it is called from `add_font`,
`set_default_family` and `set_monospace_family`. An answer from before a font was
registered is not stale, it is **wrong** — the same string with the same attributes
now shapes through a different face. This is the one case a size bound cannot handle,
and it is why the cache is cleared rather than aged.

## The ordering bug this turned up

The key records the *resolved* weight and style, and resolving reads state that
building the font system sets — `ITALIC`, from the faces actually loaded. Building the
key before the font system exists therefore resolves against nothing and produces a
different key for the same request. One line, and it is a comment as much as code:

```rust
// The key records the **resolved** weight and style, and resolving reads state
// that building the font system sets (`ITALIC`, from the faces actually loaded).
let _ = font_system();
```

## What it bought

| | J299 baseline | after |
|---|---|---|
| `measure/line` (30 chars) | 16.3 µs | **79 ns** |
| `build_ui/task_list/12` | 382 µs | 195 µs |
| *the same tree with no strings* | *93 µs* | *163 µs* |

The third row is the **control**: the same widget tree with every string replaced by a
box of the size that string would have taken. No code on that path changed, and it
moved by 1.75× — so the machine was that much slower on the second run than on the
first, and the absolute columns are not comparable.

The ratio is, and it is the claim:

> Building a twelve-row frame used to cost **4.1×** the same tree with no strings. It
> now costs **1.20×**.

Text has gone from three quarters of the frame to a sixth of it. What is left is the
tree walk and taffy — the cost the roadmap thought was the whole problem, now that it
is what remains.

That the control moved at all is worth keeping in view: these benchmarks run on a
developer machine under WSL2 with nothing pinned, so a number is only meaningful
against another number from the same run. Every claim here is a within-run ratio for
that reason.

## Verification

Three tests, and they are about the three things that can go wrong rather than about
the happy path:

- `a_cached_measurement_is_the_measurement` — a hit returns what a miss would have.
- `a_string_still_in_use_survives_the_rotation` — pushes past `CACHE_CAP * 2 + 64`
  distinct strings while touching one, and finds it still there. This is the promotion
  rule; without it the cache would evict exactly the strings a steady screen needs.
- `registering_a_font_forgets_what_was_measured` — the invalidation above.
