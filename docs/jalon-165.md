# Jalon 165 — Accessibility: spoken announcements (live region)

## Analysis

Column reordering offered a **visual** cue (the ghost + the slide) and, at rest, a
**re-readable** position ("column N of M" in the header's semantics). But a screen-reader user
"sees" neither the ghost nor the drag: at the moment of the drop (mouse) or the keyboard step
(Ctrl+Arrow), **nothing was announced**. A **live region** was needed — the equivalent of the
Web's `aria-live="polite"`: text that assistive technology reads **when it changes**, without
moving the focus.

## Technical decisions

- **A dedicated, polite live node.** The AccessKit bridge gains a reserved node (`LIVE_ID`,
  out of reach of the `WidgetId`s offset by `+1`), a child of the root, with the `Label` role
  and marked `Live::Polite` (it does not interrupt the current reading). Its label carries the
  message. It only appears **if a message is present** (no cost otherwise).

- **Re-announced only on change.** The message **persists** in the snapshot: carried over
  as is every frame, AccessKit does not repeat it (it only speaks when the text changes). So
  the shell has nothing to "clear" — it sets a new text, and the announcement goes out.

- **Shell-driven, generic.** A `set_announcement(String)` method (desktop only, a no-op on
  Android/Web) feeds the field, published each frame through `a11y.update(..., announce)`. Any
  shell event can therefore speak.

- **Both reordering paths.** On the **drop** of a drag and on the **keyboard step**
  (Ctrl+Arrow — the path screen-reader users take, since they do not drag), the shell announces
  "Column moved to position N".

## Implementation

- `a11y.rs`: the `LIVE_ID` constant, `live_node(message)` (the `Label` role + `Live::Polite`),
  `build_tree_update` takes `announce: &str` (the live node + a root child when non-empty),
  `Snapshot.announce`, `A11y::update(..., announce)`.
- `app.rs`: the `announce: String` field, `set_announcement`, called on the drop (`pointer_up`,
  the `Drag::Reorder` branch) and on the keyboard step (the `on_key` Left/Right branch of a
  reorderable header); passed to `a11y.update`.

## Verification

- **Unit**: `announcement_adds_a_polite_live_region` — with no message, no live node; with a
  message, a `Live::Polite` node carrying the text, referenced by the root.
- The `frus-shell` suite **green** (23 tests). `cargo test --workspace` **green**.

## What's left

- **Throughput**: two consecutive **identical** announcements (the same position reached twice)
  do not repeat — acceptable here; a toggle (a non-breaking space) would force a repeat.
- Extending announcements to other gestures (multiple selection "N selected", sorting "sorted
  by X ascending"), on the same mechanism.
- The **Android** AccessKit provider remains a separate piece of work (desktop only here).
