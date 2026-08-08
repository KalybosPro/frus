# Milestone 42 — Responsive by default

Makes adapting to size **easy and the default**, through three complementary
primitives (each built and tested separately).

## Batch A — Size classes (`SizeClass`)

`frus-core`: `SizeClass { Compact, Medium, Expanded }` (Material 3 breakpoints,
in logical px: < 600 / 600–840 / ≥ 840), `SizeClass::from_width(w)`, `rank()`.

`frus-widgets`: the `Responsive` widget — `responsive(width).compact(a).medium(b)
.expanded(c)` — picks a subtree according to the tier, with a **graceful
fallback** (the nearest tier, preferring the smaller one when the distance
ties). It **delegates everything** to the chosen variant (like `Keyed`), so it
slots in anywhere.

## Batch B — `Wrap` (flex-wrap)

`Style.flex_wrap: bool` → `taffy::FlexWrap::Wrap`. `Flex::wrap()` enables it;
`Wrap::new()` is the named entry point (a row that wraps). Children that overflow
the main axis **reflow** onto a new line, with no breakpoint. The height is
driven by the content (real multi-line layout): it is the right tool for an
"action bar / 3→2→1 tiles".

## Batch C — `LayoutBuilder`

`LayoutBuilder::new(|size| widget)` builds its content **from its real box**, not
just from the window: a component adapts wherever it is placed. The same
mechanics as the virtualised list — a layout leaf, content built on the fly,
rendered through `render_item` — so **no retained state** (hover and clicks are
fine, no persistent focus and no deferred overlay) and **its own size = its
style** (set a height or `flex`).

Picking the right primitive: **`Wrap`** for a reflow at automatic height,
**`Responsive`** to branch on the window's class, **`LayoutBuilder`** to branch
on the real measured box (at fixed height).

## Demo

A responsive task card: width per tier (Batch A), a header whose action buttons
**reflow** in a `Wrap` (Batch B), and a summary line in a `LayoutBuilder` that
shortens its text when the box is narrow (Batch C). The internal fields (input,
progress bar) follow the card's width.

## Tests

- `frus-core`: `SizeClass`'s thresholds + ordering.
- `frus-widgets`: `Responsive` (choice by width, fallback), `Wrap` (style), and
  `LayoutBuilder` (receives its real box, adapts the number of tiles).
- `frus-layout`: `flex_wrap` really does move the overflowing child onto the next
  line (a functional reflow test).

## Limits (v1)

- `LayoutBuilder` has a fixed height (it is a leaf): it does not measure its own
  content — use `Wrap` when the height has to follow the reflow.
- `Wrap`: rows of equal height (no masonry-style packing).
