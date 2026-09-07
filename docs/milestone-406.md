# Milestone 406 — Forty-seven ways to ignore the reader

Milestone 403 gave the framework one place where a reader's font setting is applied:
`TextStyle::resolved()`. A probe written to check the *heights* printed something else
entirely.

```
Kbd        x1  tallest glyph 13.0
Kbd        x2  tallest glyph 13.0
Toast      x1  tallest glyph 16.0
Toast      x2  tallest glyph 16.0
Dropdown   x1  tallest glyph 18.0
Dropdown   x2  tallest glyph 18.0
```

A reader who turns the system type up gets nothing. Not a warning, not a partial effect —
the widget simply does not answer.

## The door

```rust
pub fn text(&mut self, position: Point, text: impl Into<String>, size: f32, color: Color)
```

A bare `f32`. Forty-seven call sites across twenty-three widgets walked through it, naming
a constant and handing it straight to the shaper. `resolved()` was never in the path, so
the setting could not reach them, and no test could tell: measurement went through the
matching raw door (`frus_text::measure`), so paint and layout agreed with each other
perfectly — on the wrong number.

`Scene::text_styled` was right next to it, taking a `ResolvedTextStyle`. Two doors, one
correct, both spelled `text`.

**The fix is to delete the wrong one.** `Scene::text` now takes the resolved style, and
`text_styled` is gone — one method, one spelling. Every raw site became a compile error,
which is what turned this from an audit into a work-list: the compiler found all
forty-seven and would not let one through.

## What each site had to say

The type does not answer the question, it only forces it. There are two right answers and
the reference gives both:

| | what it does | who does it |
|---|---|---|
| `TextStyle::new(SIZE).resolved()` | follows the reader | anything a reader **reads** |
| `ResolvedTextStyle::exact(SIZE)` | keeps its size | a glyph that is an **icon** |

`exact` is not an escape hatch, it is the correct answer for a checkbox's tick, a tree's
chevron, a star in a rating, the figure inside a step marker, an avatar's initials at two
fifths of a circle it does not control. Each of those lives in a box that does not move,
and type that grew would leave it. They are named `exact` at the call site with the reason
beside them, where before they were an anonymous `13.0`.

## Two answers to the same reader, and both are the reference's

**A component grows.** Its default height is a **floor**, not a ceiling —
`math.max(_targetTileHeight, contentHeight)` in the reference's list tile,
`math.max(_kChipHeight - padding, labelHeight)` in its chip. Our `ListTile` already did
this (`min_height`) and our `Button` already did it (`.max(measured.height)`). Our `Chip`
did not: its height was `Length(32)` flat, so at a scale of 2 its glyphs needed 34 px in a
32 px box and were cut. `frus_text::line_box(min, style, padding)` is that rule written
once, and it is what the row-shaped widgets now use.

**Chrome caps the type instead.** A toolbar cannot grow — it would push every screen down —
so the reference keeps `kToolbarHeight` and clamps the title's scaler to `1.34`, "to keep
the visual hierarchy the same even with larger font sizes". `TextStyle::clamp_scale` says
that, and `AppBar` uses it. Below the cap the title follows like anything else; above it,
it stops.

Having both is the point. A framework with only the first pushes an app bar off the screen;
one with only the second is deaf.

## Three live bugs the sweep turned up on the way

- **`AlertDialog` measured and painted at different sizes.** Its body was painted through
  `TextStyle::new(TEXT_SIZE).resolved()` and measured with `measure_wrapped(TEXT_SIZE)`. At
  any scale but 1.0 the box and the words disagreed. Milestone 403 introduced this the day
  it made `resolved()` scale.
- **`TextField` did it too, and worse**: multi-line went through `resolved()`, single-line
  did not. The caret is placed from `layout()`, which used the raw size — so at a scale
  above 1 the cursor would have landed where the glyphs were not, in the one widget where
  that is unforgivable. Everything in the field now goes through one `text_style()`.
- **The layout cache was blind to the setting.** `signature_of` hashed styles, structure and
  measure keys, but not the scale. A reader who changed the system font would have got the
  previous frame's geometry with the new frame's glyphs. It is one line at the root now.

## The test, and the table it nearly missed

`the_text_a_widget_paints_follows_the_readers_font_size` builds ten widgets at scale 1 and
scale 2 and asserts something grew. `a_box_that_holds_text_grows_with_it` asserts no line
is taller than the box holding it. `an_app_bar_caps_its_title_rather_than_growing` asserts
the other answer: proportional at 1.2, pinned at 1.5, 2.0 and 4.0.

The second was written comparing each line against the widget's **outermost** rect, and
that is too loose to be worth much. A `Table`'s outer box is hundreds of pixels tall and
would have accepted any row at all — and the table was in fact broken: a cell **paints**
its label rather than laying it out, so as far as the layout is concerned its content is
empty, and `min_height: ROW_H` was a ceiling in everything but name. At a scale of 2 a
15 px label wants 36 px of line inside 34. The comment above it said "grows with the
content (nothing is clipped)".

It now compares against `Primitive::Text`'s `bounds` — the box the text was actually laid
out in. That is what caught the table, and it is the version worth keeping.

The first test is still the one that matters most, because `exact` remains reachable and is
right often enough that it will be reached for again — and it is exactly as silent as the
raw `f32` was.

## Seven goldens moved, and why

All seven are date pickers, and the whole difference is a one-pixel rightward shift of the
weekday initials — `S M T W T F S`. `WeekdayCell` declares no width, so its box is whatever
column the grid gives it, and it was centring its letter against `CELL`: the constant
belonging to a **different** widget. That is this milestone's own bug in miniature, a number
that is not the number. Reading the band pixel by pixel shows the glyphs intact and nothing
clipped, and the shift is in the correct direction.

It is invisible at a scale of 1 and gets steadily worse above it, because `cell()` grows
with the reader and the grid's columns grow with it while a frozen `CELL` does not. Which is
the entire point.

## Left

- **A chart's marks do not follow the reader.** Axis ticks, value labels, the legend and the
  tooltip are `exact`, deliberately: they are annotations pinned to a plot area whose height
  the caller fixed, and type that grew would run out of the plot rather than move it.
  Making the plot answer to the reader is a chart-layout change, and it is recorded here
  rather than smuggled in.
- **The scale is still linear.** The reference's `TextScaler` is a *function*, and large
  sizes grow less than small ones. Ours multiplies. Every size in the framework now passes
  through the one place that would have to change.
- **No platform reports the setting yet.** Android has it in `Configuration.fontScale`;
  nothing carries it to `MediaQuery`. Until that wire exists this is only correct in tests
  and in an application that sets the scaler itself — the same missing wire as milestone
  399's.
- **`line_box` is not applied everywhere a constant meets a line of text.** The row-shaped
  widgets, the chip, the date cell and the table row have it. A `SnackBar`'s body and the
  navigation rail's item do their own arithmetic and were checked to grow — the rail's item
  reaching 65 px where its floor is 58 — but only ten widgets are held to it by a test, and
  only the ones somebody remembered to add.
