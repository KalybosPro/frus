# Milestone 140 — Multi-line field scrollbar (+ touch)

## Analysis

Milestone 139 made the multi-line field scrollable with the **wheel**, but with no visible
affordance and no touch route: with a finger, a press starts a text **selection**, not a
scroll. The missing piece is a **scrollbar** — which, in frus, is already draggable by
mouse **and** by touch (the `Drag::Scrollbar` gesture is source-agnostic). One bar
therefore covers both needs at once.

## Technical decisions

- **Reuse the generic bar.** Where the field registered itself as a scrollable region
  (milestone 139), it now also calls `add_scrollbar(id, viewport, …)` — exactly like a
  `Scroll` or a virtual list. The bar is drawn, its thumb registered (`scrollbar_at`), and
  the shell drags it through `Drag::Scrollbar` on click **as with a finger** (`pointer_down`
  tests `scrollbar_at` for every source).

- **The bar hugs the box, not the widget.** The scrollable region and the bar must run
  along the **input box**, not the whole widget (which includes the floating label above). A
  `Widget::text_viewport(rect)` method yields that frame (below the label, `rows` high); the
  scrollable registration and the bar both use it, so the thumb lines up exactly with the
  scrollable text.

- **Nothing new on the interaction side.** Wheel, inertia, elastic overscroll, thumb
  dragging: it all comes from the existing scrolling machinery. The field merely **declares
  itself** into it (region + bar) through `text_metrics` (overflow) and `text_viewport`
  (frame).

## Implementation

- `widget.rs` (+ the `Box`/`Keyed`/`Responsive` forwarders): the `text_viewport` method.
- `textinput.rs`: the `text_viewport` impl (the box below the label, `field_height` tall).
- `ui.rs`: the walk registers the region **and** adds the bar over that frame, with the
  current retained offset.

## Verification

- **Rendered and looked at**: the bar runs along the right edge of the box (below the
  "Notes" label), the thumb reflecting the scroll — the `multiline_scrolled` golden
  regenerated.
- **No regression**: the `frus-widgets` + `frus-test` suite green; a short field still has
  no bar (no overflow).
- The thumb reuses the `Drag::Scrollbar` drag already covered by the scrolling tests (mouse
  and touch go through the same `pointer_down`).

## What's left

- **Finger scrolling directly on the text** (a fling): still conflicts with selection; left
  as is (the bar is the touch affordance).
- **Auto-hiding** the bar (appearing only on hover/scroll), overlay style.
- **↑/↓ arrows** moving the caret between lines (the next milestone).
