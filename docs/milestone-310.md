# Milestone 310 — A theme for one subtree

Milestone 309 gave the theme per-widget defaults: an application can say *every card is
flat* once. It could still only say it once, for **everything**. There was one theme per
frame, so a dark panel on a light page, or a section whose cards are flat while the rest of
the application's are not, had to be written out property by property at every call site
inside it — the framework failing at exactly the point a theme was supposed to help.

The reference has had the answer since the beginning: a `Theme` widget that replaces the
ambient one for its subtree, read from the *context* rather than passed as an argument. That
is what this milestone is.

```rust
Themed::new(Theme::dark(), sidebar())                              // this part is dark

Themed::tweak(|t| t.widgets.card.elevation = Some(0.0), settings)  // and this part is flat
```

`tweak` is the form that gets used, and it takes a **closure** rather than a value because
the ambient theme is not known when the tree is built. The walk resolves it on the way
down — which is the same reason the reference reads its theme from the context.

## What had to be true for this to work

**It reaches layout, not only paint.** Milestone 309's whole point was that a divider's
height and a card's margin are theme settings, and they are resolved during layout. So the
swap happens in `build_layout` *and* in `hash_node`, the relayout cache's fingerprint of
that same walk. The two making the same swap is what keeps the cache honest: a fingerprint
taken under one theme and a geometry computed under another is a stale layout that appears
only on the second frame.

**It is put back on the way out.** The paint walk is split in two — `walk_node` swaps the
theme and calls `walk_node_themed`, which is the walk proper. Not for elegance: the walk has
a dozen early returns for barriers, opacity groups, transforms and clips, and a restore
written at the end of that function would be skipped by every one of them. The split makes
the restore unskippable. Without it a theme would leak sideways into whatever the subtree's
parent painted next, which reads as "the theme applies from wherever I set it, downwards" —
a bug that is obvious once seen and invisible until then.

**An overlay carries the theme it was declared under.** A dialog, a drawer or a tooltip is
painted long after the walk has left the node that declared it, so it would otherwise come
out in the root's theme — and only when it opened, which is the worst possible moment to
find out. The theme now travels in the deferred-overlay record. The reference had to grow a
mechanism of its own for the same reason, which is a good sign the problem is real and not
an artefact of this design.

**The direction comes from the theme too**, so `rtl` stopped being a field set once at the
root. It is honest about where it stops: it decides which edge a drawer slides from, which
way an anchored overlay opens, the sign of a rotation, and the mirroring of any layout root
inside the subtree — but **not** ordinary flow, because the frame's rects are computed once
at the root and mirrored there as a whole. Flipping an application is still done on the
theme handed to `build_ui`. That limitation is written on the accessor rather than left for
someone to discover.

## The macro, and what it found

`Themed` is a **transparent wrapper**: it is its child, adding one thing. `Keyed` already
was one, and its `Widget` implementation was ninety hand-written forwarding methods. Writing
a second copy of that by hand was not an option — this repository has already lost an
afternoon to a wrapper that forwarded *most* of the structure, and found it on a device.

So the forwarding became a macro, and the macro leaves exactly two hooks to its caller:
`key` and `theme_override`, the two a wrapper can have a reason to claim. Each wrapper
states both — one claimed, one visibly forwarded — so the one it is not for cannot be
silently defaulted.

Then a test compares the macro's method list against the `Widget` trait's own, read out of
`widget.rs`, and it failed on the first run:

```
a transparent wrapper would answer these for itself: ["reorder_axis", "reorder_draggable"]
```

Both were missing from `Keyed` and had been all along. A keyed card in a board dragged along
the **horizontal** axis instead of the vertical one, and a drop-only slot could be lifted.
Nothing was failing; nobody had wrapped one in a `Keyed` yet. **The list a wrapper forwards
is not a thing to keep by hand** — and a test that reads the source is a fair way to say so,
because the next hook added to the trait will fail it too.

A second, smaller thing fell out of the same reasoning. Two nested `Themed`s are **one** node
in the tree, because a transparent wrapper reports its child's children: the inner one would
never be asked for its theme at all. So `Themed::theme_override` applies its own theme and
then asks its child, outer first. Nesting composes because it was made to, not because it
happened to.

## The cost

`Theme` is 1.5 KB, and `theme_override` is asked of every node on every frame. Returning
`Option<Theme>` would put that on the stack of a recursion as deep as the tree, so it returns
`Option<Box<Theme>>`: a word on the stack, and an allocation only where a subtree actually
claims a theme. Clippy noticed the same thing about the enum inside `Themed`, from the other
end.

## Verification

1021 tests (11 new), `clippy` silent on every target, `rustdoc` clean under `--all-features`,
and **all 77 goldens unchanged** — the expected result: nothing in the tree uses a subtree
theme yet, so every pixel must be where it was.

The seven new tests are one per way this could be wrong: the theme reaches paint; it reaches
layout; it is restored for the sibling that follows; nesting starts from what it inherits;
`new` replaces where `tweak` merges; an overlay keeps the theme it was declared under; and
the wrapper adds no box of its own.

## Left

- **Direction is not per-subtree.** A `Themed` that flips the direction moves what the walk
  decides, not the flow around it — mirroring a subtree's rows would mean mirroring a
  sub-range of the frame's rects about the subtree's own box, and the walk only learns that
  range by walking it.
- **A theme cannot animate.** The reference lerps between two themes over 200 ms; here a
  swap is instant. `Color::lerp` exists, so this is a `Theme::lerp` and a wrapper away.
- **The rest of the widgets.** Still four wired to `Theme.widgets`. `Button`, `Chip`,
  `Tabs`, `TextInput` and `AppBar` hold their own literals — and `AppBar::center_title`,
  from milestone 306, still resolves `caller ?? platform` with the theme's term missing from
  the middle.
- **Nothing carries a theme across `build_ui`.** An application that swaps the *root* theme
  without rebuilding its view would replay cached repaint boundaries in the old colours. It
  cannot happen today, because the theme comes from application state and a state change
  rebuilds, but nothing enforces it.
