# Milestone 18 — Navigation (screen stack + slide transitions)

Adds multi-screen navigation with an animated transition on push and pop.

## Mechanism

The [`Navigator`] shows one **full-window screen**. During a transition it
renders **two** screens (outgoing + incoming), offset horizontally according to a
`0 → 1` progress.

- `Widget::navigator() -> Option<(progress, push?)>`;
- `build_layout`: a navigator is a **leaf** (its screens are laid out separately,
  full-window);
- `build_ui`: for each screen, a sub-layout at window size and then an offset
  render (`render_screen`). The offsets:
  - **push**: incoming from the right (`x = (1−p)·w`), outgoing to the left
    (`x = −p·w`);
  - **pop**: the other way round.

The `Navigator` is **controlled**: the application owns the route stack and the
progress, and rebuilds the screens each frame.

## API

```rust
Navigator::new(current_screen, w, h)                    // no transition
Navigator::new(current, w, h).from(previous, p, forward) // transition in progress
```

## Loop (shell)

```
Msg::Push(route) → nav_from = current screen ; routes.push(route) ; progress = 0 ; forward
Msg::Pop         → nav_from = current screen ; routes.pop() ; progress = 0 ; back
Redraw           → if nav_from: progress += dt/duration ; at 1 → nav_from = None
                   view = Navigator around the current screen (+ from during a transition)
```

## Demo

Three screens: **Home** (with "Details →" / "Settings →" buttons), **Details**,
**Settings** (the controls card). Each screen has a "← Back" button. Screen
changes **slide**.

## Tests

- `Navigator::navigator()` exposes the progress and direction.
- A transition renders **both screens** (outgoing + incoming).

## Limits (v1)

- **Horizontal slide** transition only (no choice of fade or scale).
- Screens rebuilt every frame during the transition (no render cache).
- No "back gesture" (swipe) and no built-in navigation bar.
