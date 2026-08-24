# Milestone 401 — Saying what a widget is, from outside it

Two things that turned out to be one thing. The roadmap had a 🔴 saying there was no way
to put a role on an already-assembled widget; the same day, the observation arrived that
the reference's `AppBar` title is a **widget** and ours was an enum with a string in it.
The second is the reason the first mattered.

## The gap

Every widget in `frus-widgets` answers `Widget::semantics` for **itself**. That covers the
ordinary case and misses the one that keeps coming up: a caller is handed a built widget
and knows something about it that the widget does not know about itself.

Milestone 397 walked straight into it. An `AppBar` marked its **text** title as a heading —
the landmark a screen reader's user jumps between — and could do nothing of the sort for a
*widget* title, because by then the title is a `Box<dyn Widget>` and the bar has no way in.
So a bar's accessibility depended on which constructor the caller had reached for. That is
not a distinction anybody using assistive technology should be able to feel.

## The rename that had to come first

The reference's `Semantics` is the **widget**; the data bag is `SemanticsProperties`. Ours
had the data called `Semantics`, which is the name the widget needs.

So `frus_core::Semantics` → `SemanticsProperties`, 32 files, mechanical. Same argument as
milestone 369 renaming `IconName` to `Icons`: somebody who knows the reference types what
they know, and a name that is *nearly* right costs more than one that is plainly different.
The historical record keeps the old name — earlier milestone notes describe what was true
when they were written.

## The widget

`Semantics::new(props, child)` states a role, a name or a state for a child that cannot
state it for itself. Two constructors carry the common cases: `Semantics::heading(child)`
and `Semantics::merge(child)`, the latter being the reference's separate `MergeSemantics`
widget spelled as the constructor it is.

It is read in **one place** in the walk, the way a `ModalBarrier` is: the subtree is walked
exactly as usual and what it produced is reconciled afterwards. Walking first and
reconciling after is what makes it exact — a widget deep inside annotates itself without
knowing anything above is speaking for it, including widgets written after this code.

### Adding is the default; merging is asked for

By default it **adds** a node and leaves the child's alone. `merging` drops the subtree's
annotations and speaks for all of them, carrying over what they said.

The additive default is not the reference's — its `Semantics` makes a container node with
the child's nested inside — and the reason is worth stating. **The destructive behaviour
cannot be undone by the caller.** Wrapping a whole screen to name it, under a merging
default, would collapse every control on it into one node. The opposite mistake leaves one
node too many, which is noise rather than loss.

`SemanticsProperties::over` is the merge: this one's answers win, the other's show through
wherever it said nothing. **Two labels are joined, one line each** — as the reference does
for a merged subtree — because picking one would drop the other with nothing to say which.
A container that knows a child is a heading rarely knows what the words are; dropping them
would replace a spoken title with silence, which is worse than the unlabelled heading it
was fixing.

### What our flat tree costs, stated plainly

This framework's accessibility tree is **flat**: every annotated widget is a child of the
window (`a11y.rs:117`). So merging really does discard, where the reference merges into a
container node and keeps the shape. A nested structure a reader could descend into is not
something ours can express yet. Said here rather than discovered later.

## The title is a widget

`app_bar.dart:1067` is three lines and ours is now the same three:

```dart
title = _AppBarTitleBox(child: title);
if (!excludeHeaderSemantics) title = Semantics(header: true, …, child: title);
title = DefaultTextStyle(style: titleTextStyle!, softWrap: false,
                         overflow: TextOverflow.ellipsis, child: title);
```

`enum Title { Text(String), Widget(…) }` is gone. `AppBar::new("Inbox")` builds a plain
`Text` with **no style on it** and hands it to the same path `AppBar::title(widget)` uses.
`title_widget` is renamed `title`, because there is one kind of title now.

Three consequences, and the third is the one I did not expect:

1. **Every title is a heading**, not only a string one. The 397 asymmetry is closed, and a
   test asserts the two constructors answer identically.
2. **The type is handed down, not applied.** A `Text` inside a caller's widget picks up the
   bar's `title_large` because it never chose a size; one that chose keeps its own. That is
   milestone 400's rule, and it is what makes handing a style to a widget the bar never
   looked into safe.
3. **The manual truncation is gone.** `crate::text::truncate(&content, &style, title_room)`
   cut the string against a width computed before the layout ran — the same shape of
   mistake as milestone 392. Now `soft_wrap: false` and an ellipsis come down with the
   style, and the words are cut by the box they are actually given.

The `ConstrainedBox` around the title stays: it is the reference's `_AppBarTitleBox`. The
inherited ellipsis alone would very nearly bound the title — a text handed an overflow mode
grants the squeeze — but "very nearly" is how an over-long task name evicted its own delete
button in milestone 333. The bar knows what is left after the actions have taken theirs;
saying it outright is one line and makes the guarantee testable.

## Left

- **`namesRoute`** — the reference announces a screen's title on arriving at it, and the
  shell already has a live region (`a11y.rs:29`) to say it through. Needs a route-change
  signal to reach the shell, which is the same missing wire as the navigator's `observers`.
- **`hint` and `selected`** on `SemanticsProperties`. AccessKit has both; nothing in the
  widget crate produces either yet, so adding the fields alone would be two more properties
  that ship unusable.
- **A nested accessibility tree.** The flat one is a real limitation, not a simplification
  that happens to be fine; `explicitChildNodes` and `container` have no meaning until it
  exists.
- **`Text`'s style fields should be `Option`s**, not a `TextStyle` plus milestone 400's
  `Chosen` record. The two carry the same information, but the reference's nullable fields
  let a caller say *size 20, inherit the weight* and ours cannot: `TextStyle::new(20.0)`
  answers all three. Next milestone.
