# Structuring an application

frus applications are ordinary Rust crates. Nothing about the framework asks you to
keep an application in one file, and past a few hundred lines nothing about Rust
does either — the framework itself is ninety-nine modules.

This is the shape [`crates/frus-demo`](../crates/frus-demo/src/) uses, and it is a
reasonable default for anything larger than a counter.

```
src/
  lib.rs          the Application impl, the entry points, and the module list
  prelude.rs      one import for the whole app
  model.rs        the state, and the questions you ask it
  message.rs      Msg — everything that can happen
  update.rs       reduce(): the one place state changes
  storage.rs      loading and saving
  theme.rs        the palette
  screens/
    mod.rs        which screen the current route means
    todo.rs       one file per screen
    settings.rs
    …
  parts.rs        the pieces screens share — rows, cards, tiles
```

## What goes where

The split follows the Elm triangle the framework is built on, not an arbitrary
taxonomy. Each of the three sides is a file you can read on its own:

- **`model.rs`** holds the state and the *derived questions* — `active_count(app)`,
  `current_route(app)`. Keeping those next to the state rather than in the views is
  what stops the same count being computed three different ways.
- **`message.rs`** holds `Msg` alone. It is the app's vocabulary; having it in one
  place makes it obvious when a variant is really two.
- **`update.rs`** holds `reduce`. One function, one place state changes, no matter how
  many screens send into it.

Views are split by **screen**, because that is how you think about them and how they
change. Anything two screens both draw moves to `parts.rs` when the second one wants
it — not before.

## `use` across files

Rust modules see each other through `use`, and items are private to their module
unless they say otherwise. Within one crate, `pub(crate)` is the visibility you want:
visible to the rest of your application, invisible to anyone depending on it.

```rust
// model.rs
pub(crate) struct Todo { pub(crate) done: bool, pub(crate) label: String }
pub(crate) fn active_count(app: &TodoApp) -> usize { … }
```

```rust
// screens/todo.rs
use crate::model::{active_count, TodoApp};
```

`lib.rs` names the modules, and that list is the map of the application:

```rust
mod message;
mod model;
mod parts;
mod prelude;
mod screens;
mod storage;
mod theme;
mod update;
```

## The prelude

Most files in a UI application want most of the same names — the widgets, the theme,
`Msg`, the state type. Naming them file by file is thirty lines of imports per screen
that tell the reader nothing.

A crate **prelude** is the usual answer, and it is the same idea as a UI toolkit
shipping one import that brings in its whole widget set:

```rust
// prelude.rs — what every screen needs.
pub(crate) use crate::message::Msg;
pub(crate) use crate::model::*;
pub(crate) use crate::theme::*;
pub(crate) use frus_widgets::{column, row, /* … */};
```

```rust
// screens/settings.rs
use crate::prelude::*;
```

Use it for what is genuinely common. A screen that needs one unusual widget should
still import that one by name — the prelude is there to remove noise, not to hide
where things come from.

Two things the compiler will tell you, both worth knowing in advance:

- **Macros do not travel through a glob.** `column!` and `row!` are exported at
  frus-widgets' root, so re-exporting them from a prelude that modules glob-import
  makes every use ambiguous (E0659) — the name arrives by two routes. Import them by
  name in the modules that use them.
- **Struct fields need `pub(crate)` one by one.** Enum variants inherit their
  visibility from the enum; fields do not inherit it from the struct.

## Which way the dependencies point

Splitting an application makes its dependency graph something you have to write down,
and that is most of the value. If `model.rs` or `update.rs` finds itself importing a
screen, look at what it is reaching for: usually it is *data* that happened to be
declared next to the widget that drew it, or a pure function over the state that
happened to be declared next to the screen that displays its result. Both belong with
the model. The state and the reducer should not need the views.

## Tests

`update` is pure, so most of an application's tests need no GPU and no window, and
they belong next to what they test:

```rust
// update.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_a_task_puts_it_at_the_end() { … }
}
```

For tests that cut across several modules — a whole screen's behaviour — a `tests.rs`
at the crate root, declared `#[cfg(test)] mod tests;`, keeps them together without
having to make items public just to reach them.
