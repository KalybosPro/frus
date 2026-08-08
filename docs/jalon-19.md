# Jalon 19 — State transitions · Back gesture · Advanced overlay

Three interaction refinements, built as sub-batches that were each built and
tested.

## Batch A — Advanced overlay

- **Auto-flip**: an anchored overlay (`Below` / `Tooltip`) that would overflow an
  edge of the window flips to the other side of the anchor (vertically) or is
  nudged back inside the window (horizontally). The logic lives in
  `Builder::process_overlays`.
- **Clickable scrim**: `Portal::dismiss(msg)` emits `msg` when the click lands
  **outside** the content of a `Center` modal. Implemented through a full-screen
  hit added **before** the content (so the content beats it where they overlap),
  exposed by `Widget::overlay_dismiss`.

## Batch B — Animated state transitions

- **Switch sliding**: the knob and the track colour interpolate between off and
  on. The generic mechanism: `Widget::anim_target() -> Option<f32>` declares the
  target; `Runtime::advance_values` drives a retained value (by identity) towards
  it and hands it back through `Status::value`. **No animation on mount** (the
  value adopts the target directly the first time the widget is seen).
- **Theme fade**: `Theme::lerp(other, t)` interpolates every token. The shell
  captures the outgoing theme when the theme is flipped and blends outgoing →
  target over ~0.25 s.

## Batch C — Back gesture (swipe), with a native feel

A drag from the left edge (`BACK_EDGE` px) pops a screen, the transition
following the finger **1:1**, then a physical settle.

```
Pressed (x < BACK_EDGE, stack non-empty) → Drag::Back ; BackGesture{ progress=0, velocity=0 }
CursorMoved (drag)                       → progress = (x − start)/width
                                           velocity = EMA of the finger's speed (fraction/s)
Released                                 → projected = progress + velocity·BACK_PROJECT
                                           settling = if projected > 0.5 { 1.0 } else { 0.0 }
Redraw (settling)                        → damped spring started from velocity:
                                           a = K·(target − p) − C·v ; v += a·dt ; p += v·dt
                                           at rest near the target → finish (pop if 1.0)
```

The three ingredients of the **native feel**:

1. **Velocity** — a quick *flick* commits even from halfway; a slow stop below
   the halfway point cancels. The decision is made on the **projected position**
   (position + momentum), as iOS does.
2. **Spring settle** — on release, the transition carries on with the **finger's
   momentum** as its initial velocity, through a near-critically-damped spring
   (`K=220`, `C=30`) → a soft arrival with no overshoot, rather than a linear
   ramp. **The same spring drives button navigation** (started at velocity 0 → an
   *ease-out*), so the motion is consistent everywhere (`spring_step`).
3. **Parallax + depth** (`Navigator`, shared with push/pop) — the back screen
   moves `NAV_PARALLAX=0.3×` slower, is rendered **behind** (corrected depth
   order) and is **darkened** in proportion to how much of it is covered.

The preview reuses the `Navigator` (pop: the current screen leaving to the right,
the screen below entering from the left), **without modifying the route stack**
until the gesture commits; the pop only happens at the end of the committing
settle.

## Tests

- `Theme::lerp` reaches its bounds and differs in the middle.
- `advance_values`: adopts the target on mount, animates on change, forgets
  widgets that have gone.
- A click on a `Center` modal's scrim returns the dismiss message.
- A pop halfway through: the back screen is parallaxed (offset compressed towards
  0) and rendered behind the outgoing screen.
- (The 24 earlier tests are kept → **28** in total.)

## Limits (v1)

- Auto-flip: simple flipping and nudging, no fine repositioning (corners).
- Back gesture: a fixed edge zone in physical px; it can overlap a control placed
  far to the left.
- The theme fade rebuilds a blended `Theme` per frame (cheap).
- Motion unified on `spring_step` (gesture + button); the theme fade and the
  hover/focus transitions keep their own ramp (not a spring).
