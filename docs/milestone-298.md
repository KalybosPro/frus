# Milestone 298 — The advisory checks become real

Milestone 294's lesson was that an advisory check which goes red stays red: the golden
suite was broken for five milestones and the job that would have said so was set to
`continue-on-error`. Three other checks were in exactly that position — rustfmt,
clippy, and the strict rustdoc pass — each with a `TODO` in the CI file saying what to
clear before flipping it. This clears all three.

Three commits, deliberately separate, because a formatting pass mixed into anything
else is unreviewable.

## rustfmt

`cargo fmt --all`: 40 files, nothing else in the commit. The largest single change is
`crates/frus-gpu/src/text.rs`, where milestone 295's edit left a whole `match` block
under-indented — a Python script wrote it and no formatter had been over it since.

## clippy — 71 to zero

Read rather than auto-applied: `cargo clippy --fix` refuses to run in this checkout,
because `core.autocrlf` makes it see every file as dirty. Most were routine —
`map_or(true, …)` into `is_none_or`, `%` into `is_multiple_of`,
`.iter().copied().collect()` into `to_vec()`, a `clone` on a `Copy` type, a loop
indexing what it was already iterating.

Three groups needed a decision.

**`type_complexity` (12)** was mostly one shape wearing different hats:
`Box<dyn Fn() -> Box<dyn Widget<Msg>>>` — the closure a `Table` header, a `Table` row
or a `Kanban` column takes for a cell holding a widget rather than a string. That is
`CellFn<Msg>` now, public because it appears in public signatures. A type alias is
transparent, so no call site changed. The rest became local aliases where they live:
`ReorderSpec`, `RowWidgets`, `MoveFn`, `Comparator`, `BulkActions`, `Check`, `Overlay`.

**`field_reassign_with_default` (7)** were all test fixtures. Five in the demo built
the same thing — an app with its editable grid filled — so they share `app_with_grid`;
the other two take the struct-update form.

**`too_many_arguments` (6)** are each one drawing operation described exactly: a
gradient rectangle's eight properties, a render call's device/queue/target/size, one
decorated line. Splitting them would mean passing the same values in two hops, so each
carries a targeted `#[allow]` with the reason written beside it. Restructuring
`Scene::gradient_rect` is an API change with call sites, not a lint fix.

One real find on the way: milestone 293 moved `CHART_COLORS` out of the demo's
`model.rs` and left its doc comment behind, documenting nothing.

## rustdoc — twelve broken links

- Two pointed at types that never existed: `crate::ClipRect` (it is `ClipRRect`) and
  `crate::RepaintBoundary` (it is `Container::repaint_boundary`).
- One pointed at `run_desktop`, an entry point that is now the `run()` the
  `frus_shell::main!` macro generates.
- Three linked to private items (`Painters::render`, `range_mark`, `ValueAnim`). Two
  became plain code spans; `ValueAnim` became **public**, because it is the type of a
  public field and hiding the link would have hidden the leak.
- Five were in `form`'s module docs, which rustdoc merges into the `pub mod form;`
  item in `lib.rs`, where `Rule` and `Form` are not in scope. Written in full now.
- One, `[`final_x`]`, needed `Self::`.

Behind them the strict pass had a second complaint it could never reach: an `ignore`
code block in `media.rs` containing `{ … }`, which is not parseable Rust. That is why
clearing an advisory check is worth doing even when you think you know what it says —
the first error hides the rest.

## What CI enforces now

| check | before | after |
|---|---|---|
| `cargo fmt --all -- --check` | advisory | **blocking** |
| `cargo clippy --workspace --all-targets` | advisory, no `-D warnings` | **blocking, `-D warnings`** |
| `RUSTDOCFLAGS=-D warnings cargo doc` | advisory second step | **the only step, blocking** |

Two advisory jobs remain, both for reasons written where they sit: the goldens, until
the runner's mesa version is pinned, and the Android build, while its NDK setup
settles.

## Verification

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-targets` — silent.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` — clean.
- 812 workspace tests, 24 in `frus-gpu`, **127 pixel tests, all byte-identical**. The
  only change that could have moved a pixel was a Bézier constant losing digits an
  `f32` cannot hold (`0.552_284_75` → `0.552_284_8`, the same `f32`); the goldens
  confirm it did not.
