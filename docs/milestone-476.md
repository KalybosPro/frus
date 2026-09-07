# Milestone 476 — The last two widgets that could not be themed

Closes [#51](https://github.com/KalybosPro/frus/issues/51).

The standing rule is that every widget's styling must be overridable: themed defaults are
fine, hardcoded-only never is. `WidgetThemes` is how that is kept, and #51 listed about
twenty widgets with no entry at all.

## Most of the list had already been closed

The issue was written when the registry held 21 entries. It holds 51 now, and sixteen of
the twenty it names — dialogs, list tiles, snack bars, tooltips, bottom sheets, banners,
both pickers, menus, the rail, the drawer, the progress indicators, both search widgets,
data tables, expansion tiles and the floating action button — acquired one in the
milestones since. Checking that first was most of the work, and it is the reason this
milestone closes an issue that reads like twenty.

Four gaps were left. **Two of them were not gaps.**

- **`CarouselView` needs no entry.** Its `paint` is empty — it draws nothing at all. It is
  two `Button`s around a slide the application supplies, and those are themed by
  `ButtonTheme` already. An entry here would be a struct nothing reads.
- **The text-selection colour was already overridable**, as `Theme::selection`, which
  `TextField` reads when it paints a selection. It is on the theme rather than in
  `WidgetThemes` because it is not one widget's property — anything that ever grows
  selectable text will want the same colour.

Writing that down is the point. An empty `CarouselTheme` would have satisfied the issue's
letter and made the framework worse: a reader who found it would reasonably expect setting
it to do something.

## `NavBarTheme`

Height, padding, background, title style, hairline colour and hairline thickness — the six
things the bar actually reads. The two that are not colours are the ones worth having: a
bar themed in its colours and sized from a constant is themed in the half that shows and
unthemed in the half that decides where everything under it starts.

`height` had to become an `Option` inside the widget. It was an `f32` initialised to the
default, which cannot tell *the caller said 56* from *nobody said anything* — and without
that distinction a theme either never wins or always does. The chain is
`caller ?? theme ?? framework` and a test holds all three terms, including the last: a bar
with nothing said at its call site takes the theme's height, or the first half of that test
would pass on a widget that ignores the theme entirely.

**There is no field for the back button's size.** The bar names one explicitly when it
builds the button, and an explicit argument outranks a theme — so the field would be one
the widget never reads, which is worse than an absent one.

## `ScrollbarTheme`

Thickness, margin, minimum thumb length, radius, the thumb's colour, and its three
opacities — at rest, warmed under a pointer, held. Every number `add_scrollbar` was built
from.

There is no per-call override to outrank: nobody configures a scrollbar at a call site, the
walk draws it. So the chain is two terms, and the test's second half is that the framework
still answers where the theme says nothing.

**The thumb is a fade of one colour, not three colours.** A theme substitutes the colour
being faded and may move the three levels; it does not hand over three finished colours,
because warming from one to another means interpolating colours, and interpolating in the
wrong space is this repository's recurring bug. Fading one colour is the arithmetic that
was already right, and the theme does not get to change which arithmetic runs.

**And no field for a track**, because the framework paints none — the reference's is
transparent unless a caller asks. A field that painted nothing whatever it was set to would
be worse than no field.

## What the tests hold

Two new tests per entry: every field reaches the painted output, and what the caller said
outranks what the theme says. Verified by removing the three theme lookups in the bar and
watching exactly those two tests fail, and nothing else in 1 906.

The colours are asserted on the scene rather than on a rendered pixel, and that is a
decision rather than a shortcut: neither widget does any blending arithmetic of its own —
each hands an opaque colour to a fill — and *a colour asked for is the colour painted* is
pinned once for the quad path in `painted_colours`, not once per widget. Where this
milestone touches colour arithmetic, it deliberately did not change it.

No golden moved. Every field defaults to `None`, so an application that says nothing gets
exactly what it got yesterday.
