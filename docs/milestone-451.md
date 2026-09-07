# Milestone 451 — Six widgets that took a corner now take a shape

Milestone 450 built `ShapeBorder` and recorded the obvious next step: the widgets that
want a shape still took a radius. `Card`, `Chip`, `Dialog`, `Button`, the FAB and the
snack bar all take a `shape` in the reference.

Doing them together is the point. A property that means one thing on a card and another on
a button is worse than one that is missing.

## One rule

```rust
pub fn resolve_shape(
    own: Option<ShapeBorder>,
    themed: Option<ShapeBorder>,
    radius: Option<BorderRadius>,
    fallback: ShapeBorder,
) -> ShapeBorder
```

The caller's word, then the theme's shape, then the theme's plain **radius** read as a
rounded rectangle, then the widget's own default.

The third rung is the one that needs explaining. A radius was all a theme could say until
milestone 450, and applications have written them. So a theme naming a `shape` outranks
one naming only a `radius`, and naming both is naming the shape.

## One field

On the widget there is **one** field. `radius()` is a shorthand that writes a rounded
rectangle into it; `shape()` writes whatever it is given; the last one called is the one
that counts. Two builders naming the same property should behave that way, and the
reference has only the one.

## Two of them were wrong

**A button's stadium was `height / 2`.** That is the right number for a button wider than
it is tall and the wrong one for a button that is not — a stadium takes half its **short**
side. A 24×80 button asked for a 40-pixel corner on a 24-pixel box.

**The FAB was `radius(FAB_SIZE / 2)`** — the same number written out, right only while the
button was exactly that tall. It is a stadium now, so an extended one stays a lozenge
instead of a box with corners rounded past its own height.

Saying the word instead of the number gets both right at every size.

## And one was named after the reference and typed as something else

`DialogTheme::shape` carried the reference's *name* with a `BorderRadius` behind it. That
is the clearest case of the deviation this milestone closes: `shape` in the reference is a
`ShapeBorder`, and a corner radius is one of the things one can be.

It is one now, with `Dialog::radius` as the shorthand — a **breaking change** for a theme
or a caller that passed a number to `shape`.

## The shadows

Every one of these draws a shadow before its box, and a shadow is a blurred rectangle
whatever the shape is. So each takes the corners the shape *resolves to*
(`as_rounded(bounds)`), and none at all from a shape with no rounded form. A bevelled card
casts a square-cornered shadow, which is closer to right than a rounded one and is what
the reference's own shadow does for shapes it cannot outline.

## The tests

- `a_shape_outranks_a_radius_and_a_caller_outranks_both` — the rungs.
- `a_button_is_a_pill_at_any_size` — including the 24×80 case the old arithmetic turned
  inside out, and a circle taking a square out of the middle.
- `a_theme_shapes_a_card_over_its_radius`.

All three fail with the inferred shapes restored, and **the goldens did not move**.
