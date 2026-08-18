# Milestone 336 — The catalogue, counted; and the three largest gaps closed

A deadline arrived: the reference's context, functionality and widgets, finished this
month. The first honest thing to do with a deadline is find out what is actually left, so
this milestone begins with a count rather than a feature.

## The count

Every class in the reference's `widgets/` and `material/` libraries that extends one of the
widget base classes — 391 of them — against this framework's public exports, with a synonym
map for the ones we spell differently (`Row`/`Column` are `Flex`, `TextField` is
`TextInput`, `SegmentedButton` is `SegmentedControl`, and so on) and a skip list for the
ones that cannot apply: platform views, web embedding, windowing, restoration, the
inspector's own scaffolding, and slivers, which are a consequence of the reference's
scrolling architecture rather than a widget a caller reaches for.

**28 names came back unmatched. Six of those were the script's fault** — `Form`,
`FormField`, `TextField`, `AutofillGroup` and two slivers matched nothing because our
exports nest them in a submodule the regex did not read.

So the true gap is **22 widgets out of 391**, and they fall into four groups:

| group | names |
|---|---|
| **rows and boxes** | `ListTile`, `Flexible`, `Placeholder`, `Baseline`, `IgnoreBaseline` |
| **focus** | `Focus`, `FocusTraversalGroup`, `FocusTraversalOrder`, `ExcludeFocus`, `ExcludeFocusTraversal`, `FocusableActionDetector` |
| **shortcuts and actions** | `Shortcuts`, `Actions`, `ActionListener`, `CallbackShortcuts`, `ShortcutRegistrar`, `KeyboardListener` |
| **filters** | `BackdropFilter`, `BackdropGroup`, `ColorFiltered`, `ImageFiltered`, `ShaderMask` |

That is a better position than the roadmap's "More widgets" line suggested, and it makes
the remaining work plannable: three subsystems and a handful of boxes, not a catalogue.

## `ListTile`

The largest single gap, and the most reached-for row in the reference: a drawer entry, a
settings line, a contact, a menu item are all this shape.

The measurements are the reference's Material 3 defaults, taken from the source rather than
from memory — 16 px in front and 24 behind, a 16 px gap either side of the text, and a
height of 56 / 72 / 88 by line count, 48 / 64 / 76 dense. Roles too: `body_large` on
`on_surface` for the title, `body_medium` on `on_surface_variant` below it.

Two decisions worth recording.

**The text column is an `Expanded`, and the slots are not.** That is milestone 334's shape
applied where it belongs: a long title is cut with an ellipsis, and the trailing chevron
keeps its size. A tile is a fixed height, so a title that wrapped would run out of the
bottom of it — except a three-line tile's subtitle, which is what the extra room is for,
and which therefore wraps rather than truncates.

**It composes under the ambient theme, not at construction.** A theme reaches text styles
and colours, so a tile built eagerly inside a `Themed` subtree would come out in the root's
palette. `build_themed` is the hook that already existed for this.

Disabled beats selected, as everywhere else in this framework: a tile that cannot be picked
does not advertise that it was.

## `Flexible`

One line, now that `Expanded` exists: a loose fit is the same wrapper with the basis left
at the content instead of zeroed, so the child takes *at most* its share and less if it
wants less.

One caveat the reference does not have, and it is written into the doc rather than papered
over: flexbox shares a deficit in proportion to basis, so a fixed sibling gives up its
share too unless it says `no_shrink()`. The reference's inflexible children are never
squeezed at all, so it gets that for nothing; here it is one call, and the test says so.

## `Placeholder`

The reference's colour, stroke and 400 px fallback, all overridable. It grows into the room
on offer rather than insisting on its fallback, because a stand-in that pushes the layout
it is illustrating out of shape is worse than useless.

## A guard earned its keep

`every_control_with_an_enabled_flag_honours_all_four` caught a forwarding `on_click` in a
small adapter type inside `ListTile` — a false positive in the strict sense, since the
adapter has no state to disable. The answer was not an exception: the adapter existed only
because `ConstrainedBox` could not take an already-boxed child, which every slot holds. It
has a `new_boxed` now, the adapter is gone, and the guard is untouched.

## Left, in order

1. **Focus** — six widgets, and the framework already tracks focusables in the walk; what
   is missing is the declarative surface over it.
2. **Shortcuts and actions** — six widgets, a whole subsystem, and the largest of the four.
3. **Filters** — five widgets, all GPU work: a colour matrix, a blur behind, a shader mask.
4. **`Baseline` / `IgnoreBaseline`** — taffy has baseline alignment; nothing here reaches
   for it yet.

Not counted here, and larger than any of it: **depth**. A widget's presence in the
catalogue is not the same as its every property, and the roadmap's twenty open items are
mostly depth. The count above is the floor, not the ceiling.
