# Milestone 421 — The body was padded for intrusions it was never told about

Last of the four slots, and the one that is not a bar.

```rust
let body_pane = Flex::column().flex(1.0).child(inset_pad(
    body_widget,
    0.0,
    insets.right,
    0.0,
    insets.left,
));
```

The shell applied the side intrusions to the body's content and told the body **nothing**.
Both halves of that were wrong, and the second was the expensive one: a `SafeArea` inside a
body read the **ambient** description — the whole notch, as if no shell had been involved —
and padded for intrusions that had already been dealt with.

Under an app bar that is a whole status bar of empty space:

```
no bar       content at y = 40    (the safe area holds the notch off — correct)
with a bar   content at y = 136   (56 + 40 for the bar, then 40 again)
                          ^^^^ the bar had already taken it
```

## What the reference does

```dart
// scaffold.dart:3019
_addIfNonNull(
  children,
  _BodyBuilder(extendBody: …, extendBodyBehindAppBar: …, body: …),
  _ScaffoldSlot.body,
  removeLeftPadding: false,
  removeTopPadding: widget.appBar != null,
  removeRightPadding: false,
  removeBottomPadding: widget.bottomNavigationBar != null || widget.persistentFooterButtons != null,
  removeBottomInset: _resizeToAvoidBottomInset,
);
```

The body is laid out **full width** and handed a description: the sides kept, the top removed
when there is an app bar, the bottom removed when something below holds the edge off, and the
keyboard removed when the layout has already shortened the body for it.

Which is the sentence this milestone is really about: **a body that wants the notch avoided
says so**, and now it can, because the slot is told the truth about its edges.

## What changed

- The body is wrapped in a `MediaScope` and no longer in a padding.
- Its top is zero when an app bar stands in front of it, and the real intrusion when the body
  runs *behind* the bar (`extend_body_behind_app_bar`) — the `max(padding.top, appBarHeight)`
  half of the reference's `_BodyBuilder` is not in it yet, only the safe-area half.
- Its bottom is zero unless `extend_body` puts the body under the bottom slots, in which case
  it faces the edge itself.
- Its `view_insets.bottom` is cleared when `resize_to_avoid_bottom_inset` — the column has
  already shortened the body for the keyboard, so there is nothing left to avoid.

## `inset_pad` has no callers left

```rust
warning: function `inset_pad` is never used
```

Four milestones ago every slot went through it. That warning is the shape of the change: the
shell says what there is to consume, and each slot's widget consumes it. The helper is gone.

## The tests, and what they read on the old code

- `a_safe_area_in_the_body_is_told_what_the_bar_already_took` — 136 on the old code, where
  the bar ends at 96.
- `a_body_is_told_about_the_cutout_beside_it_rather_than_padded_for_it` — a bare body's
  background reached `x = 48` instead of the screen's edge, and a body that asked for a safe
  area could not have been given one, because the description said the cutout was still
  there after the shell had padded for it.

No golden moved (91 + 13 + 27).

## Still open

**The bottom half of the reference's body contract is deliberately not in this step.** The
reference shortens the body for the keyboard and for the widgets below it, never for the
gesture bar — `minInsets.bottom` is `resize ? viewInsets.bottom : 0` (`scaffold.dart:3220`) —
so a plain body runs to the screen's edge and its own `SafeArea` is what holds it off. frus
still shortens it by `bottom_clear`, and tells it `padding.bottom = 0` to match, which is
self-consistent but not the reference.

Moving it is a change of contract for every screen already written: a body that says nothing
would stop being held clear of the gesture bar. It belongs in its own step, with the guides
and the examples moved with it.

Also left from `_BodyBuilder`: `max(padding.top, appBarHeight)` and
`max(padding.bottom, bottomWidgetsHeight)`, the two places where an extended body is told how
far the slot it runs under actually reaches.
