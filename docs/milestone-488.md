# Milestone 488 — Two more implicit animations, and one quantity under both

Advances [#30](https://github.com/KalybosPro/frus/issues/30) (five of eleven; the issue
asks for two or three at a time).

`AnimatedAlign` and `AnimatedSlide`. They are one mechanism, the way `AnimatedScale` and
`AnimatedRotation` were one mechanism in milestone 477 — and the reason is the same: to the
paint walk they are the same kind of thing.

## One quantity, two rules

A node's box can be offset by two rules, and both are a **pair of fractions** the walk
multiplies by a box:

- an **anchor** (`Widget::alignment_geometry`) — a share of the *free space* around the
  child, which is where the child sits in it;
- a **slide** (`Widget::translate_fraction`, milestone 487) — a multiple of the *child's
  own* box, which is how far it has moved from where it was put.

So there is one animated quantity, `Widget::anim_offset`, and each rule reads it when the
runtime has one. A node has one of those rules, and the wrappers in `animated.rs` each give
the rule they animate a node of their own, so the two cannot collide — the module has
worked that way since the first of them.

The alternative was two nearly identical timelines, two `advance_*` passes over the tree,
and two of everything to keep in step. What made it one instead is that the *interpolation*
is identical: two numbers, one clock, so a child moving diagonally arrives on both axes at
the same moment rather than on two clocks that happen to agree.

## The anchor animates, unlike a pivot

Milestone 477 wrote down that a scale's **pivot** does not animate — it is a choice of
origin, not a quantity, and interpolating it would slide a shape across the screen while
every number describing the shape stood still. The note ended by wondering whether the same
would apply to an alignment.

It does not, and the difference is worth stating: an anchor **is** where the child is. A
pivot says where the maths starts from and a child that changes pivot has not moved; a child
that changes anchor has moved, and moving it is the entire content of the change. So this
one interpolates.

## The anchor keeps its kind

An anchor is interpolated in **its own coordinates** — `x` for a physical one,
start-to-end for a directional one — and the reading direction is applied to the result
rather than to the two ends. `AlignmentGeometry::with_fractions` is what puts the
interpolated pair back into an anchor of the same kind.

The other way round is wrong in a way that only shows up in a right-to-left script:
resolving both ends first and interpolating the physical numbers makes a directional
movement mirror **at its ends** rather than as a whole. A child crossing from start to end
would still start and finish in the right places, and would cross the box the wrong way
between them. There is a test that reads the same quarter of the way in both directions.

## `AnimatedSlide` is what milestone 487 unlocked

The issue's own note about this one said its offset is in units of the child's own box, so
it needs the child's size at paint time — reachable, but a different quantity from the ones
in that batch. Milestone 487's `FractionalTranslation` is exactly that quantity, so this is
now the animated form of a widget that already exists: `FractionalTranslation::animated`,
and a name over it.

The number a caller writes is a multiple of a size the **layout** decides, which is the
whole difference from an animated `Transform::translate`. A panel parked one width off its
own edge does not need anybody to know how wide it ended up.

## In the demo

The task screen's floating action button **slides out of the way when the quick-actions
sheet opens**, two of its own heights down, and comes back when the sheet closes. It is
what the reference's scaffold does with its own animator, and it is the case the fraction
is for: nothing in that screen knows how big a floating action button is.

It also has to be a widget that is **there in both states**. An implicit animation adopts
its target on mount rather than playing on the frame it is born — otherwise every page
would breathe as it opened — so a thing that appears cannot slide in, and a thing that is
always there can.

## Tests

Mounted, mid-flight and at rest for each, as the issue asks. Plus the fourth test milestone
477 recommended to whoever took the next batch, and it earns its place here: **does the
paint read the tween?** Both rules are checked, because they are two separate lines in the
walk — a 20 px mark halfway across a 100 px box sits at 40, and a mark half way through a
slide of its own width has moved 10. A number that moves correctly and never reaches the
screen passes every value test there is, and this repository has shipped that bug before.
