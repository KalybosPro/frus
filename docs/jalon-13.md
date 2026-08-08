# Jalon 13 — Opacity + fade-in

Adds an opacity propagated through rendering, and a **fade-in** when widgets are
mounted.

## What ships

- **`Color::fade(opacity)`** (frus-core): multiplies the alpha channel.
- **`Anim.opacity`** (default **1.0**, started at 0 on mount); `Runtime::advance`
  drives it towards 1. `Runtime::opacity(id)`.
- **`Runtime.mounted: HashSet<WidgetId>`**: the set of widgets present; a **new**
  id starts at opacity 0 (hence fades in).
- **`collect_ids(&tree)`**: a lightweight walk by identity, to diff mounting and
  unmounting before `build_ui`.
- **`Status.opacity`**: read by the widgets, which **multiply the alpha** of all
  their colours (`Container`, `Text`, `TextInput` — text, background, border,
  caret, selection, shadow, gradient).

## Loop (shell, RedrawRequested)

```
tree = view(state)
ids = collect_ids(&tree)
for each new id (not in runtime.mounted): mounted.insert(id) ; anims[id].opacity = 0
mounted.retain(present)                // re-appears if re-added later
animating = runtime.advance(dt)        // opacity + hover + focus
ui = build_ui(&tree, size, &runtime)   // opacity -> Status.opacity -> alpha
render ; if animating -> redraw
```

## Demo

At start-up, the whole UI **fades in**. Every **new item** added to the list
(clicking the button) **fades in**.

## Tests

- `Color::fade`: alpha scaling.
- `Runtime::advance`: opacity rises from 0 towards 1; default 1 with no entry.

## Deferred (honestly)

- **Disappearance (fade-out)**: a widget removed from the tree has no primitives
  left to draw. Animating that requires **retaining outgoing widgets** (their
  primitives or their subtree) for the duration of the transition — a
  reconciliation pass with a list of "outgoing" widgets, which will be a
  dedicated milestone.
