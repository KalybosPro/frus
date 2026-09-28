# Milestone 594 — AnimatedSize

`AnimatedSize` is one of the two implicit animations issue #30 left open, both waiting on #52.
Milestones 592 and 593 were the first two steps of milestone 587's plan. This is the first
widget built on them.

## What it does

`AnimatedSize::new(duration, child)` is a box that **follows its child's size over time**. When
the child grows or shrinks, the box moves to the new size over `duration` seconds instead of
jumping there. The child is cut to the box on the way, and everything around the box moves with
it.

- **Child constraints.** By default the child is given the width on offer and is asked for its
  height, which is what a section that expands and collapses wants. `width` and `height` change
  that in a `ConstraintsTransformBox`'s vocabulary. On an axis the child is given as it came,
  the box asks for the room on offer (milestone 590's `fill_axes`).
- **Options.** `curve` sets the easing (linear by default), `alignment` places the child while
  it does not fit (centred by default), and `clip` cuts it to the box (on by default). These are
  the reference's defaults.
- **Reduced motion** ends every move at once.

In the demo, the task list grows when a task is added and shrinks when one is deleted or
filtered out.

## How it works

The reference's `RenderAnimatedSize` lays its child out, compares its size with the size it
was going to, and restarts a controller from where it is if they differ. Here the same thing
is split between the layout, the runtime and the walk:

- **The layout measures.** The box is a measured leaf, like a `ConstraintsTransformBox`. Its
  closure lays the child out, tells the runtime the child's size, and answers with the size the
  box has got to, **rounded up to whole pixels** (milestone 289's rule for measurers: a
  fraction rounded down is a box the content no longer fits in).
- **The runtime keeps each box's move:** from, to, elapsed, curve. It sits behind a `RefCell`,
  as the layout cache does, because the layout holds only a shared reference. A box seen for the
  first time takes its child's size at once. A child measured at a new size starts the move
  again from wherever the box had got to. `Runtime::advance` moves every box on, so the shell's
  frames and the test harness's both do. A box no frame has seen is forgotten.
- **The fingerprint** of a moving box includes where it is and where it is going. The layout
  cache relays it out at every step and reuses it once it has arrived (milestone 593).
- **The walk** lays the child out as a `ConstraintsTransformBox`'s, and cuts it to the box.
  On an axis the child decides, it comes back at its own size, which is where the box is
  going. An axis the child is given is not animated at all, because the box fills the room
  there. So the child is never laid out at a size it is only passing through, and a paragraph
  does not rewrap at every frame.

  A first version laid the child out at its recorded target explicitly. A mutation removing
  that branch survived every test, and the reasoning above is why: the branch could not change
  anything. It was removed.

The layout asks intrinsic questions along the way. On an axis the child is given, it offers
nothing ("how big with no limit") or zero ("how small could it be"). Those answers are given
but not recorded, or the box would chase a size the child never ends up at: a paragraph with no
limit is one line, and at zero it is one word a line. The first version recognised only the
zero. A test of a paragraph in a row showed the box starting a move on its very first frame,
from the one-line answer, and the rule was completed.

## Differences from the reference

- **One frame late.** A child's new size is seen during the layout of the frame that changed
  it, and the box starts moving on the next frame. The reference starts on the same frame.
  Milestone 587 foresaw this and judged one frame at 60 Hz acceptable.
- **No "unstable" state.** The reference stops animating and tracks a child whose size changes
  on consecutive frames. Here every change restarts the move from where the box is.

## Found on the way

- **The demo's test harness never advanced the runtime.** A list growing after a test added its
  tasks stayed at its first size, and the rows it cut away could not be pressed. The harness now
  lets a second of frames pass after adding tasks, the way the reference's tester pumps and
  settles. It is a second rather than "until nothing moves" because the home screen always has
  something moving.
- **A measured leaf never asked for the room.** A `ConstraintsTransformBox`'s branch of the
  layout returned no fill request, so a box meant to take the width on offer was as wide as its
  content inside a plain container. The branch now passes the widget's own request, which is
  nothing for a plain transform box.

## Verification

- The move itself:
  - a new box is its child's size;
  - a grown child is followed, half-way at half the time;
  - a change mid-move starts from where the box is;
  - an intrinsic question is not a change;
  - reduced motion arrives at once;
  - a box that has gone is forgotten.
- In a page: one row, then three. The box stays one row tall on the frame the change is seen,
  is 80 px half-way, and 120 px at the end, and what is below follows it at every step.
- Half-way, two rows are painted and the third is cut at the box's 80 px.
- A container widened from 200 to 300 px: the bar inside, which fills the width it is given,
  is 300 px on the next frame. The width is the room's, not the child's, so it is not followed,
  as the reference's box under tight constraints is not.
- In a row, the layout asks how narrow the box could be before giving it a width. The box does
  not take that answer for a change: it is at rest from the first frame, as tall as the
  paragraph at the width it gets.
- The demo's 79 tests.
- **On an Android phone.** Adding a task to the demo's list, a capture taken right after the
  press shows the new row cut at the bottom of the growing box and the footer on its way down.
  A second later the row is whole and the footer is in place.
- Mutations, each failing a test:
  - a restart from the child's new size rather than from where the box is;
  - the size not rounded up to whole pixels, caught by the demo's tests;
  - no clip;
  - the move left out of the fingerprint;
  - the runtime not advancing the moves;
  - the measured branch dropping the fill request;
  - `AnimatedSize` not asking for the width;
  - the intrinsic questions recorded, and the first version's zero-only rule.
