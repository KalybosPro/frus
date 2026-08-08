# Milestone 40 — New widgets: Popover, Autocomplete, Kbd

An "overlays & input" batch. (UI text in English, as preferred.)

## Widgets

- **`Popover::new(anchor, open, on_dismiss).content(widget)`** — an anchored,
  controlled floating panel with **free content**, closing on an outside click.
  Generalises `Menu` (which only accepts action items); reuses `Portal`
  (auto-flip + clickable scrim + `overlay_dismiss`).
- **`Autocomplete::new(value, on_input, on_pick).suggestion("...")`** — an input
  field (`TextInput`) with a floating **suggestion list** (through a `Below`
  overlay). **Controlled**: the app supplies the value *and* the already-filtered
  suggestions; the list only floats when it is non-empty. Typing → `on_input`;
  clicking a suggestion → `on_pick`.
- **`Kbd::new("Enter")`** — a keycap (shortcut hint): a small rounded frame + a
  muted label.

## Demo (the "About" tab)

- A **`Popover`** "Info" opening a details panel (free content).
- An **`Autocomplete`** "tag" with suggestions filtered by what is typed
  (`State.tag_draft`, `Msg::TagInput/TagPick`).
- **`Kbd`**: the line "Shortcuts: [Enter] add [Tab] navigate".

## Tests

- `Popover`: closed → no overlay; open → overlay + `overlay_dismiss`; content.
- `Autocomplete`: no overlay without suggestions; otherwise a floating list;
  clicking a suggestion → `on_pick(label)`.
- `Kbd`: keycap (border) + label painted.
- 90 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Autocomplete`: no keyboard navigation through the suggestions (arrows);
  filtering is the app's business.
- `Popover`: `Below` placement (no choice of top/left/right anchoring).
