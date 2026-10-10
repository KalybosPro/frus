# Milestone 641 — BoxDecoration: shape, sides and shadows

## Objective

`BoxDecoration` in the review against the reference (`painting/box_decoration.dart`,
`painting/box_border.dart`, `painting/box_shadow.dart`). frus's decoration was a subset:
one shadow, a uniform border and no shape. The reference's work is split over three
milestones, because two parts need new work in the GPU renderer:

1. **This one:** the shape, the border side by side, and the list of shadows.
2. Gradients: linear with begin, end, colours, stops, tile mode and transform; radial; sweep.
   This needs colour ramps in the renderer.
3. `DecorationImage`, `backgroundBlendMode`, and the shadow's `blurStyle` (inner, outer,
   solid).

| | reference | frus before |
|---|---|---|
| `shape` | `BoxShape.rectangle` or `circle`: a circle centred in the box, as wide as its shorter side (`box_decoration.dart:436`) | none |
| `boxShadow` | a **list**, painted in order (`:452`), interpolated pair by pair (`BoxShadow.lerpList`) | one shadow |
| `border` | `Border` (top, right, bottom, left) or `BorderDirectional` (top, start, end, bottom), each side its own colour and width; sides of one colour keep round corners and the circle, sides of several are drawn one by one (`box_border.dart:654`) | one width and one colour for all four sides |

## What was done

- **`BoxShape::{Rectangle, Circle}`** and `BoxDecoration::shape`. A circle is painted, and its
  shadows and border drawn, as the circle inscribed in the box's shorter side. A container
  that clips with a circle clips to the ellipse inscribed in its box, which is the circle in
  a square box.
- **`BoxDecoration::shadows: Vec<BoxShadow>`**, painted in order behind the box.
  `BoxDecoration::shadow(s)` and `Container::shadow(...)` add one, and `shadows(...)` sets
  them all. Two lists interpolate pair by pair; the extras of the longer one grow from
  nothing or shrink to nothing.
- **`Border { top, right, bottom, left }`** made of `BorderSide`s:
  - constructors `Border::new(width, colour)` (kept), `all`, `symmetric` and `NONE`;
  - `is_uniform`, `dimensions` and `lerp`.

  **`BorderDirectional { top, start, end, bottom }`** with `resolve(direction)`. **`BoxBorder`**
  is either of the two; `BoxDecoration::border(...)` takes both.
- **Painting a border whose sides differ:**
  - In one colour, with round corners or on a circle, a single band between the box and the
    box inset by each side (an even-odd path; the inner corners are the outer ones less the
    thicker side).
  - Otherwise each side is its own trapezoid, mitred at the corners.
  - A uniform border is still drawn with the fill, in one primitive.
- **The room a border takes** is each side's own width (`content_padding_in(direction)`), and a
  container's padding follows it in the reading direction.
- **`BoxDecoration::paint_into_in(scene, rect, opacity, direction)`** places a directional
  border; `Container`, `DecoratedBox` and `DecoratedBoxTransition` pass the theme's direction.
- **`Container`** keeps a real decoration, so what a `BoxDecoration` says now reaches the
  paint: its shadows and its shape were lost before. New builders: `box_border(...)`,
  `shape(...)` and `shadows(...)`.
- **`Path::append`**: one path after another, for the band's hole.

**Changed:** `BoxDecoration` is `Clone`, no longer `Copy`, because a list of shadows is not
`Copy`. Its field `shadow` is now `shadows`, and `Border` has four sides instead of `width` and
`color`; `Border::new(width, colour)` builds the same uniform border as before.

## Not done here

- `BorderSide::style` and `strokeAlign`.
- A circle's clip in a box that is not square is an ellipse, where the reference's is the
  circle.
- Several colours with round corners: the reference refuses them (an assertion,
  `box_border.dart:711`). frus draws the sides one by one, ignoring the corners.
- Gradients, `DecorationImage`, `backgroundBlendMode` and `blurStyle`: the next two milestones.

## Tests

- `frus-core`: a circle centred on the shorter side with a round shadow; shadows painted in
  order; four colours drawn as three trapezoids when one side has no width, with the padding
  following each side; one colour with corners as one band (two sub-paths); a uniform border
  still one primitive; a directional border on the left in left-to-right and on the right in
  right-to-left; shadow lists pairing up and shapes swapping half way; the border's
  constructors and its lerp.
- `frus-widgets`: a container's padding by side and direction, its circle clip, its shadows
  accumulating, and no border at width 0.
- `frus-test`: a new picture, `decoration_shapes_sides_shadows`: a card with two shadows, a
  ringed circle with its shadow, four coloured sides, and a one-colour band with round
  corners.

Mutation testing: ten mutants, nine killed at once. The survivor treated every border as one
colour: the test of several colours was on square corners, where the band is never drawn
anyway. The test now also draws them with round corners, side by side, which kills it. A
check that the band's hole starts where its corners say was added before the run, and kills
the mutant that left the inner corners at the outer radius.
