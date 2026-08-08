# Milestone 85 — Accessibility: semantic annotation + AccessKit bridge

## Analysis

§14 recommends: *do NOT invent accessibility — adopt **AccessKit** (the
cross-platform Rust standard: UIA on Windows, AT-SPI on Linux, macOS). frus
annotates the semantics per widget (role/label/value/state), and AccessKit talks
to the native screen readers.* And: *bake the label into the widgets now, wire
AccessKit up afterwards.*

## Architecture

- **`frus-core/semantics.rs`** (zero-dependency): `Role` (Button, CheckBox,
  Switch, Slider, TextInput, Label, ProgressBar, Tab, ListItem…), `Toggled`
  (None/False/True), and `Semantics { role, label, value, toggled, clickable,
  disabled, range }` with builders. A frus-native type, **mappable** onto
  `accesskit`.
- **`Widget::semantics()`**: a hook (default `None` = a layout container),
  delegated by `Box`/`Keyed`/`Responsive`. Implemented on Button, Text, Checkbox,
  Switch, Slider, ProgressBar and TextInput.
- **Collection**: `build_ui` harvests the meaning-bearing nodes (a non-null role
  or a label) with their visible bounds, in **paint order** (= reading order) →
  `Ui::semantics()`.
- **`frus-shell/a11y.rs`** (desktop only): maps the frus tree onto an
  `accesskit::TreeUpdate` (a `Window` root + one node per widget), and wires up
  the `accesskit_winit` adapter:
  - a **shared snapshot** (`Arc<Mutex>`) written each frame, read by the
    `ActivationHandler` on the AT's thread;
  - an **action queue**: the `ActionHandler` translates a click or focus
    requested by the AT into an `A11yAction`, replayed in the loop
    (`drain_a11y_actions` → `dispatch(msg)` / focus) — so the AT can both **read
    and activate** the UI.
  - `Deactivation` is a no-op (the tree rebuilds on the next frame).

A constraint: `accesskit_winit` requires the window to be **hidden** when the
adapter is created → so it is created `with_visible(false)` and revealed
afterwards (desktop only; Android is unaffected).

## Decisions

- A **flat** tree under the root (paint order) for this first pass: screen
  readers navigate an ordered list. A faithful hierarchy (groups) will come if
  needed.
- Android excluded by `cfg`: AccessKit has a separate provider there (a separate
  piece of work). The semantic hook itself is cross-platform and already baked
  in.
- `WidgetId::from_u64` added (public): an action coming from the AT is
  re-identified back to the widget (the inverse of `as_u64`).

## Tests (287 → 296)

- `frus-core`: the `Semantics` builders, `is_meaningful`.
- `frus-widgets`: `semantics_tree_carries_roles_and_labels` (the tree carries
  roles/labels/states; the container is ignored).
- `frus-shell`: the role mapping onto AccessKit, the `TreeUpdate`'s structure
  (root + children), focus pointing at a node that exists, a `node_id` round
  trip.

## Validation

- 21 suites green; a desktop smoke run: the app runs with the adapter (window
  hidden→revealed), **inert** under WSL for want of an AT bus, without crashing.
  Android compiles (a11y excluded).
- **An honest limit**: actual reading by a screen reader (NVDA/Orca/VoiceOver) is
  not observable in this environment (no AT-SPI bus under WSL). The bridge is
  correct by construction and tested at the mapping level; end-to-end AT
  validation is a separate desktop-with-a-reader task.

## What's left

- Progressive adoption of `semantics()` across more widgets (RadioGroup, Tabs,
  links, images).
- An Android AccessKit provider.
- A semantic hierarchy (groups/regions) if a screen reader calls for it.
