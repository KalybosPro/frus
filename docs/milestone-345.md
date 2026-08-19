# Milestone 345 — The band, and the four things it found

Milestone 335 taught the layout to notice when a box's children run past it, and the shell
to say so in the console. It left one thing undone and wrote it down: **the reference
paints a striped band across the offending edge, and here nothing appeared on the screen.**

The console half is what a developer reads. The band is what a *screenshot* shows — and a
screenshot is what a bug report is made of. Three of this project's own milestones went
hunting for defects a band would have pointed straight at: a delete button laid out past
its card (333, 334), a segmented control 221 pixels outside its parent (335).

## The look is the reference's on purpose

Black and yellow diagonal stripes, three quarters opaque, over a tenth of the box along
the edge the children ran past. Those are the reference's colours (`0xBF000000`,
`0xBFFFFF00`) and its `_indicatorFraction`, because the entire point of the thing is to be
recognised on sight by somebody who has never read this framework's documentation.

The *mechanism* is not the reference's: it paints one repeating diagonal gradient, and this
paints the stripes as parallelograms clipped to the band. A repeating diagonal gradient is
a shader feature; a parallelogram is four points.

**Debug builds only.** A band is a message to whoever is building the application, and in a
release build there is nobody there to read it. The reference draws the same line.

## What it found, within a minute of being switched on

### Four goldens wearing a band for a pixel that did not exist

The time and date pickers came up striped, and the overflow behind it was **one pixel**.

taffy rounds each node's edges to whole pixels, independently. A section 169.6 tall, whose
grid sits at 21.6 and is 148 tall, becomes a box of 169 with a child at 22 — and the child
now ends one pixel below its parent. Nothing is laid out wrong; the arithmetic simply
happened twice at different precisions.

`Layout::overflows` was asking the question of the **rounded** layout, which is the one
layout that cannot answer it. It reads the unrounded one now, and the four bands went out
with the rounding. The half-pixel tolerance that was there stays, for genuine sub-pixel
slack.

This is the more valuable of the two findings, because it was making the *existing* console
reports untrustworthy: an overflow survey that cries wolf at rounding is one nobody reads.

### A test fixture that had been overflowing since it was written

`a_faded_group`, the golden for group opacity, put a 90×60 box with 20 px of padding —
130×100 — inside a 120×80 stack. The band landed across the very overlap the picture exists
to show. The offset is 10 px now, and the picture is the one it always meant to be.

### `Tabs` sized itself by whatever was on the busiest tab

Not found by the band — found by chasing the roadmap's account of the settings screen's
5-pixel overflow, which named `Tabs` as the cause. A tab set takes the width it is offered
now, as the reference's does. Sized by its content instead, the widest thing on any tab
decides how wide the whole control is, so the bar jumps from tab to tab and a panel that
does not fit hangs out of whatever is centring it rather than being told to fit.

It is `main_axis_fill` again — milestone 342's hook, whose axis is a *question*, not the
widget's own main axis: a tab set is a column asking for the horizontal one.

### The settings screen's overflow was never `Tabs`

Filling the tab set changed the overflow by nothing at all. Measured unrounded it is 4.5 px,
not 5, and the cause is that something in the Controls tab will not go below about 380 px.
The roadmap said otherwise for ten milestones. It says this now, and the pin came down from
5.5 to 4.6.

## Left

- **No label.** The reference writes `RIGHT OVERFLOWED BY 5.0 PIXELS` across the band,
  rotated for the vertical edges. The console says the same thing here, which is enough
  until the band is being read on a device — where the console is not.
- **A band per overflowing box.** Ten nested boxes overflowing together paint ten bands,
  where the reference paints one per render object too. It is loud, and it is loud in
  proportion to how wrong the layout is.
