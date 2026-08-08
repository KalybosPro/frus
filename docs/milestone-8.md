# Milestone 8 — Text input + keyboard focus

Adds an editable input field, keyboard focus, and keyboard events.

## Architectural position

We had announced "real reconciliation". In practice, with the **controlled**
model (the iced approach), it is not needed here:

- the field's **value** lives in the application state (`State`) → preserved
  across rebuilds with no machinery;
- **focus** is runtime state, **keyed by `WidgetId`** (the identity from
  Milestone 6), just like hover and press.

So we ship a complete editable field **without a diffing engine**. A persistent
state tree (real reconciliation) will only be useful for rich uncontrolled state
(caret/selection navigation, internal scrolling, animations): deferred.

## What ships

- **`Key`** (`Text`/`Backspace`/`Enter`) and **`Status { interaction, focused }`**
  passed to `Widget::paint`.
- **`InputState.focused`**: the widget holding keyboard focus.
- **`Widget`**: default methods `on_key(&Key) -> Option<Msg>` and
  `focusable() -> bool`.
- **`TextInput`**: a controlled, focusable field, with a focus background/border
  and a caret; `append` + `backspace`.
- **`Ui::focus_hit(point)`** and **`dispatch_key(tree, focused, key)`**.
- **Runtime** (shell): focus set on click, tree kept in order to route keys,
  winit keyboard events translated into `Key`.

## Runtime loop

```
MouseDown  → pressed = ui.hit(cursor) ; focused = ui.focus_hit(cursor) ; redraw
KeyPressed → if focused { msg = dispatch_key(tree, focused, key) ; update ; redraw }
Redraw     → tree = view(state) ; ui = build_ui(&tree, size, &input) ; render ;
             keep ui (hit-testing) + tree (keyboard routing)
```

## Demo

A "Name: [____]" field editable from the keyboard (focus on click, focus ring,
caret) and a "Hello {name}!" greeting that updates as you type — the field being
controlled, its value comes from the state and returns to it through `Msg`.

## Tests

- `TextInput::on_key`: `Text("c")` then `Backspace` produce the right value.
- `Ui::focus_hit` + `dispatch_key`: a key routed to the focused field produces
  the expected edit message.
- `focusable()`: the field is focusable; focus status is independent of hover.

## Limits (next milestones)

- No **caret navigation** (arrows), no selection, no copy/paste.
- No diffing engine (needless while the state stays controlled).
- Still no scrolling/clipping.
