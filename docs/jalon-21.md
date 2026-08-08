# Jalon 21 — Framework / application split (`run(app)`)

Extracts a **hosting API**: `frus-shell` becomes a pure framework, generic over
an [`Application`] trait; the todo app becomes an **external consumer** living in
`frus-demo`. Motivated by J20, which had made the coupling concrete (the app was
coded *inside* the shell).

## The trait

```rust
pub trait Application {
    type Message: Clone;
    fn update(&mut self, message: Self::Message);
    fn view(&self, theme: &Theme, w: f32, h: f32) -> Box<dyn Widget<Self::Message>>;
    fn theme(&self) -> Theme { Theme::dark() }      // default
    fn tick(&mut self, _dt: f32) -> bool { false }  // the app's own animations
    fn title(&self) -> String { "frus".into() }
    fn can_go_back(&self) -> bool { false }          // enables the back gesture
    fn back_gesture(&mut self, _progress: f32) {}
    fn back_gesture_end(&mut self, _velocity: f32) {}
}

frus_shell::run(MyApp::default())?; // opens the window and drives the loop
```

A minimal app implements only `update` + `view` (everything else has a default).

## Division of responsibilities

| Framework (`frus-shell`) | Application (consumer) |
|---|---|
| Window, renderer, event loop | State (`State`), `update`, `view` |
| `Runtime` (hover/focus/scroll/editing/animations) | Theme + fade (`theme`) |
| Hit-testing, click/keyboard routing, clipboard | Screen transitions, route stack |
| Dragging (bars, selection, handles) | Animation settling (`tick`) |
| **Measuring** the back gesture (edge, progress, velocity) | **Deciding** the gesture (`can_go_back`, `back_gesture*`) |
| Mount/unmount fades | — |

The key point — the **back gesture**: the framework measures (the `BACK_EDGE`
edge zone, progress, smoothed velocity) and calls *hooks*; the app decides
(preview through `view`, commit or cancel by projecting the velocity, spring
settle in `tick`). The framework stays **ignorant of `Route`s**.

## Shared utility

`frus_widgets::spring_step(p, v, target, dt, K, C) -> (p, v, at_rest)`: one step
of a reusable damped spring (screen transitions, gestures). The stiffness,
damping and projection constants are **app policy**.

## File tree

- `frus-shell`: `application.rs` (the trait) + `app.rs` (generic `App<A>`) +
  `run<A>(app)`. **Zero business code.**
- `frus-widgets`: `spring_step` made public.
- `frus-demo`: `TodoApp: Application` (all the former demo code migrates here);
  now depends on `frus-widgets`.

## Tests

- Migrated into `frus-demo`: add/trim, toggle/delete/clear, non-empty render, and
  a new **`back_gesture_flick_commits_pop`** (a quick flick commits the back
  navigation, driven without a mouse through the hooks + `tick`).
- 30 `frus-widgets` tests + 4 `frus-demo` + the `run` doctest.

## What this unlocks

- Writing a frus app **without touching the framework**.
- Several apps and examples side by side.
- A sound base for the next milestones (nav bar, scroll inertia).

## Limits (v1)

- `view` rebuilds the whole tree every frame (no diffing or memoisation).
- No sub-commands or effects yet (no `Command`/async from `update`).
- Navigation is still done "by hand" in the app (the route stack) — a
  first-class router remains a future candidate.
