# Jalon 10 — Caret, navigation, selection and clipboard

Makes input fields fully editable and introduces the first **widget state
retained by the runtime, keyed by identity**.

## Architectural decision — a widget-state `Runtime`

Interaction and editing state is gathered into a [`Runtime`] passed to
`build_ui`:

```rust
struct Runtime {
    input: InputState,                 // hover / press / focus
    scroll: ScrollState,               // scroll offsets
    edits: HashMap<WidgetId, Edit>,    // NEW: caret / selection per field
}
struct Edit { cursor: usize, anchor: Option<usize> }  // character indices
```

A field's **value** stays controlled (application state); **caret and
selection** are the first widget state retained **by identity** (`WidgetId`,
Jalon 6) — the real reconciliation brick.

## What ships

- **`Key`** enriched: `Left/Right/Home/End{shift}`, `Delete`, on top of
  `Text/Backspace/Enter`.
- **`Widget`**: `on_edit(&mut Edit, &Key)` (editing), `cursor_at(local_x)`
  (placement on click), `selected_text(&Edit)` (copying).
- **`Status`** carries `cursor`/`selection` → the field draws the caret and the
  highlight.
- **`TextInput`**: insertion at the caret, navigation, Shift+arrow selection,
  deletion (Backspace/Delete), caret placement **on click**.
- **Clipboard** (through `arboard`, in the shell layer): **Ctrl+C/X/V**,
  **Ctrl+A** (select all). Pasting reuses `Key::Text`.
- **`find_widget(tree, id)`**: finds a widget by identity in order to route
  keystrokes and queries to it.

## Runtime loop (shell)

```
ModifiersChanged → tracks Shift / Ctrl
MouseDown        → focus + caret placed at the clicked point (cursor_at)
KeyPressed       → Ctrl+C/X/V/A (clipboard), otherwise on_edit(&mut edit, key)
                   → updates Runtime.edits (+ the value through Msg) → redraw
Redraw           → build_ui(&tree, size, &runtime)
```

## Tests

- Editing: insertion at the caret; Shift+arrow selects and Backspace then
  deletes; Home/End clamp the caret.
- `selected_text` returns the selected range.
- `find_widget` + `on_edit` produce the expected edit message.

## Simplifications (v1)

- Single-line; **character** indices (not graphemes or composite emoji); no
  mouse drag-selection; best-effort clipboard (silent failure when unavailable,
  e.g. an environment with no clipboard).
