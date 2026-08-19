# Milestone 340 — The backdrop, and the layer that was never drawn

The last two of the twenty-two widgets milestone 336 counted: `BackdropFilter` and
`BackdropGroup`. Building them found a bug three hundred milestones old, and that turned
out to be the larger half of the work.

## What a backdrop asks the renderer to do

The other three filters act on a subtree, and a subtree is already a texture — the layer
it is composited through. A backdrop acts on **the frame so far**, which is not a texture
at all. The renderer has to stop half-way, read what it has drawn, filter that, and carry
on.

It cannot read it out of the target: a surface texture is a render attachment and nothing
else. So a frame that contains a backdrop is built in a **staging texture** we own, cut
into segments at each backdrop, and blitted onto the target at the end. A frame with no
backdrop — which is nearly every frame — takes the path it always took, one pass straight
into the target, unchanged.

Each segment ends where a backdrop begins. Between segments the encoder is **finished and
submitted**, because the filter that comes next reads the stage and a command buffer still
being recorded has not written anything yet. That is one of the two things that had to be
right; the other is that the backdrop draw comes *before* its own layer's content, which
is what makes the frosting sit under the bar rather than over it.

## `BackdropGroup`, and what sharing costs

A backdrop is expensive in a way the other filters are not. Sixty frosted rows in a list
is sixty stops, sixty copies, sixty blurs.

`BackdropGroup` makes it one. Backdrops carrying the same key are filtered once: the copy
is taken before the first of them and every other one reads the same texture. The result
is visually identical, because all sixty were filtering the same picture anyway.

The catch follows directly from the mechanism, and the reference states it too: two
grouped backdrops that **overlap** will look wrong, because the copy they share predates
both. The test for the sharing is exactly that — two backdrops over the same region,
shared and unshared, and the unshared pair blurs twice. It is the only way to observe from
the outside that the sharing happened at all.

The group needs no key of its own. Its identity in the widget tree is the key: stable
across frames, unique without anything having to hand one out, and scoped by the walk the
same way the focus flags are.

## `FilterContext`

The filter hook took the widget's box, so a mask could be written in fractions of it. It
now takes a small context carrying that box *and* the enclosing backdrop group, for the
same reason: both are properties of **where the widget turned out to be**, not of what it
was built with. A widget cannot see its own ancestors; the walk can.

## The layer that was never drawn

`ClipRect` around a `BackdropFilter` is the shape the reference tells callers to reach
for — it is how a backdrop is given its bounds. Written here, it rendered **nothing**.

The reason was not new. A group is rendered into a texture of its own and that texture is
composited; a layer found *inside* that group is not a primitive the group can paint, so
it was skipped. The comment saying so had been in `render_group` for a long time, filed as
an accepted limitation. It was not one. It meant a rounded card around a fading group, a
clip around a transform, a shape around anything composited — silently gone.

A group now renders its nested layers first, depth-first, each into a texture of its own,
and composites them into its own pass. The recursion is safe to interleave with the shared
instance buffers because every level prepares, records and **submits** before returning:
the deepest group is on the queue before its parent writes a single instance.

Two goldens moved, and every pixel that changed got **brighter** — 24 on the edges of an
opacity group, 247 on the edges of a rotated card. That is the milestone-339 coverage fix
finally reaching the antialiased edges of layers those tests had been dimming all along.
Nothing structural moved in either picture.

## And one fold, which is not the same fix

Nested layers rendering correctly still would not have made `ClipRect(BackdropFilter(…))`
right: a backdrop pushed one level down would be filtering the clip's own contents rather
than the frame. So a clip around a filter is **folded into one layer**, the same fold
milestone 339 introduced for two filters, and the merge rule was sharpened to allow it: a
backdrop refuses to share a layer with a filter *of the layer itself*, but shares happily
with nothing at all, and nothing at all is exactly what a clip is.

## Left

- **`Baseline` / `IgnoreBaseline`** — taffy has baseline alignment; nothing reaches for
  it. That is the last of the twenty-two.
- **A backdrop inside another layer filters that layer, not the frame.** An opacity group
  or a transform around a backdrop puts it in a group, and a group has no frame-so-far to
  read. The reference documents the same caveat, for the same reason. The clip case — the
  one that matters — is folded and works.
- **Nested layers are not cached.** The layer cache is keyed by rank in the top-level
  scene; a nested group is re-rendered every frame its parent is. That is strictly better
  than not being drawn, and it is the obvious next thing to fix.
