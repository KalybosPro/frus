# Milestone 308 — The card was three cards at once

The sweep continues from milestone 307. This one is a single widget, and it is the
clearest example so far of the third shape of deviation that note named: **not wrong so
much as unfinished** — one thing where the reference has several, and nothing the caller
can change.

## What it was

```rust
pub struct Card<Msg> {
    padding: f32,
    children: Vec<Box<dyn Widget<Msg>>>,
}
```

Two fields, one of them the child. Everything a card looks like was written into its
`paint` as a literal: a shadow at blur 12, dropped 4, at 30 % alpha; the theme's radius;
the theme's plain `surface`; and a 1 px border in the theme's outline.

## What the reference has

Three cards, not one:

| | shadow | outline | surface |
|---|---|---|---|
| elevated | yes, at elevation 1 | no | a container tone |
| filled | no | no | a flatter container tone |
| outlined | no | yes | the plain surface |

They are not decorations of the same thing. Each says something different about how far
the card should stand off what is behind it, and the choice between them is the
application's. Ours drew a shadow **and** an outline — which is none of the three, and
reads as a card that cannot decide.

Plus `color`, `shadowColor`, `elevation`, `shape`, `margin` and `clipBehavior`, all
overridable, against a theme, against generated defaults.

## What it does now

`CardVariant::{Elevated, Filled, Outlined}`, with `Card::filled()` and
`Card::outlined()` as the short way to say it. Elevated is the default, so a card that
is not told anything looks like the reference's default card.

The outline now belongs to exactly one variant. The shadow belongs to two — the elevated
one by default, and any card given an explicit `elevation`, because a filled card raised
on purpose is a thing an application may want and refusing it would be the same mistake
one level down.

**Elevation is a height, not a blur radius.** `Card::elevation(6.0)` does not mean "blur
by six": the blur and the drop are both derived from the depth, so a card 1 px off the
page casts a tight shadow under its own edge and one 6 px off casts a wide soft one
below it. That is the property the reference's `elevation` has, and it is what makes a
number comparable between two widgets.

`color`, `radius` and `margin` are the caller's too. The margin defaults to the
reference's **4 px** — cards are usually stacked, and two flush against each other read
as one surface.

Two things are deliberately **not** the reference's:

- **The padding stays.** The reference's card has none and leaves it to the content.
  This one keeps its default of 16, because a card whose text touches its own edge is
  the more common mistake and the default costs one call to undo. It is written down in
  the builder's own documentation as an addition rather than passed off as parity.
- **The colours are the nearest this scheme carries.** The reference reaches for
  `surfaceContainerLow` and `surfaceContainerHighest`; this palette has
  `surface_container` and `surface_container_high`. Using the nearest tone is a better
  answer than inventing two more roles for one widget, and the comment at the site says
  which is which.

## Verification

- Five tests, and the first is the milestone: each variant painted, counting shadows and
  measuring the outline, so *elevated has a shadow and no outline* is a fact the suite
  holds rather than a claim in a note.
- One that the three sit on three different tones — a variant that changes nothing
  visible is not a variant.
- One on elevation as a height: zero removes the shadow, an explicit elevation gives a
  filled card one, and a taller card blurs wider **and** drops further.

  That last assertion was wrong the first time and the test caught it. The rectangle
  handed to `shadow` is the card's box grown by the blur on every side, so its `y` runs
  *upwards* as the blur grows — reading the drop straight off it says a taller card
  drops less. The test now takes the growth back off before comparing. A shadow's
  bounding box is not its position.
- One that the margin is there by default and gone when refused, and one that the colour
  and the rounding follow the caller.
- 1003 workspace tests, `clippy` silent on every target, `rustdoc` clean under
  `--all-features`.
- **Two goldens moved, both looked at.** `controls_toggles` and `small_indicators` are
  the two scenes with a card in them, and both show the same thing: the hairline gone
  from around a card that also casts a shadow, and the surface a tone up from the page.
  That the rest of the suite did not move is the useful half — the change reached the
  cards and nothing else.

## Left

- **`clipBehavior`.** The reference can clip its child to the card's rounding; here a
  child with a square background still squares off the corners it sits in.
- **Ink.** A card is not tappable in this framework — the reference's `InkWell` inside a
  `Card` is the usual way a list of cards is built, and with milestone 306's ink now in
  place, `Card` taking an `on_click` and a splash is a small step that has not been
  taken yet.
- **The theme still carries no per-widget defaults**, which is the same gap milestones
  306 and 307 both ended on: an application can set one card's elevation, not every
  card's.
