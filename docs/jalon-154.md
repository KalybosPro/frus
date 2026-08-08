# Jalon 154 — Autocomplete: scrollable suggestion list

## Analysis

`Autocomplete`'s floating list (milestones 150–152) **stretched without end**: a query with
many matches produced a list as tall as the screen, overflowing below the anchor. The visible
height had to be **bounded** and the rest left to **scroll**, like a dropdown button's menu
or a Material autocomplete (a ~5–6 option window).

## Technical decisions

- **A `max_visible` threshold, otherwise a bare list.** `Scroll` has a **fixed-height**
  viewport (no "max-height"). Rather than forcing systematic scrolling (a pointless bar over
  2 suggestions), the widget wraps the list in a `Scroll` **only** if the number of
  suggestions **exceeds** `max_visible`; below that, it pushes the plain `Flex` list (the
  original behaviour, no regression). The viewport = `n·ROW_H + (n−1)·gap`.

- **Reuses `Scroll` as is.** No change to the scrollable container: wheel / touch / bar
  already work in the overlay (the `Scroll` registers itself during the build walk, including
  under the floating portal). The suggestions stay **focusable**; keyboard focus reveals the
  targeted suggestion inside the viewport.

- **Controlled and opt-in.** `max_visible` defaults to `None` (unlimited). The application
  picks the window; the scroll state is retained in the runtime by identity, like any
  `Scroll`.

## Implementation

- `autocomplete.rs`: the `max_visible: Option<usize>` field + `.max_visible(n)`; `rebuild`
  wraps the list in `Scroll::new().width(w).height(viewport)` past the threshold, otherwise
  pushes the plain list; the `ROW_GAP` constant.
- `goldens.rs`: the `autocomplete_scroll` golden (6 suggestions, `max_visible(3)`).

## Verification

- **Unit**: past the threshold, the overlay is a `Scroll` whose height = 2 lines
  (`max_visible(2)`, 4 suggestions) and which does contain all **4** suggestions; below it
  (`max_visible(5)`, 2 suggestions), the overlay stays the **bare list** (2 children, the 1st
  = `Pick("a1")`). The J150–152 tests unchanged.
- **Golden** `autocomplete_scroll` **inspected**: a 3-line viewport (Alabama / Alaska /
  Arizona), a **scrollbar** on the right (≈ half → 6 items), "a" highlighted.
- `cargo test --workspace` **green**.

## What's left

- **Auto-scrolling to the active suggestion**: revealing `active` in the viewport when the app
  advances it from the keyboard (today only the real **focus** is revealed).
- **Adaptive height**: bounding by pixels (`max_height`) rather than by line count, useful if
  the suggestions become widgets of varying height.
