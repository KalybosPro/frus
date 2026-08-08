# Jalon 32 — New widgets (6)

A batch of six widgets covering indicators, structure and layering.

## Widgets

- **`ProgressBar::new(value 0..1)`** — a determinate bar (`muted` track +
  `primary` fill, rounded; the value is clamped).
- **`Divider::new()`** — a thin horizontal `theme.border` separator (stretched by
  the parent).
- **`Badge::new(text)`** — an accent pill (counter / label).
- **`Stack::new().layer(a).layer(b)`** — **superimposed** layers (same box, last
  one on top). Each layer fills the stack; fine positioning happens *inside* the
  layer (e.g. an aligned `Flex`). Handled as a special branch in `build_ui` (like
  `Scroll`/`Navigator`); the layers are complete subtrees (nested overlays and
  scrolls allowed, because they are stored → lifetime `'a`).
- **`Tabs::new(selected, on_select).tab(label, content)`** — **controlled** tabs;
  a `[header, panel]` composite (only the selected panel is realised).
- **`Spinner::new()`** — a **continuously animated** activity indicator.

## Continuous animation (for `Spinner`)

A new, reusable mechanism:

- `Widget::continuous(&self) -> bool` (default `false`); `Spinner` returns
  `true`.
- The driver sets a `Ui::wants_animation()` flag as soon as a continuous widget
  is met; the shell **redraws for as long as it is true**.
- A **clock** `Runtime.time` (in seconds, advanced by the shell) is exposed to
  widgets through `Status::time` — the `Spinner` derives its rotation phase from
  it. A base for any time-driven animation (pulsing, continuous scrolling…).

## Demo (integration)

- Header: `Stack(Spinner + Badge)` (a pill showing the number of active tasks, on
  the spinner) — shows Stack + Spinner + Badge together.
- Todo card: a `Divider` + a completion `ProgressBar` (done / total).
- Settings screen: `Tabs` ["Controls", "About"].

## Tests

- `ProgressBar`: fill ∝ value (0.5 → 50/100), clamping.
- `Divider`: a `border` line.
- `Badge`: pill + text.
- `Stack`: the layers share the same origin (superimposed).
- `Tabs`: `[header(N buttons), panel]`; no panel when the selection is out of
  bounds.
- `Spinner`: a ring of `DOTS` dots, a time-dependent distribution,
  `continuous() == true`.
- 56 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Spinner`: fixed number of dots; accent colour.
- `Stack`: full-frame layers (fine positioning delegated to the layer).
- `ProgressBar` is determinate only (indeterminate = a `Spinner`).
