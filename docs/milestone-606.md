# Milestone 606 — Shadows as the reference casts them, and the text selection bar

A phone running an application built with frus showed the bar over a text selection (Cut,
Copy, Paste, Select all) with a **large grey box around it**, and a bar that did not look like
the platform's. Two questions came with it: why is this bar not like the reference's, and why
do widgets have a shadow at all?

## The answer to the second question

The reference does lift these surfaces: the selection bar by 1, a card by 1, a menu by 3, a
floating button by 6. **But its shadows are small and light.** Every height is three shadows,
from a table:

- a sharp one close under the surface, at a fifth of the shadow colour's strength;
- a softer one further down, at a seventh;
- a wide faint one all round, at an eighth.

At a height of 3 they are blurred by 3, 4 and 8 pixels.

Here, fifteen widgets each had a copy of their own formula instead: one shadow blurred by
**four times the height plus eight**, at 30 to 35 % of black. A height of 3 was blurred by 20
pixels, and the shadow's rectangle grown by those 20 on every side.

## The grey box

That alone would have given heavy shadows. The **box**, with a visible edge, came from the
renderer.

A shadow is drawn as a rectangle that already reaches its blur beyond the shape, and nothing
is drawn outside that rectangle. The shader faded the shadow across the rectangle's edge, from
the blur inside to the blur outside. Only the inner half was ever drawn, so **every shadow
stopped short, at half its strength**, in a hard grey line.

The renderer's own comment said the right thing: the blur "softens the edge *inside* the quad".
The shader now does that. The fade runs from the shape's edge less the blur, at full strength,
to the rectangle's edge, at nothing.

The goldens show what that line looked like. Every slider thumb had a grey square around it.

## Shadows now

- **`BoxShadow::for_elevation(height, colour)`** gives the reference's three shadows for a
  height. Between two heights of the table it interpolates, so a height that animates moves
  smoothly, and past 24 it stays at 24. Lifting off the page fades in over the first unit of
  height rather than jumping.
- **`paint_elevation(scene, rect, radius, height, colour)`** paints them. It paints nothing at
  a height of 0 or in a colour with no opacity.
- **All fifteen widgets use it**: banner, button, card, dialog, drawer, expansion panel,
  floating button, menu panel (so the popup menu, the menu anchor and both dropdowns), the
  navigation drawer and rail, the search bar and view, the selection bar, the slider's thumb
  and the snack bar.
- **The shadow colour is the colour at full strength**, as the reference's `shadowColor` is. The
  table applies the strengths. The defaults that stood in for them (the scheme's shadow at
  30 %) are now the scheme's shadow. A colour an application names is used as named, and the
  three shadows take their share of it.
- **The drawer's shadow is no longer pushed sideways** onto the content. It is cast as any
  lifted surface's is, and the wide ambient shadow is what reaches the content beside it, as in
  the reference.
- Several widgets passed a false corner radius to their shadow (the blur itself). Each now
  passes its shape's own.

## The text selection bar

As the platform's own, which the reference matches:

- **A pill**: 44 pixels high, with ends that are half circles, whatever the theme's corners are.
- **White with black words** on the default light theme, and **`#424242` with white words** on
  the default dark one. A theme with its own surface gets its surface and its on-surface.
- **Just lifted**: a height of 1. It was 3, with the heavy shadow above.
- **More room at the ends than between the words**: 14.5 pixels before the first and after the
  last, and 9.5 either side of the others. It was 16 everywhere. A button is at least 48 wide,
  a target a finger can hit.
- Words at the regular weight.
- The highlight under a finger is square where a button meets another and round at the bar's
  ends, so it never spills past the pill.

## For the tests

A test that asked "how high is this?" used to decode the old formula from one shadow's blur. A
small shared probe (`shadowprobe`) now reads it back: the three blurs of a height are unique
to it, so the probe finds the height they came from. It also reads the colour a shadow was
named in, and how many surfaces cast one.

## Verification

- **Rendered**: going out from a box, its shadow gets lighter pixel by pixel down to the page's
  white at the rectangle's edge. There is no step, and at the box's edge the shadow is at half
  its strength. A surface one high casts a faint grey just under it and nothing ten pixels on.
- The table at a height of 3, exactly. Half-way between 4 and 6. Nothing past 24. A colour's
  own opacity scales all three shadows. A height of 0.5 is half as strong as 1.
- Three shadows painted for a height, grown by their blur and spread, with the corners grown
  with them. Nothing at 0 or in a transparent colour.
- The selection bar: a pill of the right colours on the light and the dark default, a height of
  1, its words black or white. A theme of its own colours it. The ends keep 14.5 and the middle
  9.5, and a one-letter word is still 48 wide.
- The widgets' own tests, now reading heights and colours through the probe: a card is as high
  as it is told, a hovered button rises, a drawer casts nothing until a colour is named.
- Twenty-two goldens re-recorded, each with a shadow in it: the square halos around the slider
  thumbs are gone, and the menus' shadows are soft and close. The search view's panel also
  casts from its own rounded corners now; it passed its blur as its radius.
- Mutations, each failing a test:
  - the old fade across the quad's edge;
  - the first shadow's strength;
  - no interpolation between heights;
  - no fade-in below a height of one;
  - a transparent colour still painting;
  - the bar at a height of three;
  - the bar in the scheme's surface on the default theme;
  - the same room at the ends as between;
  - no narrowest target;
  - the bar's corners taken from the theme.
- Not seen on a phone yet: no device was connected.
