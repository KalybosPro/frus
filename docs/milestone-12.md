# Milestone 12 — Shadows, gradients, horizontal scrolling, scrollbar & drag, focus animations

A broad milestone, delivered as validated sub-steps.

## A. Shadows + gradients
- `Primitive::Rect` enriched: `color2` + `gradient_dir` (linear gradient) and
  `blur` (soft edge). `Scene::gradient_rect` and `Scene::shadow`.
- The fragment shader mixes `color`→`color2` along `dir`, and softens the edge
  over `blur` pixels (the shadow). `Container::gradient(end, dir)` and
  `Container::shadow(dx, dy, blur, color)`.

## B. Horizontal scrolling
- `Scroll::axis(Axis)` (`Vertical` / `Horizontal` / `Both`); the offset is now
  `(x, y)`. The content is laid out freely on the scrollable axis or axes
  (`Layout::compute_scroll`). Wheel + **Shift** = horizontal.

## C. Visible, draggable scrollbar
- Track + thumb drawn over the content (not clipped), with proportional size and
  position. `Ui::scrollbar_at`; the shell tracks the thumb **drag** (the first
  drag).

## D. Drag-selection in fields
- Click-drag extends the selection (`Widget::cursor_at` + anchor);
  **double-click** selects the word (`Widget::word_at`). Generic drag tracking
  (`Drag`).

## E. Focus animation
- `Runtime.anims` generalised (`Anim { hover, focus }`); `Runtime::advance`
  animates hover **and** focus. The `TextInput`'s border grows and colours in on
  focus (`Status.focus_progress`).

## Decisions & infrastructure

- **Drag**: a `Drag` state on the shell side (scrollbar | text selection), set on
  `MouseDown`, applied on `CursorMoved`, cleared on `MouseUp`. Reusable.
- **Clipping**: the bars are drawn with the *outer* clip (they are not cut off by
  the viewport's content).

## Tests

- `Runtime`: `advance` (hover) and `focus_animates_independently`.
- `TextInput`: `word_at_finds_word_bounds`.
- (Tests A–C are validated visually plus through the existing offscreen shader
  rendering, which did not regress.)

## Deferred (honestly)

- **Animated appearance/disappearance** (fade on mount/unmount): needs a
  reconciliation pass that **retains "outgoing" widgets** for the duration of the
  transition, plus an opacity propagated to every primitive (text included). That
  is a milestone in its own right; flagged as the riskiest one from the start.
- Multi-line selection, scroll inertia, physically blurred (Gaussian) shadows:
  out of scope for v1.
