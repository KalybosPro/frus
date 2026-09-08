# Milestone 487 — The boxes that were left

Closes [#37](https://github.com/KalybosPro/frus/issues/37).

Seven entries on a list, all small, all self-contained. Two of them turned out to be
already here under other names, one is the general form of two others, and the one the
issue says to take first needed a hook in the walk rather than a widget.

## `IndexedStack`, and why it is not a wrapper

A stack that lays **every** child out and paints one. The difference between it and
[`Offstage`] is the whole reason it exists: `Offstage` takes a branch out of the tree —
no box, no paint, nothing retained — which is right for a branch that is genuinely gone
and wrong for the three pages of a tabbed screen. There, every switch re-measures a
subtree that was laid out a moment ago, and the page that comes back comes back blank:
its scroll offset, its caret and its focus were the runtime's record of a widget that
stopped existing.

The obvious implementation is to wrap each unshown layer in a `Visibility` that keeps its
size — the widget that already withdraws paint, input and speech while leaving the box.
It is also wrong, and the reason is worth writing down: **a wrapper changes the layout it
was brought in to preserve**. A stack under `StackFit::Expand` *gives* each layer the
box; with a wrapper in between, the box goes to the wrapper and the real layer, hugging
inside it, comes out a different size from the one it would have had. An indexed stack
whose hidden pages are laid out at the wrong size is an indexed stack that has kept
nothing.

So the withdrawal happens **from outside**, in the walk: `Widget::stack_visible` names the
shown layer, every other one is walked in full and then everything it added — its
primitives, its input targets, its accessibility nodes — is dropped again. That is
`apply_barrier`, the mechanism `Visibility` uses, applied to a subtree the walk chose
rather than to one that wrapped itself. The layer is a bare layer, measured exactly as it
would have been.

The test that carries it is the contrast: with either page shown, both boxes exist and
are the same; put one in an `Offstage` and its box is gone.

## `UnconstrainedBox`, and the general form under it

`ConstraintsTransformBox` says what its child is given, per axis, and is then as big as
what came back — held to what it was itself offered, which is the reference's
`constraints.constrain(child.size)`. `UnconstrainedBox` is that box with the constraint
taken away, exactly as it is in the reference, and `UnconstrainedBox::axis` frees one axis
and leaves the other alone.

Three decisions in it are not obvious.

**A definite axis is handed to the child, not merely constrained.** The framework's layout
engine takes sizes rather than the reference's `(min, max)` pairs, and a child with no size
of its own — a plain container — hugs a definite constraint down to nothing instead of
filling it. So an axis with a number on it is *given* to the child (`compute_filled`, or
`compute_scroll`'s existing rule when the other axis is free) and an axis without one is a
question (max-content). The first version constrained instead, and a box asked to lay its
child out in a hundred pixels came back with a child nought wide — a plausible answer to a
question nobody asked.

**The transform is a small vocabulary, not a closure**, and this is the part the reference
does not have to think about. Its `constraintsTransform` is a function of the incoming
constraints, and it can be, because `performLayout` runs once with the real constraints.
Here the layout and the paint walk are two passes over the same tree, and only the first is
told what the parent offered: by the time the walk lays the child out, the box has been
sized to the child and the offer is not recoverable from it.

The first version kept the offer in a `Cell` for the walk to read, and it was wrong in a way
worth recording. Taffy calls a measured leaf **more than once** — a min-content probe, a
max-content probe, and a final call in which it passes back the size it has already settled
on. The last call is therefore not the deciding one: a box asked for half of a two-hundred
pixel column measured its child at a hundred, was settled at a hundred, and was then asked
again *as* a hundred — so the record said fifty, and the walk drew a child half the size of
the box holding it. A function run again on the wrong input answers plausibly, which is
worse than not answering.

So an axis says one of three things, and each survives the round trip: `Fixed(n)` is `n`
either way, `Unbounded` is the same question with no input, and `AsGiven` produced a child
of exactly the box's own extent, the box being that child held to what was offered. Every
transform the reference itself ships is one of those three. The relayout cache is told
about the box too — its size does not follow from its style, so it poisons its entry the
way a `LayoutBuilder` does.

**The overflow is reported per edge**, from where the child actually landed. A centred child
that is too wide runs past both sides, and a band on one of them would say it was a hundred
pixels too wide rather than two hundred. That visible failure is the difference between this
box and `OverflowBox`, which spills on purpose and quietly.

## `SizedOverflowBox`

`OverflowBox` with the hole given a size: the box is what the neighbours make room for, the
child is laid out at its own size — or at one stated — anchored, and allowed past every
edge. An overflow box fills whatever it is offered, because nothing else can anchor a child
that contributes no size; this is the form for a slot narrower than the space around it.

It differs from the reference in one stated way. There, the child receives the *parent's*
constraints; here it is asked or told, because an overflow box's node is an ordinary leaf
and nothing records what was on offer by the time its child is laid out. The general form
above is where a caller who needs the offer goes.

## `FractionalTranslation`

An offset stated as a fraction of the child's **own** size: `(0.5, 0.0)` is half a width
across, `(-1.0, 0.0)` is a whole width to the left, exactly clear of where it was. Paint
and hit-test, layout untouched, like `Transform::translate`.

It is a separate widget rather than a builder on `Transform` because the number is not
known where the widget is written. The child's size is the layout's to decide, and the one
place that has both the widget's request and the child's box is the walk's child offset —
which is where the multiplication happens, next to the plain translation it is added to,
sign-flipped for a right-to-left reading order for the same reason.

## `KeyedSubtree` and `ListBody`: the two that were already here

`Keyed` **is** the reference's `KeyedSubtree` — a key around a subtree and nothing else,
transparent in every other respect. What was missing is that widget's own bulk
constructor: `Keyed::wrap(index, child)` keeps the key the child already has and falls back
to its position when it has none. It is for keying somebody else's children, where
`Keyed::new` would overwrite the keys the caller set on the items they cared about.

`ListBody` is a `Flex` column, and that is not a shortcut: this framework's flex defaults
are `flex_shrink: 0`, `align: Stretch` and `justify: Start` — a column that squashes
nothing, stretches everything across and packs from the start, which is the reference's
list body rule for rule. So it ships as a named entry point beside `Wrap`, which has had
the same shape since it was written, and what it adds is the statement: this column is the
content of a viewport, and nothing in it is meant to flex.

The one thing `Flex` could not say was the reference's `reverse`, so `Flex::reverse` is the
other half — a flip rather than a flag, so reversing twice is the direction it started
with. A transcript that grows from the bottom is what it is for.

`LimitedBox` stays out, as the issue asks: it needs to know whether the incoming constraint
was *unbounded*, which taffy owns and does not report.

## The picture

`constraint_boxes`: the unconstrained row spilling past both edges of its column with a
band on each, the 40 px slot with a 140 px tile hanging out of it and the neighbour sitting
where the slot really ended, and a tile painted exactly one width clear of the faint one
showing where the layout put it.

`IndexedStack` is not in it, and cannot be: everything it does that a plain stack does not
is invisible by construction — a page laid out and not drawn looks exactly like a page that
was never there. That is what the inspector's boxes are for, and where its tests read.
