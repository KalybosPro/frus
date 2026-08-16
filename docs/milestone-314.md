# Milestone 314 — One control, not three buttons touching

`SegmentedControl` built itself out of `Button`s: the chosen segment a filled primary
button, the others outlined ones, two pixels apart, with the outer corners rounded by hand
to make the row look joined. It had one setting — the radius of those corners.

That is the same finding as `Tabs` in milestone 311, in the widget standing next to it: a
composite drawn as a row of something else. And it is visible, not theoretical. A segmented
control is **one** control — one outline around the group, hairlines where the segments
meet, and the chosen one filled — and three buttons with a two-pixel gap is three buttons
with a two-pixel gap.

## What the reference specifies

From `segmented_button.dart`: height **40**, a `StadiumBorder` around the group, a
`BorderSide` of `outline`, elevation **0**, `label_large` for the type, an icon size of
**18**, and `showSelectedIcon` **true** — the chosen segment carries a checkmark. The
selected colours are `secondary_container` on `on_secondary_container`; the unselected ones
have **no fill at all** and take `on_surface`.

The old control used the accent for the chosen segment, which is a filled button's colour.
The tonal container is what says *chosen among these* rather than *press me*.

## Two things that had to be worked out

**Who draws the outline.** If each segment draws its own, every joint is two hairlines
thick and the ends are heavier than the middle. So the control draws one stroked box with
the group's radius, plus a hairline at each division — and the segments draw only their
fill. The fill is then **inset by the border width**, because a fill painted edge to edge
would rub out the stretch of outline running along the chosen segment. There is a test for
exactly that, since it is invisible in a screenshot at anything under close inspection.

**Who decides how wide a segment is.** The reference gives every segment the width of the
widest, so that renaming one does not move the divisions between the others. A segment that
measured only its own label cannot do that, so each carries a shared handle to the whole
list — an `Rc` rather than a copy per segment — and measures the widest itself. It has to be
measured rather than counted: the widest label in characters is not the widest in pixels.

The checkmark's room is reserved in **every** segment, not only the chosen one, or the whole
control would change width as the selection moved.

## The stadium, again

The group's radius defaults to half the control's height, following milestone 313's rule
rather than repeating its number: a control told to be 32 px tall keeps stadium ends.

## What it dragged out of the previous milestone

The paginated data table put its "rows per page" picker in a footer beside a strip of page
buttons. With the checkmark reserved in every segment it no longer fitted, and the last
segment ran off the edge of the table.

Looking at why, the strip beside it was the same problem milestone 313 had already found and
only half fixed: **page numbers and arrows are one-glyph buttons**, and 313 gave a 40 px
circle to the four in the framework's own widgets while leaving the pagination strip, the
table's column-menu button and this picker as they were. That milestone's note says the
goldens it re-blessed "were checked to be the same change in the same widget" — and one of
them, this table, had a control running off its edge in the accepted picture. It did not.
Fixed here: the page buttons are circles, the column menu's three dots fit inside a header
cell, and the footer's picker turns the checkmark off, which is what
`show_selected_icon(false)` is for.

The lesson is about the checking, not the buttons: **twenty-two pictures is more than can be
checked by reading five of them and reasoning about the rest.**

## Verification

1046 tests (8 new), clippy silent, rustdoc clean. The tests pin what the widget claims:
one outline and not one per segment; two hairlines for three segments; the chosen segment
fills with the tonal container and takes a checkmark, which can be refused; the fill sits
inside the outline; every segment is as wide as the widest; a segment announces whether it
is the chosen one; and `caller ?? theme ?? framework` on the height.

Three goldens moved and all three were read: `navigation_pickers`, which holds the control
under the tab bar rewritten in milestone 311; and the two above, which are 313's oversight
rather than this milestone's change.

## Left

- **No multiple selection.** The reference's `multiSelectionEnabled` lets several segments
  be chosen at once; this takes one index.
- **No icon of your own.** A segment can carry the checkmark and nothing else; the
  reference takes an arbitrary icon per segment.
- **Nothing is disabled.** Neither the control nor a single segment can be greyed out —
  the third widget in three milestones to be missing `enabled`, which is starting to look
  like a gap in the framework rather than in any one widget.
