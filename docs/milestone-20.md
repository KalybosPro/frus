# Milestone 20 — A real example app: todo list

The first **real application** written with frus, to exercise the API end to end
rather than line up feature demos. The home screen **becomes** the todo app; the
Settings screen stays reachable (button + back gesture) so nothing is lost from
the nav / gesture / overlay / theme coverage.

## Features

- **Add** a task: input field + button, **or the Enter key**.
- **Check / uncheck** (strikes the label out and greys it), **delete** (`×`).
- **Filter**: All / Active / Done (the active filter is highlighted).
- **Live counters** ("N active · M done"), **empty state**.
- **Clear completed** with a **confirmation modal** (clickable scrim).
- **Light/dark theme** (fading) and a **scrolling list**.
- Additions and deletions **fade** (mount/unmount, free since J13/J14).

The classic Elm model: `State { todos, draft, filter, next_id, … }`, `Msg`,
`update`, `view`. Tasks carry a stable `id`; the `ToggleTodo(id)` /
`DeleteTodo(id)` messages target by identity, not by position.

## What the real app revealed (the point of the exercise)

1. **A missing API — `TextInput` ignored `Enter`.** A todo list without "Enter to
   add" is not credible. → Added `TextInput::on_submit(msg)`: `Key::Enter` emits
   the message **without** changing the value. The shell wiring was free (Enter
   already went through `on_edit`).
2. **Framework/app coupling.** The application (`State`/`Msg`/`update`/`view`)
   still lives **inside** `frus-shell`, not as an external consumer. Writing a
   real app makes that coupling concrete: the natural next architectural step is
   a **hosting API** (a generic `run(app)`) so the app can be a separate crate.
   Deferred to a dedicated milestone.
3. **Positional identity.** Deleting a task in the middle shifts the positional
   identity of the ones after it (the retained state — hover, animation —
   "jumps"). Acceptable here; it motivates a future **keyed identity**.

## API added

```rust
TextInput::new(value).on_input(Msg::Draft).on_submit(Msg::Add) // Enter = submit
```

## Tests

- `add_todo_from_draft_and_trims_blanks`: adds, clears the field, ignores blanks.
- `toggle_delete_and_clear_done`: checks, counts, deletes by id, clears the
  completed ones.
- `view_builds_a_non_empty_scene`: a rendering smoke test.
- `TextInput`: `enter_submits_without_changing_value`,
  `enter_without_submit_is_noop`.
- Total: **30 frus-widgets tests** + the shell tests.

## Limits (v1)

- The app is still hosted inside the shell (see point 2) — no public `run(app)`.
- No in-place editing of a task, no persistence (in memory).
- Long labels are not truncated (single-line measured text).
