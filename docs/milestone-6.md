# Milestone 6 — Widget identity + interaction states

Makes the UI responsive to the pointer: hover, press, and a correct click
(committed on release). Establishes **widget identity**, the founding brick of a
future reconciliation.

## What ships

- **`WidgetId`**: a widget's positional identity (hash of the path from the root
  through child indices), stable across frames as long as the structure is.
- **`Interaction`** (`None`/`Hovered`/`Pressed`) passed to `Widget::paint`.
- **`InputState`** (`hovered`/`pressed`): interaction state retained by the
  runtime.
- **`Container`**: `hover_color` / `pressed_color`; `paint` picks the colour
  according to the status.
- **`Ui`**: `hit(point) -> Option<WidgetId>` and `msg_for(id) -> Option<Msg>`;
  `build_ui(root, size, &InputState)`.
- **Runtime** (shell): hover tracked on movement, press on *mouse down*, message
  emitted on *mouse up* **if press and release land on the same widget**.

## Architectural position

*Full* reconciliation (diffing trees to preserve components' internal state
across rebuilds) only makes sense once there are stateful components (text
input, scrolling, animation), which do not exist yet. So this milestone ships
the **founding** brick — identity — plus the **pointer-driven interaction
state**, which is enough for hover/press/click. Subtree diffing will come with
the first stateful component.

## Runtime loop

```
CursorMoved → cursor ; h=ui.hit(cursor) ; if h≠hovered { hovered=h ; redraw }
MouseDown   → pressed = ui.hit(cursor) ; redraw
MouseUp     → if pressed==ui.hit(cursor) { update(state, ui.msg_for(id)) } ; pressed=None ; redraw
Redraw      → ui = build_ui(view(state), size, {hovered,pressed}) ; render
```

A widget's status: `Pressed` if pressed **and** hovered, otherwise `Hovered` if
hovered, otherwise `None` (the same behaviour as `:active`/`:hover`).

## Demo

The "+ Add a square" button lightens on hover, darkens on press, and only adds a
square on release (a real click).

## Tests

- `WidgetId`: same path → same id; different paths → different ids.
- `InputState::status_for`: pressed takes precedence over hover; not "Pressed"
  once the pointer has left the widget.
- `Ui`: `hit`/`msg_for` route correctly; **hover changes the painted colour** of
  the button (checked on the produced primitive).

## Limits (next milestones)

- No **keyboard focus** and no subtree diffing (state preservation).
- **Positional** identity: fragile if the tree structure changes around an
  interactive widget (explicit keys to consider later).
