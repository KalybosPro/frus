# Milestone 293 — The demo was one file, and it taught that

A question, asked plainly: *the demo puts all its code in one file — does that mean an
application cannot be split across several and import between them?*

No. Rust has modules, `use`, and `pub(crate)`; the framework itself is ninety-nine
files. But the question was fair, because the only large frus application anyone can
read was 4,360 lines in `crates/frus-demo/src/lib.rs`, and an example teaches whatever
it does whether or not it meant to.

So it is split. This milestone changes no behaviour: same widgets, same tests, same
scene.

## The shape

```
src/
  lib.rs          the Application impl, the entry point, the module list      332
  prelude.rs      one import for the whole app                                 44
  message.rs      Msg — everything that can happen                            169
  model.rs        the state, the questions worth asking it, and its data      376
  update.rs       reduce(): the one place state changes                       624
  storage.rs      loading and saving                                           55
  theme.rs        the palette                                                  68
  l10n.rs         the three languages                                          40
  assets.rs       the embedded images                                          33
  parts.rs        what more than one screen draws                             147
  screens/
    mod.rs        which screen the current route means                         74
    todo.rs 484   settings.rs 220   wizard.rs 251   charts.rs 141
    data.rs 118   grid.rs      93   tour.rs    86   board.rs   80
    task.rs  76   journal.rs   61
  tests.rs        the tests that cut across modules                           939
```

The split follows the Elm triangle the framework is built on, not a taxonomy borrowed
from somewhere else. `model.rs` holds the state *and the derived questions*
(`active_count`, `current_route`) — kept next to the state, they cannot drift into
three slightly different versions of the same count. `message.rs` holds `Msg` alone,
because the list is the application's vocabulary and seeing it whole is what makes it
obvious when a variant is really two. `update.rs` holds `reduce`, one function, one
place state changes, however many screens send into it.

Views are split **by screen**, because that is how you think about them and how they
change. `parts.rs` holds what more than one screen draws — and nothing arrived there
on suspicion that it might be shared later; it moved in when the second screen asked.

## The prelude

Thirty widget names at the top of every screen tell the reader nothing. They are named
once in `prelude.rs`, and each module writes `use crate::prelude::*;` — the same idea
as a UI toolkit shipping one import that brings in its whole widget set.

It is for what is genuinely common. A screen needing one unusual widget still imports
that one by name; the prelude removes noise, it does not hide where things come from.
`screens/data.rs` says `use crate::screens::todo::*;` for the confirmation card it
borrows, and that line is worth its space.

Two things the compiler had to say about it, both worth recording because anyone
splitting an application will meet them:

- **Macros do not travel through a glob.** `column!` and `row!` are exported at
  frus-widgets' root, and re-exporting them from a prelude that modules glob-import
  makes every use ambiguous (E0659) — the name reaches the module by two routes. They
  are imported by name in the modules that use them.
- **`pub(crate)` is the visibility you want**, not `pub`. Every item that left `lib.rs`
  needed it: visible to the rest of the application, invisible to anything depending
  on it. Struct fields need it one by one; enum variants inherit it from the enum.

## What the split showed

Worth recording, because it is the argument for doing this at all rather than a side
effect of having done it. The first version compiled with `model.rs` and `update.rs`
both saying `use crate::screens::*;` — the state and the reducer importing the views.
In one file that dependency was invisible; across files it is a line you have to
write, and writing it looks wrong because it is.

What they were reaching for was **data**: the data table's twelve people, the Kanban's
starting cards, the dashboard's series. And the grid's `grid_cell_error` /
`grid_faults` / `grid_next_error` — pure functions over `&[Vec<String>]`, which is
validation of the model, not drawing of a cell. All of it moved to `model.rs`, and
now:

```
model.rs   → prelude
update.rs  → prelude, screens::wizard_form
lib.rs     → prelude, screens::build_view
```

One named import each, both pointing at a single function. `wizard_form` stays in the
wizard screen because it builds a `frus_widgets::form::Form` — a widget-layer type —
so it genuinely straddles; naming it rather than glob-importing the whole view layer
is the honest way to say so.

`MENU`, three dropdown labels, went the other way: out of the model and into the
settings screen that is the only thing that draws them.

## Tests

The 37 tests moved to `src/tests.rs`, and they belong there: nearly all of them drive
`reduce` and then `build_view` and read the scene, which is three modules at once.
`save_then_load_roundtrips` was the exception — it exercises `storage.rs` and nothing
else — so it moved into that module's own `#[cfg(test)] mod tests`, as the worked
example of where a module-local test goes.

`grid_first_error` went, too. It was the workspace's one `dead_code` warning and a
roadmap item: unused by the application, called only by a test, and a one-line
delegation to `grid_next_error(grid, None)`. The test calls that instead.

## What this is not

It is not an architecture prescription. There is no `domain/`, no `infrastructure/`,
no repository interface with one implementation — a to-do list does not have those
layers, and inventing them in the flagship example would teach something worse than a
single file does. What the layout claims is smaller and true: state, messages and
`update` are each findable in one place, screens are one file each, and the compiler
tells you when a dependency between them appears.

The guide is [`docs/app-structure.md`](app-structure.md), linked from both READMEs.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **812 tests, 0
  failures**: the same count as before the split, which is the point. `frus-demo`'s
  37 all pass, one of them now from `storage.rs`.
- `cargo clippy -p frus-demo` — 43 warnings, every one a category already on the
  roadmap's backlog (`too_many_arguments`, `type_complexity`, …). The split introduced
  no new class of warning, and no `dead_code` remains.
