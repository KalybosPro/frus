# Milestone 286 — The shared element, and the frame that already knows both answers

A `Hero` says one thing and nothing else: *this is the same thing as that*. Two heroes
carrying the same tag on the two sides of a route transition are understood to be one
element in two places, and the transition flies it from where it was to where it is
going instead of fading one copy out while another fades in.

## Why this was tagged "design first", and why it turned out not to be

The roadmap called this one **🔴 design first**: "two trees, one flight, and identity
across a rebuild". Identity is genuinely the hard half — nothing in a widget tree says
that a thumbnail and a full-size picture are the same picture, so a `Hero` has to say
it, and a tag is the smallest thing that can.

The other half, though — *two trees* — turned out to be a problem the framework had
already solved. During a route transition the navigator holds **both screens in the
same frame** and walks both of them. So both boxes are known at the same moment, and
nothing has to be remembered from the previous frame, diffed, or reconciled across a
rebuild. The flight is resolved right after the two screens are drawn, inside the
navigator's own branch of the walk, where the transition's progress is already in hand.

That is the whole design. It fits in one method.

## What flies, and what is taken away

The travelling copy is the **destination**'s own painting, lifted out of the frame by
owner and mapped onto the box it is passing through — never a third widget built for
the occasion, which would be a second definition of the same thing and free to drift
from it. `Draggable`'s ghost and the reorder preview already worked this way; this is
the third user of the same idea.

Both originals are taken out of the frame for as long as the flight lasts. A thing that
is flying is not also sitting at either end, and leaving them in is the difference
between a shared element and three copies of one.

A tag with no counterpart on the other side is left alone — the ordinary transition,
which is the right answer. So is a tag used **twice** on one side: which of the two is
*the* one is not a question the framework can answer, and guessing would be worse than
not flying.

## The registry that must ignore visibility

Every other registry the walk fills — hits, focusables, drop targets — is about what a
pointer can reach, so each is behind a "is any of this on screen?" guard.

Heroes must not be. Half way through a transition the screen being left has usually
slid off the edge, taking its hero with it, and **that off-screen box is exactly what
the flight starts from**. The first run of this showed it plainly: only the incoming
hero was ever recorded, so no pair ever matched and nothing flew. Recording heroes
above the guard fixed it.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **785 tests, 0
  failures**, 4 of them for the flight: that a matched pair paints as *one* box and not
  two, that it starts at the source's size and ends at the destination's, and that an
  unmatched tag changes nothing.
- `cargo build --workspace --all-targets` — OK, no new warning.

**On a physical device** (Huawei, Android 10): tapping a task's avatar opens its own
screen, and a single avatar is caught mid-transition, travelling and growing between
the row it left and the place it is going — with neither the small one nor the big one
drawn beside it.

## Found, not fixed: a wrapping text reports one line

Building the demo's task screen turned up a defect that is **not** part of this
milestone and is not fixed here: a `Text` with `.wrap()` that is **shrunk to fit** — as
it is when centred on a column's cross axis — is laid out at the narrow width and wraps
onto two lines, but reports the height of **one**. Whatever follows it overlaps it.

Two call-site workarounds were tried on the device and neither helped, which is itself
evidence that the height is settled before the width is: giving the text a definite
width did not change the result. The demo's screen now simply does not wrap, so it does
not show the bug off, and the bug is recorded in the roadmap where it belongs — as a
text/layout issue, not a hero one.

## Also fixed

One French `expect` message in `frus-layout`, in a repo that is otherwise English
throughout.

## What's left

- **The path is a straight line.** Platforms curve it, so that a diagonal flight reads
  as one motion rather than two. That is a refinement on top of `lerp_rect`, not a
  different mechanism.
- **No cross-fade between the two contents.** The destination's painting flies the
  whole way, so a source and destination that look very different swap abruptly at the
  start rather than dissolving.
- **Only route transitions.** A shared element that moves *within* one screen — a list
  item expanding in place — would need the same treatment driven by something other
  than the navigator.
- **A tag used twice on one side is ignored**, deliberately. A rule for choosing (the
  visible one? the first?) would be a guess dressed as a decision.
