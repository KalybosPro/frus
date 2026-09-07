# Milestone 423 — The shell was spending the notch on the body's behalf

Milestone 421 gave the body a description and left one half of the contract alone, on
purpose, because it is a change of behaviour for every screen already written. This is that
half.

## What the reference actually does

```dart
// scaffold.dart:3220
final EdgeInsets minInsets = MediaQuery.paddingOf(context)
    .copyWith(bottom: _resizeToAvoidBottomInset ? MediaQuery.viewInsetsOf(context).bottom : 0.0);
```

```dart
// scaffold.dart:1088
final double contentBottom = math.max(0.0, bottom - math.max(minInsets.bottom, bottomWidgetsHeight));
```

Read the `copyWith`. `minInsets.bottom` is **not** the gesture bar — it is the keyboard, or
zero. So the body is shortened for the keyboard and for the widgets below it, and **never
for the system's own bottom intrusion**. A plain body runs to the screen's edge, and the
description it was handed at `scaffold.dart:3032` still says the intrusion is there:

```dart
removeBottomPadding: widget.bottomNavigationBar != null || widget.persistentFooterButtons != null,
```

Removed when something below it holds the edge off. Kept otherwise — so that a body which
must be clear of the gesture bar can say `SafeArea` and be answered.

frus did the opposite. It shortened the body by `bottom_clear`, which is
`max(insets.bottom, view_insets.bottom)`, and then — since 421 — told the body
`padding.bottom = 0` to keep the two consistent. Self-consistent, and not the reference.

## Why it is worth changing

Not fidelity for its own sake. A shell that spends the intrusion for you makes three
ordinary things impossible:

- a background or a hero image that should reach the bottom of the screen;
- a list that should scroll *under* the gesture bar, which is what edge-to-edge looks like;
- any screen that wants the room and would have to fight the shell to get it.

And it makes the ones that *do* want the clearance invisible: every screen paid for the notch
whether it used the room or not, and none of them said so.

## What changed

- The body's spacer is `resize_to_avoid_bottom_inset ? view_insets.bottom : 0` — the
  keyboard, and nothing else.
- The body's description keeps the real bottom intrusion, unless a bar or a footer below it
  holds the edge off.
- Told to run **under** the bottom slots (`extend_body`), it is told the further of the two:
  the intrusion, or how far the slot it runs under reaches. That is `_BodyBuilder`'s
  `max(padding.bottom, bottomWidgetsHeight)` (`scaffold.dart:969`); `nav_h` was hoisted out
  of the floating button's block for it. The footer's own height is still missing from that
  second term, because nothing measures it.

## Two tests changed, and they were the ones to change

`a_body_alone_still_clears_the_navigation_bar` was the old contract's name for itself. It is
`a_body_alone_reaches_the_edge_and_is_told_what_is_there` now, and it asserts **both** halves:
the plain body reaches `H`, and the body wrapped in a `SafeArea` stops at `H - 30`. The
second half is the one that could not have passed before — the description said the bottom
was already dealt with, so a safe area in the body had nothing to hold off.

`the_keyboard_shortens_the_body_unless_the_screen_declines` kept its first half unchanged (a
body still stops at the keyboard) and its second half now reads the reference's answer: a
screen that declines the resize has said the keyboard is an **overlay**, `minInsets.bottom`
is zero, and nothing shortens the body at all.

## The migration, such as it is

Every screen in the demo has something below its body — a bottom bar, a bottom app bar, a
persistent footer — so none of them changed. The guide did: the `Scaffold` module doc said
that none of `extend_body` and its siblings "lets content sit under the system's own bars,
which are not the application's to spend". That sentence was the old contract, stated as a
principle. It now says what is true, and `Scaffold::body` shows the one line a screen adds
when it wants the old behaviour:

```rust
.body(SafeArea::new(form))
```

No golden moved (91 + 13 + 27): the golden scenes describe no intrusions.

## Still open

`_BodyBuilder`'s `max(padding.top, appBarHeight)`, the other half of what an extended body
is told, waits on a widget being able to measure the slot it runs under — the roadmap entry
that is now down to its measuring half.
