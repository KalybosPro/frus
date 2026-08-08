# Milestone 16 — Library of named (themed) widgets

Adds ready-to-use application components, all **themed** (they read the theme at
paint time) and animated.

## Widgets shipped

| Widget | Role |
|---|---|
| **`Button`** | themed button, `Primary` / `Secondary` / `Danger` variants, hover/pressed, shadow |
| **`Checkbox`** | **controlled** checkbox (box + ✓ + label); `on_toggle(bool)` |
| **`Switch`** | **controlled** pill switch; `on_toggle(bool)` |
| **`RadioGroup`** | single-selection radio group; `on_select(index)` |
| **`Dropdown`** | **controlled** drop-down list (expands in place); `on_toggle` + `on_select(index)` |
| **`Slider`** | **draggable** `0..=1` slider; `on_change(f32)` |
| **`Card`** | themed surface (background, border, radius, shadow) wrapping one child |

## New mechanism — generic widget dragging

For the `Slider` (and future handles), dragging is generalised:

```rust
trait Widget { … fn draggable(&self) -> bool { false }
                  fn on_drag(&self, fraction: f32) -> Option<Msg> { None } }
Ui::draggable_at(point) -> Option<(WidgetId, Rect)>
```

The shell: `MouseDown` on a draggable → `Drag::Widget{id, rect}` (and applies it
immediately); `CursorMoved` → `fraction = (x − rect.x)/rect.width` →
`find_widget(id).on_drag(fraction)` → `Msg` → `update`. The same scheme as the
scrollbar, but reusable by any widget.

## Model

- **Controlled** widgets (Checkbox/Switch/Slider/Dropdown/Radio) take their value
  from the application state and emit a message; the app updates the state.
- `RadioGroup`/`Dropdown` are **containers** (a column of options): each option
  is a clickable child → its own identity → so opening and closing the Dropdown
  gets the mount/unmount **fades** for free (J13/J14).

## Demo

A settings `Card` groups a "Done" checkbox, a "Notifications" switch, a volume
slider, a `RadioGroup` of sizes and a `Dropdown` — all themed (light/dark) and
animated.

## Tests

- `Button::on_click`, `Checkbox::on_click` (returns `on_toggle(!checked)`),
  `Slider::on_drag` (fraction → value, clamped).

## Limits (v1)

- `Dropdown` expands **in place** (no floating overlay above the rest).
- No animated state transition for `Switch`/`Slider` yet (the position is
  instant).
