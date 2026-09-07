# Milestone 438 — A button that cannot say what it does

A snack bar could carry an action and nothing else. The reference gives it a second control:
the close cross at the end of the bar (`snack_bar.dart:700`), a way out of a notification
that does not involve waiting for it or hitting the one button it offers.

## The reference's `bool` cannot be a `bool` here

`showCloseIcon` is a boolean over there, and that works because a `ScaffoldMessenger` owns
the bar: the button it builds calls `hideCurrentSnackBar` on the messenger it can reach
through the context (`:706`).

Here the application owns the queue. `SnackBarQueue` is deliberately application-side — pure,
tickable, testable, with no timer inside the widget — so there is nothing for a cross to call.
A `show_close_icon(true)` would draw a cross that does nothing, and **a button that cannot
say what it does is worse than no button**: it looks like a way out and is not one.

So it takes the message: `close_icon(Msg::Dismiss)`. The bool is the reference's answer to
its own ownership model, not a property to copy across one that differs.

## Why it is not an `IconButton`

There is a perfectly good `IconButton` in this crate, and the cross is not one, for the same
reason the action button is not a `Button`: a standard `IconButton` grounds its state layer
on `Color::TRANSPARENT`, and lerping *from* transparent produces a low-alpha colour that the
GPU then blends in linear light. On an inverted surface that reads as a patch of the wrong
colour rather than as a tint.

`CloseButton` grounds on the bar's own surface — the caller's, the theme's, or
`inverse_surface` — exactly as `ActionButton` does, so the layer is one opaque lerp. Two
milestones running (437, and now this) the same rule keeps deciding the same thing.

## The arithmetic

- **Width.** The bar reserves `ICON_BUTTON_SIZE` plus a margin either side. The reference's
  icon margin is a *twelfth* of the bar's horizontal padding (`:698`) — as near to nothing
  as a margin gets, because the cross is meant to sit at the very end of the bar. The bar's
  trailing padding drops from `PAD_X` to that margin when a cross is present, for the same
  reason.
- **Height.** A cross is a 40-pixel box where the action is 32, so the bar's floor follows
  whichever is present. Without this a bar of small type is 33 tall and cuts the top and
  bottom off its own cross — which the test asks about with an 8-pixel content style,
  because at the default type the message alone already makes the bar tall enough and the
  floor never binds.
- **Order.** The cross goes *after* the action (`:742`), so the action keeps the place a
  reader looks for it whether or not there is a cross.

## The label

The reference names the button from `MaterialLocalizations` (`:709`). This framework has no
equivalent — no widget here reads a localisation table for its own strings — so the cross is
`"Close"` until a caller says otherwise with `close_icon_label`. That is the same shape
`Image::semantic_label` already has, and the gap is worth recording rather than papering
over.

## The tests

- `a_close_icon_is_a_button_that_says_what_it_does` — one child, its message, its focus, and
  the name a reader hears, translated.
- `the_cross_comes_after_the_action` — both present, in the reference's order.
- `a_bar_with_a_cross_makes_room_for_it` — wider by the cross's box, and tall enough for it,
  asked at a type size where the floor actually binds. It fails with the floor reverted.
- `the_cross_takes_the_ink_that_is_legible_on_the_bar` — the scheme's, the theme's, the
  caller's.
- `the_cross_grounds_its_state_layer_on_the_bar` — alpha 1, and equal to the theme's rule
  over `inverse_surface`.

Four of the five could not have been run against the old code at all: the builder they call
did not exist. Only the height floor is a change to behaviour that already existed, and that
one was reverted and watched to fail.

## Still open

`SnackBarBehavior` (`:986`) — the reference's `fixed` bar spans the screen with square
corners and 24 pixels of horizontal padding, and its `floating` one is inset, rounded and 16.
This framework's bar hugs its content and a `ScaffoldMessenger` places it, so the two
behaviours are a genuine design question rather than a field: `SnackBarPosition` already owns
half of what `behavior` decides over there.

`actionOverflowThreshold` (`:998`) waits on it. It exists because a full-width bar makes the
action compete with the message for the line; a bar sized to its content has no such
competition, so the property has nothing to decide until `fixed` does.
