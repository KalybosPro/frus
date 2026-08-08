# Jalon 139 — Multi-line field scrolling (wheel)

## Analysis

The multi-line field (milestones 137–138) scrolled **only** to follow the caret,
recomputed in `paint`. There was no way to **browse** content taller than `rows` with the
wheel: the scroll was not *retained* and the field was unknown to the framework's scrolling
system. This milestone plugs the field into that system.

## Technical decisions

- **The field becomes a real scrollable region.** Instead of an ad-hoc mechanism, the
  multi-line field **registers itself** (like a `Scroll` or a virtual list) through
  `self.scrollables.push((id, viewport, 0, max_y))` when its content overflows. It thereby
  inherits **for free** all the existing machinery: wheel hit-testing (`scroll_hit`), target
  + inertia (`scroll_target`/`advance_scroll`), elastic overscroll.

- **The scroll is retained in the runtime.** The offset lives in `runtime.scroll[id].1` (the
  same map as the `Scroll`s). `Status` gains `scroll_y` (filled by `full_status` from that
  map): the field's `paint` scrolls its text by `scroll_y` (clamped to the overflow) instead
  of deriving it from the caret.

- **Caret following moves from `paint` to the shell.** Since `paint` cannot write to the
  runtime, it is the shell that, **on every keystroke** (`apply_key` → `reveal_caret`),
  adjusts `runtime.scroll[id]` just enough to keep the caret visible — through a
  `[caret bottom visible, caret top visible]` window. You type → the field recentres; you
  wheel freely otherwise. A single `Widget::text_metrics(width, cursor)` method →
  `(content height, visible height, caret top, caret height)` feeds **both** the
  registration (overflow) **and** that following.

- **Clicking stays accurate.** `cursor_at` no longer estimates the vertical scroll (it used
  to derive it from the caret): the shell now adds the **retained** scroll to `local_y`
  before the call, so clicking on a scrolled line lands exactly.

## Implementation

- `interaction.rs`: `Status.scroll_y`.
- `ui.rs`: `full_status` fills `scroll_y`; the walk registers an overflowing multi-line
  field as scrollable; the `Ui::scrollable_viewport(id)` accessor.
- `widget.rs` (+ the `Box`/`Keyed`/`Responsive` forwarders): the `text_metrics` method.
- `textinput.rs`: the `content_width` helper; `text_metrics`; `paint` scrolls by
  `status.scroll_y`; `cursor_at` with no vertical estimate (the shell supplies it).
- `app.rs`: `reveal_caret` (caret following on keystroke); both `cursor_at` call sites add
  the retained scroll to `local_y`.

## Verification

- **Rendered and looked at**: a 3-line field scrolled by ~2 lines shows "Line three/four/
  five", clipped to the box — the `multiline_scrolled` golden.
- **Unit**: `text_metrics` reports the overflow and `scroll_y` moves the text up; the
  overflowing field registers as scrollable (`max_y > 0`), a short field does not.
- **The whole suite** green, no golden moved.

## What's left

- **Touch scrolling** with a finger: on a field, a press starts a text **selection** (not a
  scroll) — a distinct gesture to arbitrate (a dedicated scrollbar, or two fingers). Here:
  wheel (and inertia) only.
- A visible **scrollbar** for the field (today: wheel only).
- **Up/Down arrows** moving the caret between lines in the field (today they navigate the
  focus) — the multi-line field's keyboard complement.
