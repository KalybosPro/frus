# Milestone 175 — Focus restored when an overlay closes

## Analysis

From the keyboard, opening a menu / a modal moves the focus **into** the overlay (trapped since
milestones 172/174). But on **closing**, the focused widget (a menu item) **disappears** from
the tree: the focus was orphaned, and navigation restarted from the **beginning** of the page.
The expected pattern ("roving focus"): return to the **trigger** — the button that opened the
overlay.

## Technical decisions

- **A focus history, shell-side.** Rather than coupling overlay ↔ anchor, the shell keeps a
  small **history** of successive focuses. On each change, the old focus (still present) is
  pushed as a **candidate trigger**. If, after a rebuild, the current focus has **disappeared**
  from the focusables, we **pop** until the first one still present and go back to it. That
  naturally handles **nesting** (a menu inside a modal): the history steps back one level at a
  time.

- **Detected by disappearance, not by event.** The app does not signal "overlay closed"; the
  shell **infers** it by comparing the current focus to the freshly built frame's focusables
  (`Ui::focusable_ids`). Robust and general: any focus that evaporates falls back onto a focus
  ancestor that is present.

- **Pure, testable logic.** The core is a pure function
  `resolve_focus(current, present, &mut history, &mut prev)` — testable without a window; the
  shell merely supplies the present set and applies the result (a redraw if the focus moved).

## Implementation

- `ui.rs`: `Ui::focusable_ids()` (the identities of every focusable in the frame).
- `app.rs`: the `focus_history` / `prev_focus` fields; `reconcile_focus()` called after the
  `Ui` is built (before the AccessKit announcement, with the focus up to date); the pure
  `resolve_focus` function (+ the `FOCUS_HISTORY_MAX` bound).

## Verification

- **Unit**: `focus_returns_to_trigger_when_overlay_closes` — anchor → item (the anchor pushed)
  → the item gone → **back to the anchor**, the history consumed.
  `focus_falls_to_none_when_no_trigger_remains` — with no trigger present, the focus falls back
  to `None`.
- `cargo test --workspace` **green** (25 shell tests).

## What's left

- **Explicit restoration to the exact trigger** (the anchor's id remembered by the overlay)
  rather than through a history: more direct, should some convoluted case escape the heuristic.
- **A visible focus ring** on return: `focus_visible` could be forced to make the jump
  perceptible (as `Command::focus` requests do).
