# Milestone 136 — Programmatic focus (making `first_invalid` actionable)

## Analysis

Milestone 135 can say **which** field fails (`Form::first_invalid`), but the application
could not **jump to it**: focus lives in the shell's `Runtime`, out of the app's reach. An
app → shell channel to **request the focus** of a field was missing — the equivalent of a
focus node or a `text_input::focus(id)`, but in the Elm spirit (a command, not a mutable
object).

## Technical decisions

- **Focus by key, not by positional identity.** A frus `WidgetId` is positional (a path
  through the tree) — the app cannot compute one. But a widget wrapped by `keyed(k, …)`
  has a **stable identity derived from its key** (`parent.keyed(hash(k))`). So the app
  references a field by the **same key** it gave it: `keyed("email", TextInput…)` then
  `Command::focus("email")`. Zero new widget API — we reuse the existing key mechanism.

- **`Command::focus(key)`, resolved after the build.** `Command` now carries, alongside
  its tasks, **focus requests** (keys hashed as `keyed` does). The shell queues them and
  resolves them against the **freshly rebuilt** tree of the next frame (the state changed
  → the view is rebuilt anyway), through `find_by_key` — which yields the identity of the
  first widget carrying that key. The most recent request that resolves wins, and the focus
  ring becomes visible again (you "jump" to the field).

- **Resolution targets the real focus identity.** `find_by_key` computes the identity
  through the **same** `child_id` as rendering, collection and hit-testing. Setting
  `runtime.input.focused` to that result therefore routes editing and the caret exactly
  like a click (a test checks it: `find_by_key == focus_hit`).

- **`run_command` unified.** The two versions (native thread / Web `spawn_local`) merge
  into one (`&mut self`) which, besides launching the tasks, queues the focus requests —
  one path, a `cfg` on the single launch line.

## Implementation

- `frus-shell/src/command.rs`: `Command` gains `focus: Vec<u64>`; the `focus(key)`
  constructor (a `DefaultHasher` hash, identical to `keyed`); `batch` merges; `is_empty`
  counts the focus; `into_parts()` replaces `into_tasks()`. A test for the key carrying.
- `frus-widgets/src/ui.rs`: `find_by_key(root, key) -> Option<WidgetId>` (exported). Tests:
  distinct resolution per key, equal to the focus identity (`focus_hit`), `None` for an
  unknown key.
- `frus-shell/src/app.rs`: a `pending_focus` field; the unified `run_command` queues the
  focus; after the build, resolution against the fresh tree → `runtime.input.focused`.

## Usage

```rust
// view: name the fields.
keyed("email", TextInput::new(&self.email).label("Email") /* .error(...) */)

// update: on submission, jump to the first invalid field.
let report = Form::new().field("email", &self.email, Rule::email("…")) /* … */;
if let Some(key) = report.first_invalid() {
    return Command::focus(key);
}
```

## Verification

- **Unit**: `Command::focus` carries the right key with no task; `find_by_key` resolves
  each key to a distinct identity, **equal to the one from the focus hit-test**, and
  yields `None` for an unknown key.
- **Multi-target**: compiles natively **and** for `wasm32` (the unified `run_command`);
  the `frus-widgets` + `frus-shell` suites green.

## What's left

- The full example (a form app that jumps to the first invalid field) is still an
  **application** to write; the mechanism itself is in place and tested.
- **Scrolling to the focused field** (if off-screen): to be added the day we have long
  scrolling forms.
