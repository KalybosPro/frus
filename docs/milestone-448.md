# Milestone 448 — A theme is eight kilobytes, and it was `Copy`

Milestone 447 built `WidgetStateProperty` and could only give it to widgets. The reference
keeps most of them on **themes** — `NavigationBarThemeData.overlayColor`, and every button
theme in the library. The blocker was recorded the same day: `WidgetThemes` derives `Copy`
and a property owns a `Vec`.

Removing `Copy` turned out to be worth doing on its own account.

## The measurement

```
Theme        = 7952 bytes
WidgetThemes = 5816 bytes
```

Eight kilobytes, and `#[derive(Clone, Copy, …)]`. Which means every `*theme` in the crate
was an eight-kilobyte `memcpy` that read like a pointer copy. There were nine of them:

- the layout walk, once per `LayoutBuilder`;
- `Themed`, once per themed subtree, plus `std::mem::replace` on the walk's own field;
- the app bar's icon theme, the date picker's, the table's;
- once per overlay pushed;
- and once for the walk's initial state.

None of them is in an inner loop, which is why nothing was ever slow enough to notice. But
`Copy` on a struct this size is a hazard rather than a convenience: it makes the expensive
thing invisible, and the next person to write `*theme` inside a loop gets no warning at
all.

So the nine are now `clone()` calls that say what they cost, and the type says it too.

## What it unlocks

`NavRailTheme::overlay_color` is the first theme field that is a property rather than a
value. The resolution is the framework's usual three rungs:

- the destination's own `overlay_color` (milestone 447);
- then the theme's;
- then the framework's state layer.

And **a property that matches no entry falls through to the next rung** rather than
answering with a default. That is what makes naming one state safe: a theme that says
something about `Pressed` has not thereby said that `Hovered` is nothing.

## The cost, honestly

`Theme` is now `Clone` and not `Copy`, which is a **breaking change** for anyone writing
`*theme` or passing a theme by value twice. The fix is `.clone()` at each site; there were
thirteen across the workspace, four of them in the demo and one in `frus-test`.

The walk still copies eight kilobytes when it enters a themed subtree. An `Arc<Theme>`
would make that a refcount bump, and would make `Themed` nearly free — recorded, not done,
because it changes the signature every widget's `paint` and `style_themed` is written
against.

## The tests

- `a_theme_answers_for_every_destination_and_one_may_still_answer_for_itself` — the middle
  rung, and the destination outranking it. Fails with the theme's rung removed.
- `a_state_neither_rung_names_falls_through` — the semantics of saying nothing. It is not
  a revert guard (with no theme rung it passes trivially); it pins the rule that a partial
  property does not silence the rest.
- `a_theme_is_far_too_big_to_copy_by_accident` — asserts the size is *large*, not that it
  is 7952. The number is not a promise; being big is the whole argument.
