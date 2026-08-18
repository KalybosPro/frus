# Milestone 335 — A box that does not fit now says so

Three milestones — 327, 333, 334 — were one bug: a task row's delete button laid out past
the edge of its card, drawn nowhere, tappable nowhere. It took a device report, a hit-test
sweep, and two rounds of measurement to find, and the reason it took that much is written
in the roadmap in one line: **a row that overflows does so silently.** The reference calls
this an error condition, writes to the console and paints a striped band across the
offending edge. Here a child simply drew outside its parent and nothing anywhere said so.

## The detector

`frus_layout::Layout::overflows(root)` walks the computed tree and returns every box whose
children ran past it, with the edge and the amount. `Ui::overflows()` is the same thing for
a frame, screen-positioned.

Two details make it usable rather than noisy:

- **One taffy tree at a time.** A scrollable, a stack, a page view, a fitter and an
  overflow box are all laid out as leaves in the parent's tree, with their content computed
  separately. So the one overflow that is deliberate — content larger than the viewport
  that scrolls it — never reaches this walk at all. No exception list was needed.
- **Half a pixel of slack.** Fractional text measured against a fractional box produces
  overflows of a few hundredths on every screen. The reference has the same tolerance for
  the same reason.

Sub-roots need one extra step, which is worth naming because it is the sort of thing that
quietly produces nonsense: a sub-root's rectangles are in *its own* coordinates, and where
that pass lands on screen is only known when the walk reaches it. So the findings wait
under the sub-root's identity and are claimed, and translated, on arrival.

## What it found on its first run

Nine screens, at a phone's width and a desktop's. Sixteen of the eighteen were clean —
which is the result that made the rest of this milestone possible, because a detector that
fires everywhere is a detector nobody turns on.

**The chart dashboard, on a phone: 221 pixels.** Four segments of a segmented control,
584 px of them, in a 363 px row. The last one was drawn outside the card. Nothing had ever
said so.

## The reference caps a segment; we did not

`_calculateHorizontalChildSize`:

```dart
childWidth = constraints.minWidth / childCount;
while (child != null) {
  childWidth = math.max(childWidth, child.getMaxIntrinsicWidth(double.infinity));
  child = childAfter(child);
}
childWidth = math.min(childWidth, constraints.maxWidth / childCount);
```

Every segment the same width — which we had — **capped at an equal share of the room** —
which we had not. Ours gave each segment a fixed width taken from the widest label and let
the sum land where it fell.

The fix is milestone 334's two new fields doing exactly what they were added for. The
control keeps its natural width and gains `max_width: 100%`, so it stops at its parent's
edge. A segment stops carrying a width at all and becomes `flex_basis: 0`, `flex_grow: 1`,
`min_width: 0` — an equal share of whatever the control was granted. When there is room,
that share *is* the natural width, so the picture is unchanged; no golden moved. When there
is not, the segments divide what there is and the labels ellipsise.

## And the fix had a second half, which only a device showed

With the control capped, the picture on the phone was still wrong: the hairlines between
the segments no longer fell where the segments met. They were spaced by the *natural*
segment width, under a comment saying the two numbers "agree today, and would stop agreeing
the moment anything stretched the control". Capping is that moment. They are now the
control's own box divided by the number of segments — which is exactly how the segments
divide it, roomy or not.

Worth saying plainly: the layout test passed, the overflow survey passed, no golden moved,
and the control was still drawn wrong. The screenshot is what caught it.

## The report

The shell names each site on the console once — a layout that does not fit does not fit on
every frame, and sixty lines a second is the same as silence — with the two ways out the
reference suggests: give the child that should give way an `Expanded`, or put the content
in a `Scroll`.

## The instrument, kept

`no_screen_draws_outside_itself` surveys every route at both widths on every run. It is the
guard that makes this milestone worth more than the one bug it found.

## Left

- **The striped band is not painted.** The console half is here; the visible half, which is
  what makes the reference's version impossible to ignore, is not.
- **Settings on a phone overflows by 5 px**, pinned by the survey so it cannot grow. The
  settings card's margins make the tab panel 9 px wider than the content column it sits in,
  so it hangs out either side of a centring row. Measured, not fixed.
- **Nothing checks the goldens for overflow**, though the same call would do it — 83
  pictures that have never been asked the question.
