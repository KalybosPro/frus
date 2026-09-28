# Milestone 593 — A measured box no longer relays out its page every frame

Step 2 of the plan milestone 587 proposed for #52 read: *the measure closure's layout goes
through the relayout cache*. Measured first, the cost turned out to be elsewhere, and bigger.

## What was measured

The frame's layout is cached per root. A root whose fingerprint (styles and structure) and
constraints are unchanged reuses its rectangles, and its measure closures are never called.
A probe counted the calls of a `ConstraintsTransformBox`'s measure closure and read the
cache's hits and misses, over three identical frames:

| tree | before | after |
|---|---|---|
| a list of 50 rows, each an `UnconstrainedBox` around a text | every frame: 10 roots recomputed, 10 measures | frames 2 and 3: 21 hits, 0 misses, 0 measures |
| an `UnconstrainedBox` around a column | every frame: 1 root recomputed, 1 measure | frames 2 and 3: 2 hits, 0 misses, 0 measures |

The closure was not the cost. **The whole root holding the box was recomputed every frame**,
with every node in it. A page with a single `UnconstrainedBox` anywhere was laid out again at
60 Hz, even with nothing moving.

## Why

The fingerprint marked a root holding a `ConstraintsTransformBox` as *volatile*, never to be
trusted between frames, for the reason a `LayoutBuilder` is: "its box comes from a closure,
and a closure cannot be hashed". That is true of a `LayoutBuilder`, whose closure is the
application's own. It is not true of a `ConstraintsTransformBox`. Its closure is the
framework's, and what it does depends only on data:

- the rule on each axis (as given, unbounded, or a fixed number);
- the alignment;
- a child that is part of the tree.

## What changed

- `ConstraintsTransform::layout_hash` feeds the rule on each axis and the alignment to the
  fingerprint.
- The fingerprint of a measured box is that, its own style, and **its child's fingerprint**,
  the way a `RotatedBox`'s already was. The entry can be trusted.
- A volatile child still makes its root volatile: an `UnconstrainedBox` around a
  `LayoutBuilder` is recomputed as before.
- `LayoutBuilder` is unchanged.

The cache inside the measure closure that step 2 described is not needed. When the root is
reused, the closure is never called. When the root changes, the closure runs once per question
taffy asks during that computation, and taffy already caches those per node.

## Verification

- A root holding an unconstrained box is reused on its second frame, with one hit and one miss.
  A longer label in the child is a miss, and the box comes back wider. A different rule
  (`Fixed(250)` in place of unbounded) is a miss.
- A root holding a `LayoutBuilder` is still recomputed every frame: two frames, two misses.
- Mutations, each failing one of those tests:
  - the rule left out of the fingerprint;
  - the child left out of it;
  - the root still marked volatile;
  - a fixed rule's number left out.
  In its first version the test changed the label and the rule in consecutive steps without
  isolating them, so a rule left out of the fingerprint still missed because of the label.
  Three of the four mutations survived that version. It now changes one thing at a time and
  checks the geometry that comes back (250, then 280 px).
- The probe above, before and after, on the same trees.
