# Milestone 28 — Keyed reconciliation (stable identity)

Widget identity was **positional** (root path → child indices). Deleting an item
**in the middle** of a list shifted the identity of the ones after it, and their
retained state (hover, focus, caret, animations, exit fade) "jumped". **Keys**
make identity stable.

## Mechanism

- `WidgetId::keyed(hash)`: an identity derived from `parent + key` (with a
  constant and an offset distinct from `child(index)` to avoid collisions).
- `Widget::key(&self) -> Option<u64>` (default `None` → positional).
- **`Keyed<Msg>`**: a **transparent** wrapper — it delegates *every* trait method
  to the inner widget, but returns `key() = Some(...)`. `Keyed::new(key, w)`
  accepts any `Hash` type.
- **DX**: a `keyed(key, widget)` helper in the DSL.

The heart of the consistency: a single `child_id(parent, index, child)` —
`child.key()` ? `parent.keyed(k)` : `parent.child(index)` — used **everywhere an
identity is derived**: `build_ui` (the normal branches **and** scroll / nav /
overlay), `collect_ids`, `find_widget`, `advance_values`. If one of them
diverged, the state would no longer match; hence the integration test.

## Demo

Task rows are wrapped: `keyed(todo.id, todo_row(...))`. Deleting a task in the
middle no longer makes the other rows' retained state "jump".

## Tests

- `WidgetId::keyed`: stable (same key = same id), ≠ `child(index)` of the same
  value, ≠ another key, ≠ the same key under another parent.
- `Keyed`: returns a key and delegates children and style; same key → same hash.
- **Integration** (`keyed_identity_survives_middle_removal`): in a list of 3
  keyed coloured items, the `owner` (= id) of one item's primitive is read; after
  removing the middle one, the surviving item's `owner` is **identical** —
  whereas **without a key** it changes. Direct proof.
- 41 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- Keys target **list children** (siblings); unique structural children (a
  `Scroll`'s content, a `Navigator`'s screen) also honour keys through
  `child_id` but stay positional in practice.
- Key hashed to `u64`: theoretical collisions (negligible).
