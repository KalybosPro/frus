# Milestone 284 — The constraint boxes, and the two that could not be built

Four widgets whose whole job is to change the size their child is allowed, or made,
to be — plus the half of the constraint vocabulary the layout was missing.

## The missing half

`Style` had `min_width` and `min_height` and no ceilings at all. Floors without
ceilings is an odd vocabulary: it can say "never squash this" and cannot say "never
let this run away", which is the more common of the two. `max_width` / `max_height`
were added, mapped onto taffy's `max_size` and folded into the layout fingerprint, so
the relayout cache still invalidates when a ceiling moves.

## The three cheap ones

- **`SizedBox`** — a fixed box, or `expand` (take everything on offer), or `shrink`
  (take only what the child needs). Nothing a `Container` could not already do, but a
  `Container` with no colour, no border and no padding is a size dressed as a
  decoration, and reads as one.
- **`ConstrainedBox`** — floors and ceilings on either axis, with `tight` (floor =
  ceiling) and `loose` (ceilings only). Set nothing and it is transparent: it is the
  constraints that make it worth having, not the box.
- **`Intrinsic`** — a box sized to what its content would *like* to be rather than to
  what the space on offer suggests, on one axis, with an optional step to round up to.

`Intrinsic` is the only one of the three with a cost, and it is worth being explicit
about: the content is measured **once more, unconstrained**, before it is laid out for
real. That measurement already existed — `natural_size`, which `RotatedBox` uses — so
the widget is small, but nesting one intrinsic box inside another multiplies the
measurements. It is a widget to reach for deliberately, not a property every box
should carry.

## The one that needed the walk

**`OverflowBox`** lays its child out to constraints of *its own*, which the child may
exceed, and paints it over whatever is around it. A background that bleeds past its
slot, a decoration wider than the row it belongs to.

That cannot be done by a style, because a child sharing its parent's layout node can
never be bigger than it. So the box is a layout **leaf** and the child is laid out
**separately** — the same shape a scrollable's content, a list item and a `FittedBox`
already use — then anchored inside the box and rendered without the box's clip. A
spill that got clipped would be no spill at all; a `ClipRect` above is how a caller
asks for one.

Two details that only showed up in the running layout:

- The box takes **the largest size it is allowed** (`100%` on both axes), not the size
  of its content. Left to hug, it collapsed to nothing on the main axis — the child
  had nothing to be centred *in*, and spilled symmetrically from a point instead of
  from a 40 px slot.
- It is registered as a layout leaf in **both** caches — the paint cache and the
  relayout fingerprint. A leaf in the layout tree that still reports children would
  have made the walk's rect indexing and the cached rect count disagree, which is the
  quiet kind of wrong.

`OverflowBox::unconstrained` is the same mechanism with the constraints removed
entirely: the child is asked how big it wants to be and gets exactly that.

## The two that could not be built, and why

Two of the boxes on the list are missing, and neither is missing because it was hard.

**`LimitedBox`** applies a maximum **only when the incoming constraint is unbounded**.
It is not a maximum with a condition attached — it is a maximum that asks a question
frus's layout cannot answer. Constraint propagation belongs to taffy; a widget is
never told what it was offered, only what it ended up with. Shipping it as a plain
maximum would have been a widget that does something other than its name says, in
exactly the case it exists for.

**Baseline alignment** — aligning a row's children on the baseline of their text
rather than on their boxes — needs the text measurement to report a baseline, and the
measure hook returns a size and nothing else. Taffy has an `AlignItems::Baseline`, but
for a measured leaf it treats the bottom edge as the baseline, so switching it on would
have produced something indistinguishable from bottom alignment and called it
something else.

Both are recorded in the roadmap with what they need, rather than half-built.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **772 tests, 0
  failures** (760 at milestone 283): 12 for the boxes, each reading the resulting
  geometry off the painted scene rather than off the style that was asked for.
- `cargo build --workspace --all-targets` — OK, no new warning.

**Not device-verified.** No device was attached. These are layout primitives whose
whole observable behaviour is geometry, and the tests read that geometry from the
scene; a screenshot would say the same thing less precisely. The on-device checks owed
from milestones 282 and 283 are still owed.

## Also fixed

Two French assertion messages in `aspectratio.rs`, in a repo that is otherwise English
throughout.

## What's left

- `LimitedBox` and baseline alignment, as above.
- **No `FractionallySizedBox` merge.** `fractional.rs` already sizes a child by a
  fraction of its parent and now overlaps `SizedBox::width_fraction`; the two should
  probably be one widget.
- **`Intrinsic` measures the whole subtree.** For a column of labels it measures every
  label; a cache keyed on the subtree's layout fingerprint would make repeated frames
  free, and the fingerprint already exists.
