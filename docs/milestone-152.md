# Milestone 152 — Autocomplete: text highlighting & active suggestion

## Analysis

`Autocomplete` (earlier milestones, its width settable since milestone 150) showed its
suggestions in **plain** text, without two cues expected of a Material field:

- **Highlighting** the part of the label that **matches** the query (the "why it matches").
- An **active suggestion** highlighted — the one that would be chosen, walked from the
  keyboard.

Milestone 150 had noted both in its "What's left", along with "keyboard descent from the
field".

## Technical decisions

- **Highlighting by segments.** The suggestion splits its label into three segments
  `[before | match | after]` (a **case-insensitive** substring search, **character**
  indices — robust outside ASCII) and draws the match in the `primary` colour, the rest in
  `on_surface`. Three `text()` calls positioned by width measurement; the match may be **in
  the middle** of a word (e.g. "gr**ap**e").

- **An active suggestion, like the Dropdown.** `active(index)`: the suggestion at the active
  index gets the **tinted background** `surface.lerp(primary, 0.14)` (hover on top), exactly
  like a `Dropdown`'s selected option — visual consistency across the framework.

- **Keyboard descent: already there.** No need to touch the shell's keyboard routing: the
  suggestions are **focusable**, so the down arrow from the single-line field (whose vertical
  cursor move returns `None` at the boundary) **navigates the focus** to the first
  suggestion; Enter chooses it (the shell activates any focused `on_click`). The app keeps
  the "active" model (a highlighted index) if it prefers driving from the keyboard without
  moving the focus. Checked by a focus-cycle test.

## Implementation

- `autocomplete.rs`: the `match_range` helper (a case-insensitive substring, character
  indices); `Suggestion` gains `query`/`active` and paints in segments; `Autocomplete` gains
  `active` + `.active(index)`; `rebuild` passes `query` (= the value) and the active index.
- `goldens.rs`: the `autocomplete` golden (an "ap" field, the list, the 2nd active, "ap"
  highlighted).

## Verification

- **Unit**: `match_range` ("Apricot"/"ap" → `(0,2)`, "pineapple"/"APPLE" → `(4,9)`, an empty
  / absent query → `None`); the matching part is a separate text **segment** ("ap" +
  "ricot" for "apricot"); the **active** suggestion is highlighted (a tinted rect); the
  field **then** a suggestion enter the focus cycle.
- **Golden** `autocomplete` **inspected**: "ap" in green in each suggestion (including in
  the middle of "grape"), "apricot" (active) highlighted. `cargo test --workspace` **green**.

## What's left

- **Scrolling** the suggestion list when it is long (a height bound + a `Scroll`).
- **A highlight that follows the focus**: tying `active` to the real keyboard focus (today
  the app drives one or the other) for a single cue.
