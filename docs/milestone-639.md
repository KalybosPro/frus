# Milestone 639 — The stack, as the reference's

## Objective

`Stack` in the review against the reference (`docs/reference-review.md`), read in
`widgets/basic.dart:4743` and `rendering/stack.dart`. frus's stack differed in what matters
most about a stack:

| | reference | frus before |
|---|---|---|
| its size | its largest unpinned child, held to its constraints (`stack.dart:625`); with none, all the room (`:657`) | only its own style or its parent: a stack of a 40 × 40 avatar in a column was 0 px tall, and the next child was drawn over it |
| default fit | `loose` (`:301`) | `Expand`, documented as a deliberate difference |
| `expand` | every unpinned child **forced** to the stack's size (`:636`) | handed the box; a child with a size kept it |
| `passthrough` | the stack's own constraints, passed through | missing |
| `clipBehavior` | `hardEdge` by default; `Clip.none` lets a child hang over the edges (`:718`) | always cut at the edges |
| touch | only inside the stack's box, whatever is drawn outside (`box.dart` asks the box first) | — |
| `textDirection` | the stack's own, for its alignment and directional pins | the theme's only |
| `Positioned.fill`, `fromRect`, `fromRelativeRect` | yes | `inset(px)` only |
| `IndexedStack` | sized as a stack, over every child, shown or not (`:768`) | the size its style said |

The documented reason for the `Expand` default — a childless box would hug and come out at
nothing — no longer held once milestone 633 made an empty container take the room it is
allowed.

## What was done

- **A stack sizes itself from its layers.** `Stack` and `IndexedStack` are measured: each
  unpinned layer is laid out in a tree of its own under the stack's fit, and the stack is the
  largest of them, held to the room offered. With every layer pinned, or under `Expand`, the
  stack asks for all the room it is given. Pinned layers never size it.
- **The reference's three fits**, `Loose` the default:
  - `Loose` allows each layer up to the stack's box. A layer that asks for the room still
    takes it, as a box with nothing to size it takes the biggest size it is allowed in the
    reference.
  - `Expand` forces every unpinned layer to the stack's box.
  - `Passthrough` hands the layer the box: one with no size fills it, one with a size keeps
    it. That is the reference's passthrough for a stack that is allowed its room, and it is
    what every stack-shaped internal widget (`Dismissible`, `Hero`, the tables,
    drag-and-drop) did before. The `stack_fit` hook defaults to it, so they are unchanged.
- **`Stack::clip_behavior(Clip)`**, with the reference's `Clip` (`None`, `HardEdge`,
  `AntiAlias`, `AntiAliasWithSaveLayer`). `Clip::None` lets a layer hang over the edges.
- **Touch stays inside the stack.** A hanging layer is drawn outside the stack but can only
  be pressed or dragged inside it, through a touch clip kept apart from the paint clip. Focus
  and semantics keep the drawn box, so Tab still reaches a hanging button.
- **`Stack::text_direction`** and `IndexedStack::text_direction`, for the alignment and the
  start and end pins.
- **`Positioned::fill`, `from_rect` and `from_relative_rect`.** The last takes `Insets`
  (left, top, right and bottom distances), which is what the reference's `RelativeRect` is.
- **`PageView` asks for the room even with a flex factor.** A flex factor is only heard by a
  row or a column. In a stack's layer the page view collapsed to nothing, where the
  reference's takes all its room. Found by the demo's guided tour, whose page view is a
  stack layer.

## Hooks

`Widget::stack_loose` is replaced by `stack_fit` (default `Passthrough`). New:
`stack_measured`, `stack_clips` and `stack_direction`. Every transparent wrapper forwards
them, and `Responsive` now forwards `stack_visible` too.

## Not done here

- A stack's **baseline** (the highest of its children's, `stack.dart:535`): a stack still has
  none of its own.
- `IndexedStack` with no shown child (the reference's `index: null`): an index past the end
  shows nothing, which covers it.

## Tests

- `a_stack_is_as_big_as_its_largest_unpinned_layer`: a column's next child starts under the
  tallest unpinned layer, not the pinned one.
- `with_only_pinned_layers_it_takes_the_room`: the fill request, and a pinned badge in the
  corner of the box the stack fills.
- `the_three_fits_are_the_reference_s`: loose by default; expand forces even a sized layer;
  passthrough fills an unsized layer and keeps a sized one.
- `a_layer_hangs_over_only_when_told` and `a_hanging_layer_is_only_touched_inside_the_stack`.
- `the_stack_s_own_direction_is_read`, `the_positioned_shorthands`,
  `an_indexed_stack_is_as_big_as_its_largest_child` (both pages).
- The demo's `no_screen_draws_outside_itself` caught the page view collapsing in the tour.

Mutation testing: nine mutants, five killed at once. Of the four survivors:

- Three were gaps in the tests, now closed:
  - **The largest layer, not the last:** the test's tallest layer was the last one.
  - **The clip itself:** a test checked where a hanging layer was drawn, not whether it was
    cut.
  - **A loose layer that asks for the room:** an empty container in a loose stack takes the
    whole stack, as in the reference.
- One was dead code: a branch for a stack with no unpinned layer, which its fill request
  already covers. It was removed.

Rerun against the new tests, the third gap still survived: passing a loose layer's fill
request on, in the walk and in the stack's measure, was redundant. A layer laid out on its
own takes the room it asks for as a root. Both copies were removed, and all four survivors
are now either killed or gone. The demo's tour was fixed by the `PageView` change alone.
