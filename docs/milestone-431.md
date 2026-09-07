# Milestone 431 — Two badges, two reds

Milestone 430 ended on a note: the navigation rail draws its badge with a red of its own,
one file over from the `Badge` widget that takes the scheme's `error`. Pulling that thread
found four more colours in the same paint, and one of them was wrong twice over.

## The indicator was the wrong role *and* the wrong kind of colour

The pill behind a selected destination was `theme.primary` at 16 % alpha. The reference
fills it with an **opaque `secondaryContainer`** (`navigation_bar.dart:1463`,
`navigation_rail.dart:1272`).

Both halves of that matter. The role is wrong — the indicator is a container, and
`secondary_container` is the role for a live-but-quiet fill, which an earlier milestone
already established when the slider's rail needed one. And the *kind* is wrong: a
translucent fill blends in linear light in this renderer, so 16 % does not paint at 16 %.
That is the trap milestone 328 measured and 329 resolved for the disabled tokens — a 12 %
wash painting at roughly 33 % — and it was still sitting here, on the one surface in a
navigation bar whose whole job is to say *this one*.

An opaque container is both the right role and immune to the trap.

## The glyph and the label are not the same colour

They were: both `theme.primary` when selected, both `theme.muted` when not.

The reference splits them, and the reason is geometric. The **glyph is drawn on the
indicator**, so it takes the indicator's content colour, `onSecondaryContainer`
(`navigation_bar.dart:1456`). The **label sits below the indicator**, on the bar's own
surface, so it takes `onSurface` (`:1476`). Painting both in one colour is only invisible
while that colour happens to read on both grounds.

The unselected pair was already right — `on_surface_variant` is exactly what `theme.muted`
is — with one exception. It is the one question the reference answers differently for the
two widgets: an unselected label is `onSurface` on a **rail**
(`navigation_rail.dart:1251`) and `onSurfaceVariant` on a **bar** (`navigation_bar.dart:1477`).
A rail's labels are always visible and are part of the page; a bar's are secondary to the
glyph above them. `NavItem` already carried a `rail` flag, so honouring the difference cost
one branch.

## One badge, one theme

The rail's badge carried `Color::rgb(0.90, 0.24, 0.24)` with a reason beside it: *an alert
dot reads as red whatever the theme*. That reasoning is defensible on its own, and it is
still the wrong answer, because the `Badge` widget in the same crate already answers the
same question from `scheme.error` and `scheme.on_error` through `BadgeTheme`.

**Two badges in one framework painting different reds is the part that is actually wrong.**
The rail's now reads the same theme, so an application that recolours badges recolours both,
and the test asserts exactly that: set `theme.widgets.badge.background_color` and the rail's
dot follows.

The count's `Color::WHITE` went the same way, to `on_error`.

## Two numbers

The glyph was 22 and the reference's is 24 (`navigation_bar.dart:1452`); the gap under it
was 2 and the reference's label padding is 4 (`:1483`).

## Themeable, as none of it was

`NavRailTheme` gained `indicator_color`, `selected_icon_color`, `unselected_icon_color`,
`selected_label_color`, `unselected_label_color` and `icon_size`. Before this every one of
those was a hard-coded read — not a themed default that could be overridden, simply
unreachable.

## The tests

- `a_destination_takes_the_roles_the_reference_names` — the indicator is the opaque
  container, the glyph and the label are the two different roles, and **the two roles
  differ**, without which asserting the split would prove nothing.
- `a_rail_and_a_bar_part_company_on_one_colour` — the rail's unselected label against the
  bar's, and that the two still agree on the glyph.
- `the_rail_s_badge_is_the_badge_widget_s_badge` — `error`, `on_error`, and then the theme
  moving both.

## Still open

The reference's destination also carries a **state layer** on hover and press
(`onSurfaceVariant` at the standard opacities). This crate paints a translucent hover pill,
which is a state layer of the right shape blended in the wrong space — the general question
that `frus-test/tests/blending.rs` pins and that the roadmap already carries. The indicator
is out of it now because a container is opaque; the hover is still in it.

`NavigationRail` has no extended form (`minExtendedWidth: 256`), no `groupAlignment`, and no
`labelType` — the reference can hide labels, show them on the selected destination only, or
show them all, and this shows them all.
