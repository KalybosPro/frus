# Milestone 337 — Focus, said out loud

Six of the twenty-two widgets milestone 336 counted were the focus group. Five of them are
here; the sixth waits on the subsystem it is really made of.

## What already existed, and what did not

Focus was a **property of a widget**: a text field answers `focusable()`, a label does not,
and Tab walks the tree in the order the walk produced. That is enough for a form and it has
been enough for 336 milestones.

What was missing is the surface the reference puts over it — the ability for a *caller* to
say something about focus without owning the widget that has it:

| | |
|---|---|
| `Focus` | make this a stop, whatever the child thinks |
| `ExcludeFocus` | nothing in here is reachable at all |
| `ExcludeFocusTraversal` | keep the stop, take it out of Tab's order |
| `FocusTraversalOrder` | these come in this order |
| `FocusTraversalGroup` | resolve that order among these and nowhere else |

## The two questions Tab and a click ask separately

`ExcludeFocus` and `ExcludeFocusTraversal` look like the same feature and are not, which is
why the reference has both and why the doc says so twice. A panel behind a sheet is
**unreachable**: nothing in it should register a stop. A toolbar button is **reachable**
and simply does not belong in a form's keyboard order: a click focuses it, Tab passes it
by. Collapsing the two would make one of those two cases impossible to express.

So a focus stop is no longer `(id, rect)`. It is `(id, rect, skip, order, group)`, and
`Ui::traversal_order()` is where the four are resolved.

## Subtree scope, and the bug it prevents

The four flags belong to a **subtree**, not to the widget carrying them: an
`ExcludeFocus` around a column excludes everything in that column and nothing after it.
The walk therefore pushes them on the way in and pops them on the way out.

That sounds obvious and is exactly the kind of thing that goes wrong: the body of the walk
has early returns in a dozen branches — a barrier, an opacity group, a clip, a scrollable,
a stack, a page view — and a flag set on the way in and cleared at the end of the function
would leak out of every one of them. The scope is a wrapper around the walk rather than a
line inside it, which is the only shape that closes on every path. `the_flags_do_not_leak_to_what_comes_after`
is the test, and it is the one worth keeping.

## Ordering is local, deliberately

Two properties, both chosen so that an explicit order is a *local statement* rather than a
rearrangement of the frame:

- **Stable.** Everything without an order keeps tree order, and so do ties. Ordered stops
  sort ahead of unordered ones.
- **Grouped.** The sort runs within a traversal group, and a group's stops are contiguous
  because the walk is depth-first. So a dialog that swaps its two fields does not touch the
  page behind it — which the test checks by asserting the page's two fields are still in
  tree order around a dialog whose two are not.

The shell needed no change: Tab already went through `focus_next`, and `focus_next` now
asks `traversal_order()`.

## Left

- **`FocusableActionDetector`**, the sixth. It is focus *plus hover plus shortcuts plus
  actions*, so it belongs with the shortcuts-and-actions subsystem rather than here — the
  next milestone.
- **`onFocusChange`.** The reference's `Focus` reports gaining and losing focus as a
  callback. Nothing here emits a message on a focus change; the runtime knows, and no
  widget can ask. That is a message-plumbing question, not a focus one.
- **Traversal policies.** The reference has reading-order and directional policies as
  objects a group can carry; here the group carries a numeric order and the walk's order,
  and directional movement is a separate geometric search that has been in place since
  long before this.
