# Milestone 397 — A heading nothing emitted, marked done on the roadmap

This one started as three small app bar properties and turned into an accessibility bug
that had been sitting behind a green tick.

## What was wrong

The roadmap said:

> 🟢 **The app bar's title carries no `Role::Heading`.**

Green. Done. Except **nothing in the framework emitted `Role::Heading` at all** — one grep
across every crate found the enum variant's declaration and one place that threw it away:

```rust
Role::Heading => AkRole::Label, // no distinct Heading role is used here
```

That comment was true of an older AccessKit. It is false of the one we depend on:
`accesskit 0.24` has `Role::Heading`. Nothing had gone back to look, and the mapping had
been quietly discarding the role for however long.

So there were two failures stacked on each other — a role nothing produced, and a
translation that would have dropped it if anything had. Either alone is invisible; together
they read as a working feature.

## Why it matters

A screen reader's user does not read a screen top to bottom. They **jump between its
headings**. The heading every screen has is its app bar's title, and ours was announced as
`Role::Label` — one more piece of text among the rest, with nothing to jump to.

## The fix

- **`a11y.rs`** maps `Role::Heading` to `AkRole::Heading`.
- **`Text::heading()`** — a text that is a landmark rather than prose. It changes nothing
  that is drawn.
- **The bar's title is a heading**, which is what the reference says with
  `Semantics(header: true)` around its own (`app_bar.dart:1079`).
- **`AppBar::exclude_header_semantics`** — the reference's switch, `false` by default. For a
  bar whose title is decorative, or one of two bars on a page where only the outer one names
  the screen: announcing both as headings gives the user two landmarks where there is one
  screen.

The test asserts the role both ways round, so the switch is pinned rather than described.

## What it stops short of

Only a **text** title becomes a heading. A widget title keeps whatever semantics it brought,
because this framework has no way to put a role on something already assembled.

The reference does: a `Semantics` widget, which wraps any child and states its role. We have
none, and the milestone-336 catalogue count did not miss it so much as never ask — it counts
classes extending a widget base, and the gap it leaves is exactly this one. It is on the
roadmap now, because half-answering it here would have meant a title whose accessibility
depended on whether the caller passed a string or a widget.

## The rest of the mapping, checked

Having found one role being flattened, the obvious question is how many others are.
Answer: **none**. Every other arm of `to_ak_role` names the role it means — `Button`,
`Switch`, `Slider`, `Tab`, `ListItem`, `ProgressIndicator` — and `Heading` was the only one
carrying a comment that explained away a downgrade.

What is still missing is the thing that would have caught it: nothing asserts that a role
this framework emits survives to AccessKit. The compiler checks the names exist; it cannot
check that `Label` was the right answer for `Heading`. That test does not exist, and it is
the kind that only earns its keep when a dependency moves under you — which is exactly what
happened here.
