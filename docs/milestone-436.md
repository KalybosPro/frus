# Milestone 436 — What a destination could not say about itself

A destination was a tuple:

```rust
type Destination = (String, String, Option<u32>);
```

A glyph, a name, and a number. That is exactly as far as a tuple goes, and it is three of
the seven things the reference lets a destination carry. The other four had nowhere to live,
so they were never written.

`NavigationRailDestination` (`navigation_rail.dart:1101`) has `icon`, `selectedIcon`,
`indicatorColor`, `indicatorShape`, `label`, `padding` and `disabled`. This milestone gives
the tuple a name and adds three of the missing four.

## A destination that cannot be reached

`disabled` (`:1161`) is the one with actual behaviour behind it, and the reference spends
two lines on it:

```dart
// :717, :723
data: widget.disabled
    ? widget.iconTheme.copyWith(color: theme.colorScheme.onSurface.withOpacity(0.38))
    : widget.iconTheme,
…
onTap: widget.disabled ? null : widget.onTap,   // :957
```

The glyph and the label take **one** rule — `on_surface` at 38 %, which is
`disabled_content` here, resolved opaque rather than handed to the GPU as an alpha
(milestone 329's finding: 38 % in linear light does not paint at 38 %). The tap goes away.

The other two follow rather than being properties of their own:

- **Nothing lights under the pointer.** A hover is the promise of a click, and there is no
  click here. The reference gets this for free from an ink well with a null `onTap`; ours
  paints its own state layer, so the hover branch is guarded.
- **The keyboard steps over it**, as it steps over a disabled button — `focusable()` returns
  `!disabled`, which is what `Button` already answers with `self.enabled`.

The indicator stays. A disabled destination that is also the selected one still shows that
it is selected; the reference wraps the indicator outside the `disabled` branch, and it is
right — greying out a destination says you cannot go there now, not that you are not there.

## A selected destination can look different, not just coloured

`selectedIcon` (`:1132`) exists because colour alone is a weak signal. The reference's own
advice is to pair a stroked icon with its filled version: `Icons.cloud_queue` at rest,
`Icons.cloud` when selected. This framework has no icon font — a destination's icon is a
text glyph — so the same idea is `.selected_icon("★")` beside `.item("☆", "Starred")`.

Unset, the resting glyph is used in both states, which is the reference's default
(`selectedIcon = selectedIcon ?? icon`).

## A destination's own indicator colour

`indicatorColor` (`:1144`) is per **destination**, not only per theme. It sits above the
theme's `nav_rail.indicator_color`, which sits above the scheme's `secondary_container` —
three rungs, and the narrowest wins. It is how one entry in a list marks itself out from the
rest without restyling the list.

## The tuple gets a name, and both shells get the properties

`Destination` is a struct now, and — this is the part that matters — it is the **same**
struct `Scaffold` and `NavScaffold` collect. They each used to keep their own
`Vec<(String, String, Option<u32>)>` and rebuild the widget's destinations field by field on
the way out, which meant a property added to the rail was a property the shells silently
dropped.

That is milestone 434's lesson one level down, so the answer is the same shape: one type,
declared once, handed over whole. The three new decorators are one line each on all four
builders (`selected_icon`, `disabled`, `indicator_color`), and they follow `badge`'s rule —
they decorate **the destination just added**.

## The tests

- `a_destination_that_cannot_be_reached_says_so` — all four consequences, and then the same
  four on a live destination, because "no message, no focus, no hover, grey ink" proves
  nothing unless the other one has all of them.
- `a_selected_destination_can_show_a_different_glyph` — both states, and a destination that
  names no second glyph keeping its first in both.
- `a_destination_can_carry_its_own_indicator_colour` — over a theme that names a different
  one, and the theme's when the destination says nothing.
- `the_shell_forwards_a_disabled_destination` — through `Scaffold`, by hit-testing the bar:
  the live destination emits its message at the coordinates the test claims, and the
  disabled one emits nothing at its own.

Run first against the code without the change: exactly those four fail with the three
properties dropped on the way into the item.

## Still open

`indicatorShape` and per-destination `padding` (`:1148`, `:1159`). The shape wants a shape
abstraction this framework does not have — everything is a rectangle with a corner radius —
and the padding wants the item's paint to inset itself from its own box, which is a real
change to how a destination measures its content rather than a field to thread through.
