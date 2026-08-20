# Milestone 361 — The edge an axis starts at, and a box that confines

Two loose ends, both small, both closed by naming a thing rather than adding one.

## The glow at the wrong end

Milestones 359 and 360 left the same note twice: a reversed area still glowed at the
**top** when it refused to go further back, and a pull-to-refresh above one still listened
at the top. Both read the sign of a refused delta in *offset* space and assumed offset zero
was the top of the screen, which reversing is precisely the act of denying.

`Scrollable::refused_edge` is that assumption made explicit and then corrected in one
place, with `Scrollable::start_edge` on top of it for the widget that wants the edge an
axis *begins* at rather than the one a particular refusal happened at. Two call sites, and
the pull-to-refresh test now reads "the edge this axis starts at" instead of `GlowEdge::Top`
— which is what it always meant.

## `Container::clip`

The reference's `Container` takes a `clipBehavior`, and defaults it to `Clip.none`: a
decoration is painted *behind* a child and does not confine it, so a box with rounded
corners leaves a photograph inside it square. Ours had no way to ask at all — the only
route was to wrap the child in a `ClipRRect`, which is also the reference's common idiom
but is not the same thing as asking the box that already has the corners.

`Container::clip()` returns the container's own radius as a `ClipShape`, which the walk
already knows how to turn into a clipped compositing layer for `ClipRRect`. Off by
default, as there: it costs a layer, which is why neither framework does it unasked.

## Left on `Container`

**`foregroundDecoration`** — a decoration painted *over* the child rather than behind it,
for an overlay border or a wash across a tile. There is no workaround here short of a
`Stack`, and no hook for it either: nothing in the widget trait paints after its children.
That is a walk change rather than a widget one, and it wants its own step.
