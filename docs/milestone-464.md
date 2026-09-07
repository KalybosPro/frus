# Milestone 464 — A floating action button was a helper that returned a button

There was no `FloatingActionButton` in this framework. There was `fab_button`:

```rust
pub fn fab_button<Msg>(label: impl Into<String>, message: Msg) -> Button<Msg> {
    Button::new(label)
        .variant(Variant::Filled)
        .size(24.0)
        .shape(ShapeBorder::stadium())
        .on_press(message)
}
```

Five lines, and it was the scaffold's entire answer for the most prominent control on a
screen.

## What that cost

**The colours were two roles out.** `Variant::Filled` means `primary` on `on_primary`.
The reference's floating action button takes `primary_container` on
`on_primary_container` (`floating_action_button.dart:809`) — a quieter, wider surface,
which is the whole visual idea of an M3 FAB. Every application built on this framework
has been drawing its main action in the wrong pair.

**There was one size**, where the reference has four, each carrying three numbers of its
own (`floating_action_button.dart:783`, `:816`, `:824`):

| | box | corner | glyph |
|---|---|---|---|
| small | 40 | 12 | 24 |
| regular | 56 | 16 | 24 |
| large | 96 | 28 | 36 |
| extended | 56 tall | 16 | 24 |

**There was no extended form at all** — no way to write "New list" next to a plus, which
is the form used whenever the glyph would not say what the action is.

**And it was a circle.** `ShapeBorder::stadium()` takes half the short side, so a 56-pixel
button rounded to 28. The reference rounds it to **sixteen**.

## The widget

`FloatingActionButton` with `new(icon)`, `glyph(text)`, `extended(label)`, `.small()`,
`.large()`, and the ten properties `FabTheme` mirrors.

`FabSize` carries its own box, corner and glyph, as one enum rather than three fields,
because all three follow from *which* of the four it is — separate fields would let a
caller build a large button with a small one's corner, which is not a design anyone wants
and is a bug report waiting to be filed.

`glyph(text)` is a plain button carrying **one character** — a plus, a tick, an arrow.
`IconButton::glyph` is the same escape hatch for the same reason: not every mark an
application wants is in `Icons`, and waiting for one to be added is not an answer.

## The corner, and the notch

This is the one place the reference's own number is wrong here, and it is worth writing
down rather than quietly deviating.

A **docked** floating action button sits in a circular notch that `BottomAppBar` cuts for
it. Rounded to sixteen, a 56-pixel square leaves four corners hanging over the bar. So
the framework's default is the reference's per-size rounding, and `fab_button` — which is
what the scaffold's docked examples use — asks for a stadium explicitly.

`Scaffold` cannot pick for the caller: it cuts the notch from the button's **bounds** and
never sees its shape. Recorded.

## `fab_button` survives, on top of the widget

```rust
pub fn fab_button<Msg>(label, message) -> FloatingActionButton<Msg> {
    FloatingActionButton::glyph(label).shape(ShapeBorder::stadium()).on_press(message)
}
```

Its two callers in the demonstration are unchanged, and both now get the right colours.
It went from being *the implementation* to being a two-line shorthand for it, which is
what a helper should be.

## The elevation

Six at rest, eight under a pointer (`floating_action_button.dart:778`), interpolated on
`hover_progress` so the button rises rather than jumping.

**Zero when disabled**, and zero when there is nothing to press. The reference's
`disabledElevation` is zero and the reason is worth stating: a control still floating
while refusing to be pressed is telling two different stories at once. `a_disabled_button_stops_floating` holds it, along with the other three things a disabled
button stops doing — answering, taking focus, and splashing.

## The tests

Nine, six of which fail when the milestone is undone — checked by putting the filled
button's colours back, making it a circle at every size, and taking the shadow away.

`an_extended_button_has_something_to_read_out` is the one with an opinion in it: an
extended button reads out its words, and a plain one carries **no label at all** rather
than an invented one. A glyph is not a name; a caller who wants it spoken wraps it in the
`Tooltip` milestone 462 built, which is what the reference's own `tooltip` argument is
for.

## The picture

**A new golden**, `floating_action_buttons`: all four together, which is the only way to
see that each size's three numbers are three numbers and not one.
