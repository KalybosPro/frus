# Milestone 417 — A surface could only be narrowed where a widget was built

The theme has been scopeable since milestone 309: `Themed::tweak` hands a subtree its own,
and the walk swaps it on the way down. The **surface** — the size, the intrusions the
platform reported, the reader's font setting — could not be. It could only be narrowed where
a widget is *constructed*, by `SafeArea::build`, which takes a closure and runs it under a
narrowed description.

That is the wrong end for a shell.

## The recorded cost

A `Scaffold` is handed slots that are **already built**. It could not tell its app bar's
subtree "the status bar has been dealt with" — it could only inset the bar from outside. So
it had one switch where the reference has two:

> there the shell makes the slot tall enough and the bar decides for itself whether to
> consume the status bar (`app_bar.dart:1190`, `scaffold.dart:3049`).

`AppBar::primary` was therefore **deliberately absent**: at the reference's default of `true`
every bar inside a shell would have padded twice, and defaulting it to `false` would have
been the same name meaning the opposite thing.

And the cost was not cosmetic, as the roadmap said when it recorded this:

> an `AppBar` used **outside** a shell draws under the status bar, because nothing is
> insetting it and it will not inset itself.

## `Widget::media_override`

The counterpart of `theme_override`, and the same shape:

```rust
fn media_override(&self, _inherited: MediaQuery) -> Option<MediaQuery> { None }
```

Four walks make the swap, and they have to stay in step or the cache and the picture stop
agreeing with the layout: `build_layout`, `build_deferred`, the relayout fingerprint, and the
paint walk. The fingerprint also **hashes** the scoped surface — `MediaQuery::measure_hash` —
because two subtrees given different descriptions must not share one, and the trap is the one
`theme_override` already documents beside it.

`MediaScope` is the widget: `new` for a description entire, `tweak` for a change to what was
inherited. A transparent wrapper, like `Themed`.

The repository's own guard caught the rest of the work: `every_wrapper_states_the_hooks_the_
macro_leaves_out` fails the moment a hook joins the claimable list without every wrapper
saying what it does with it. `media_override` is claimable for the same reason
`theme_override` is — a transparent wrapper *is* its child, so the macro cannot decide.

## `SafeArea` resolved at the wrong time, for a reason that had expired

```rust
/// The padding this widget resolved when it was constructed. Kept rather than
/// recomputed in `style`, because `style` is also called from the layout cache,
/// outside any `MediaQuery::scope` the shell installed.
resolved: Insets,
```

True when written. **Milestone 408 made it false**: the shell holds one surface across the
build, the layout *and* the paint, cache walk included. A safe area resolves when it is asked
now, which is what lets a scope above it change the answer — and `SafeArea::padding()` is a
question about the surface in force, so its tests ask it under one.

## The split, at last

- `AppBar::primary` (default `true`, as in the reference) wraps its **toolbar and bottom** —
  not its surface — in a safe area with the bottom edge left free. The bar's colour still
  runs behind the status bar, which is what a Material bar looks like and what
  `app_bar.dart:1189` says in words.
- The `Scaffold` stops insetting the bar and hands its slot a **description** instead: the
  top it should believe in (`primary ? insets.top : 0`), and the left and right it has left.

It says it **even when the answer is the ambient one**, and that is not redundancy:
`Scaffold::insets` is an explicit override, so a slot told nothing would read past it to the
surface and pad by something the shell had already decided against. That was a real failure
in the middle of this work — three scaffold tests using an explicit `insets(…)` went on
reporting the old number until the scope carried it.

## What moved, and what did not

**No golden moved.** The golden scenes describe no intrusions, so a bar pads by zero — the
change is inert where there is no status bar, which is the right shape for it.

Three scaffold tests had to change, and they were worth reading rather than fixing: each put
a bare `Container` in the app-bar slot. That slot is not padded from outside any more, so a
bare box in it is a bare box that does not consume anything — which is exactly the
reference's behaviour, and the tests now use a real bar. One of them, incidentally, was named
for the body and had always measured the bar.

The new test is the one the roadmap asked for: a bar **on its own**, no shell, holds its
title off by exactly the intrusion.

## Still open

`Scaffold`'s other slots — body, footer, navigation — are still insetted from outside. They
can move to the same arrangement now that the mechanism exists, but each is a behaviour
change of its own and belongs in its own step.
