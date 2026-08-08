# Ideas from mature UI toolkits, for frus

> Notes from reading a mature, full-featured retained-mode UI toolkit (~570 k lines of
> framework source), with frus's architecture in mind.
> The aim: **take what the prior art has proven** (the reconciliation engine, the layout
> protocol, the gesture arena, animation physics, scrolling, theme tokens), and **reject**
> what frus's Elm + taffy + wgpu architecture makes pointless (stateful widgets, global
> keys, inherited/ambient-dependency widgets, observable notifiers as app state).

---

## 0. The through line (read this first)

One principle comes back in **every** subsystem studied. It is the load-bearing
architectural decision; everything else follows from it:

> **The shell holds the state; the pure `view` only declares intent; everything drains
> out as `Msg`.**

Concretely:

- **State that lives across time** (the gesture arena, animation controllers, scroll
  offsets, focus, editing carets, retained reconciliation nodes, hit-test caches) lives in
  `frus-shell`, **outside** the `Box<dyn Widget>` tree, **keyed by `child_id`/`Keyed`**.
  frus already does this for hover/focus/edit/scroll in
  [`runtime.rs`](../crates/frus-widgets/src/runtime.rs) — the pattern needs generalising,
  not inventing.
- **The `view` carries declarative descriptors only**: `on_tap: Msg`,
  `on_drag: fn(Delta) -> Msg`, the stable identity, the intent to receive a given kind of
  recogniser. Never the state of the gesture or animation in flight.
- **Reconciliation by identity is the pivot.** The `view` is rebuilt every frame; the
  shell re-attaches the retained state to the new tree by `child_id`. The failure mode is
  always the same (already noted in CLAUDE.md): an inconsistent id walk breaks
  hover/focus/editing/animation/**a drag in flight** on reorder.
- **Phase discipline.** Split the frame into ordered, **independently invalidated** passes
  — `build → layout → paint → composite` — each drained from its own dirty list. A hover
  touches *paint* only; a text change, *layout+paint*; a theme change (frus themes at
  paint time), *paint* alone.

The rest of the document works that principle through subsystem by subsystem, with frus's
actual gaps in view.

---

## 1. Render pipeline & layout protocol
*(scope: the render-object protocol, box layout, the layer tree, the frame binding; the
scheduler and its ticker)*

### The protocol
- **Constraints down, sizes up.** The parent passes an immutable
  `BoxConstraints (min/max W/H)`; the child picks its `size` within them and reports it
  back. A parent only reads a child's size if it asked for it (`parentUsesSize`) — that
  boolean is what makes incremental relayout possible.
- **Relayout boundaries.** A node is a boundary iff
  `!parentUsesSize || sizedByParent || constraints.isTight || parent == None`.
  When it goes dirty it adds **itself** to the layout list; otherwise it walks up to the
  first boundary. A deep text edit relays out its own subtree, never the whole window.
- **Parent data.** A child's offset/flex live on `child.parentData`, not on the child →
  the child stays reusable and re-parentable (fits `Keyed`).

### Coexisting with taffy
Taffy **is** your `performLayout` for flex/grid: do not reimplement that maths. What to
take *on top of* taffy:
1. **A relayout-boundary cache**: per layout root, remember
   `(last_constraints, cached_size, needs_layout)` and **do not re-invoke taffy** if the
   constraints and the flag are unchanged. **The biggest layout win**, and it lives outside
   taffy.
2. **Intrinsic sizes** (`min/max intrinsic width`) routed into taffy's measure closure —
   for text and custom-painted content.
3. **Parent-data separation**, for cheap reordering.

### Repaint boundaries & the layer tree → wgpu
- A `RepaintBoundary` owns its own **retained draw batch**; when it goes dirty, only it
  re-records, the rest is **re-emitted by reference**.
- **The wgpu analogue for frus**: give repaint boundaries a cached fragment (a persisted
  quad list; or a **wgpu texture** for genuinely expensive content, the way a boundary can
  be rasterised to an image). The immediate textbook case: **a spring-driven bottom sheet
  sliding over static content** — cache the content as a texture, re-emit only the sheet's
  quads.
- The "compositing bits" (only allocate a real texture/pass when there is a
  clip/opacity<1/blur/transform) can be **deferred**: inline everything until there are
  real material layers.

### Ticker / vsync → driving animation
- The key discipline: **no frame is produced unless someone asks for one.** The ticker is a
  self-rescheduling one-shot: while an animation is active it asks for another frame; when
  it stops, back to idle → **0 CPU/GPU at rest**.
- **For frus (winit)**: `window.request_redraw()` is your vsync source. On each redraw,
  before the build, hand the **frame timestamp** to the active animators; then
  `if any_animation_active { request_redraw() }`. At rest, `ControlFlow::Wait`. Drive from
  the **frame timestamp**, not from a delta measured in the handler, and **clamp large
  deltas** (a backgrounded window) so springs do not explode. Cut off-screen animators (the
  Android lifecycle you already handle) — the `muted` idea.

### Rust translation
- **An arena / slotmap of nodes keyed by `Id`**, parent links as `Id`. `markNeedsLayout` /
  `markNeedsPaint` become "push an `Id` into a dirty `Vec`" — no aliasing, no mutable
  borrow walking upwards. This design fits Rust *better* than it fits a GC language.
- **No GC → explicit lifetimes** for cached wgpu textures/buffers: tie each batch/texture's
  life to the node's slot, free it on removal (an RAII `Drop` wrapper, safer than manual
  refcounting).
- **Separate phases enforce aliasing safety**: each pass takes the arena as an exclusive
  `&mut` → the type system does for free what the prior art checks with debug asserts.

### Do NOT copy
The three parallel widget/element/render trees with imperative `setState`; the flex/grid
maths (taffy); ambient-dependency widgets; the `markNeeds*` walk over a tree of mutable
parent pointers (→ arena/slotmap instead).

---

## 2. The widget/element model & foundations
*(scope: the widget framework, keys, change notifiers, diagnostics)*

### What to adopt
- **Two trees: immutable config (`Box<dyn Widget>`) + a retained node.** The *reuse*
  discipline is the heart of your rebuild pattern. frus's retained node does NOT need to be
  as fat as an element (which stores the user's `State`): under Elm the app owns the
  logical state, so the node keeps only the **ephemeral** part
  (hover/pressed/focus/caret/animation clocks/scroll offset). Call it a "retained paint
  node", not an "element".
- **`can_reuse = TypeId + Key`** (the analogue of `runtimeType + key`) plus the
  **three-phase list diff**: common prefix top→down, common suffix, then a `Key→node` map
  over the *keyed* middle. That is the proven way to make insert/remove/reorder O(n)
  **without losing the ephemeral state**. *Unkeyed* middle children that no longer line up
  positionally are destroyed (state lost) — hence the importance of keys.
- **Keys = `enum { Index(u32), Value(SmallKey), Unique(u64) }`, `Hash + Eq`.** Formalise
  `child_id`/`Keyed`. Keys are **the** fix for the "reordering loses state" bug. You
  already have `WidgetId::child`/`keyed` in
  [`interaction.rs`](../crates/frus-widgets/src/interaction.rs) — exactly the right
  foundation.
- **A diagnostic tree-dump trait**: `short()`, `props()`, `children()`, with an indented
  `dump_deep()`, behind `#[cfg(debug_assertions)]`. **A tiny investment, enormous debugging
  leverage** — especially for the identity/reordering class of bug you have already hit.
  Dump **both** trees (what `view` produced vs what was retained/reused); that side-by-side
  diff is the number one tool for debugging identity stability. `#[derive(Debug)]` is not a
  substitute.

### What to adapt
- **Ambient context** → a single immutable `Env` struct
  `{ theme, size_class, text_direction, insets/safe-area, scale }` passed **explicitly**
  downwards (into `paint`, and into `measure` if needed), with per-subtree shadowing
  (`Env::with_theme(...)`). That **replaces the whole ambient-dependency graph**. frus
  already themes at paint time → a theme change need not even re-run `view`, just repaint:
  that is *strictly simpler*, keep it.
- **A build context** → never a live handle into a mutable tree. A flat arena +
  `NodeId`: ancestor/child walks become index hops with no borrow conflicts.

### What to reject (anti-patterns under Elm)
Stateful widgets with their own `State`/lifecycle; **global keys** (cross-tree re-parenting
through a mutable global registry — a lifetime nightmare in Rust; under Elm the app moves
the state in `update`, and overlays/portals are modelled as a **separate top-level layer in
`view`**); observable notifiers as app state (a mutable observable graph is the antithesis
of a single source driven through `update`); dependant-based invalidation (stillborn, since
frus repaints rather than rebuilds on an ambient change).

> **Summary**: keep the prior art's *reconciliation engine* (config/retained split, the
> can-update rule, the keyed list diff, keys, diagnostics); throw away its *state and
> ambient-data machinery* — Elm already solves what those exist for.

---

## 3. Gestures — the arena (the crown jewel)
*(scope: the arena, the gesture binding, recognisers, tap/drag/long-press/scale, the
pointer router, hit testing, pointer events, velocity tracking)*

**frus today**: [`interaction.rs`](../crates/frus-widgets/src/interaction.rs) handles
click/press/hover/focus + text editing. No arena, no drag/long-press/scale disambiguation.
That is the biggest structural gap on the input side.

### The pipeline (4 decoupled pieces, held by a long-lived gesture binding)
1. **Hit test**: builds an ordered path of targets under the pointer, **innermost to
   outermost**, and **caches it by pointer id at down** (reused until up — correctness
   *and* performance: moves route to where the press began).
2. **Pointer router**: a subscription table per pointer id; recognisers subscribe to the
   raw stream. **Iterate over a copy** so a callback can unsubscribe mid-dispatch.
3. **The arena**: the disambiguator, one per pointer id.
4. **Recognisers** (tap/drag/long-press/scale): state machines that consume the stream,
   compete in the arena, and emit semantic callbacks.

### The arena, central rule
> *"The first member to accept, or the last one not to reject, wins."*

- **Model**: an **ordered** list of members (order = hit-test depth, innermost first) plus
  `open/held/pending_sweep` flags and an `eager_winner`.
- **Cycle**: `add` (while open) → `close` (the down has finished dispatching) →
  `resolve(accepted|rejected)`. Accepting while **closed** = wins immediately (everyone
  else gets `reject`); accepting while **open** = a pending `eager_winner`.
- **`sweep`** (on pointer-up): breaks the tie — **the first member in the list (the
  innermost) wins**, the rest are rejected. Stops a lifted finger from triggering nothing.
- **`hold`/`release`**: a recogniser that must survive the up (double-tap, a long-press
  timer) calls `hold` to neutralise the `sweep`, then `release` replays it.
- **Guarantee**: every member gets **exactly one** `accept` XOR `reject`.

### The canonical tap-vs-drag disambiguation
- **Tap accepts passively**: it waits to be the last one standing. The finger moves beyond
  the slop → immediate `reject`. The finger lifts in place → tap wins at the `sweep`.
- **Drag accepts eagerly**: as soon as the distance projected on its axis exceeds the slop
  → `accept`, which **evicts the tap**. Directional drags project the delta onto their axis
  → a horizontal drag coexists with a vertical scroll in the same arena.

### The pointer event model
`Down/Move/Up/Cancel` (+ hover/pan-zoom, deferred). Every event carries: `pointer` (id),
`position` (global, logical px), `delta`, `buttons` (a bitfield), `kind`, `timestamp`.
**`Cancel` is critical** (app backgrounded, gesture stolen): give up with no success
callback. **Velocity**: a ring buffer of the last ~100 ms; start with a weighted average,
move to a least-squares fit later for accurate flings.

### Rust translation (which solves the re-entrancy problem)
- **`Arena::resolve/close/sweep` are PURE**: they mutate the arena's maps and **return a
  `Vec<(MemberId, Disposition)>`** of callbacks to apply, instead of calling back
  re-entrantly. The shell drains that `Vec` once the borrow ends → an explicit loop instead
  of a call stack; this cleanly replaces scheduling a microtask.
- **`Copy` ids everywhere** (`MemberId`/`PointerId`/`RouteId`), never owned references.
  Recognisers are state machines returning `Option<GestureEvent>`, holding no reference
  into the app.
- **Timers** (tap-vs-long-press, hold/release): winit's `ControlFlow::WaitUntil` or a small
  timer wheel; on fire, inject a synthetic tick.

### Migration path (in tiers)
- **Tier 0 (the foundation, do this first)**: normalise winit input into `PointerEvent`
  (with an explicit `Cancel`); hit-test the taffy tree into a `Vec<HitEntry>` (innermost
  first) cached by id; a pointer router. **Not throwaway work.**
- **Tier 1 (an MVP without the full arena)**: one hard-coded "tap-or-drag" recogniser
  (down → *possible*; movement > slop before up → drag, suppress tap; up before slop →
  tap) plus a timer-based long-press. Covers ~90 % of the need. **Make it speak the
  arena's vocabulary already** ("accepts eagerly on slop crossing" / "accepts passively on
  up") so moving to tier 2 is a substitution, not a rewrite.
- **Tier 2 (the real arena)**: as soon as there are independently scrollable nested
  regions, or a draggable inside a scrollable inside a tappable card. Port the arena almost
  verbatim (**pure versions returning outcomes**) plus a primary-pointer recogniser base
  (deadline+slop, reused by tap AND long-press).
- **Tier 3 (deferred)**: least-squares velocity, scale/rotation (pinch-zoom), teams,
  multi-drag, the wheel, resampling.

---

## 4. Animation & physics
*(scope: the animation value/controller/tween/curve family; simulations — spring, friction,
clamped)*

**frus today**: springs hard-coded per widget (`spring_step`, `scroll_axis` in
[`runtime.rs`](../crates/frus-widgets/src/runtime.rs)). It works but does not generalise.
The prior art offers one very portable abstraction.

### The core abstraction
- **`Animation<f64>`** = an observable value in `[0,1]` + a `status`
  (`dismissed/forward/reverse/completed`). The producer (the ticker), the consumer (the
  widget) and the shaper (tween/curve) are **decoupled**.
- **`AnimationController`** *is* an `Animation<f64>`: it owns a `Ticker` and, above all,
  **everything it does is expressed as a `Simulation`** (a pure function `x(t)`).
  `forward`/`animate_to` → a simulation interpolating a curve over a duration;
  `fling`/`animate_with` → a physical simulation (spring/friction). **One tick loop for
  everything.** That is the most portable idea here: *one driver, pluggable time→value
  functions*.

### Tween + Curve
- **`Tween<T>` / `Animatable<T>`**: one method, `transform(t) -> T`. A single `[0,1]`
  controller drives arbitrary typed values (Color, Rect, Offset, opacity…).
- **`Curve.transform(t)`** maps `[0,1]→[0,1]`. Port: `Linear`, `Cubic` (Bézier by binary
  search → gives every `easeInOut` preset as constants), and above all
  **`Interval(begin,end,curve)`**, which unlocks **staggered animations for free**: several
  values, one controller, each on a sub-window.

### Simulations — maths that ports directly
The interface: `{ x(t), dx(t), is_done(t), tolerance }` (default 1e-3).

**Spring** — `SpringDescription { mass m, stiffness k, damping c }`;
`with_damping_ratio`: `c = ratio · 2·√(m·k)`. With `x₀ = start − end` and `v₀` the initial
velocity, the reported position is `end + sol.x(t)`. The case is chosen by the
discriminant `c² − 4mk`:

```
// Critically damped (c² − 4mk == 0)
r = −c/(2m); c1 = x₀; c2 = v₀ − r·x₀
x(t)  = (c1 + c2·t)·e^(r·t)
dx(t) = r·(c1 + c2·t)·e^(r·t) + c2·e^(r·t)

// Overdamped (c² − 4mk > 0)
cmk = c² − 4mk; r1 = (−c − √cmk)/(2m); r2 = (−c + √cmk)/(2m)
c2 = (v₀ − r1·x₀)/(r2 − r1); c1 = x₀ − c2
x(t)  = c1·e^(r1·t) + c2·e^(r2·t)
dx(t) = c1·r1·e^(r1·t) + c2·r2·e^(r2·t)

// Underdamped (c² − 4mk < 0, oscillates)
w = √(4mk − c²)/(2m); r = −c/(2m); c1 = x₀; c2 = (v₀ − r·x₀)/w
x(t)  = e^(r·t)·(c1·cos(w·t) + c2·sin(w·t))
dx(t) = e^(r·t)·(c2·w·cos(w·t) − c1·w·sin(w·t)) + r·e^(r·t)·(c2·sin(w·t) + c1·cos(w·t))
```
`is_done = near_zero(x, tol.dist) && near_zero(dx, tol.vel)`. `fling` uses a critically
damped spring (`ratio 1`, `stiffness 500`) that stops on position alone.

**Friction** (scroll/fling momentum), `drag ∈ (0,1)`:
```
dx(t) = v₀ · drag^t
x(t)  = x₀ + (v₀/ln(drag))·(drag^t − 1)
finalX (t→∞) = x₀ − v₀/ln(drag)
is_done = |dx(t)| < tol.vel
// through(x0,x1,v0,v1): drag = e^((v₀−v₁)/(x₀−x₁))
```
`ClampedSimulation` pins the position into a range (velocity keeps reporting) — for scroll.

### Implicit vs explicit = the same machine
Implicit animations (the animated-container style) are a *hidden* controller that, on each
rebuild, diffs the target props and re-targets a `Tween (begin = current value, end = new
target)` over `duration+curve`. **You get the implicit form for free the moment you have
the explicit one plus a diff-and-retarget helper.**

### Elm mapping — recommendation
Two options; **recommended: B (a retained runtime beside the pure view).**
- **A — animations as subscriptions emitting tick `Msg`s (pure)**: the view stays pure, but
  60–120 Hz of `Msg` cross `update`, and interruption/re-targeting (drag→fling) forces the
  simulation objects into the immutable model → pain.
- **B — recommended**: a small **imperative registry of controllers in the shell**, keyed
  by identity (`child_id`/`Keyed`). Created and driven by `Command`
  (`Command::animate(id, spec)`, `Command::fling(id, v)`). **The view reads the current
  value at paint time** (`ctx.animation(id).value`) — consistent with "widgets theme
  themselves at paint time". **Only** status transitions (started/finished) go back through
  `update` as a `Msg`; per-frame value changes just trigger a repaint.
  You already have a spring-driven sheet and a frame/command loop → you have *proven* you
  need retained animation state; formalise it.

### Ranked recommendations
1. **Port the physics now** (pure maths, zero coupling): `trait Simulation`,
   `SpringSimulation` (3 cases), `FrictionSimulation` (closed form; skip Newton /
   `constantDeceleration`), `ClampedSimulation`. **Generalise your bottom-sheet spring into
   `trait Simulation`** so fling, scroll momentum and the sheet share one path.
2. **A minimal driver**: `AnimationController` = a bounded value + a status +
   `Box<dyn Simulation>` + `tick(elapsed)` (~5 lines). `forward/reverse/animate_to/fling`.
3. **`Curve` + `Tween`/`Animatable`**: `Linear`, `Cubic`, `Interval` (staggered for free).
4. **The implicit form** as a thin helper once the explicit one is in place.
5. Wire it all up as a **`Command`-driven registry keyed by `child_id`**, with values read
   at paint time.

**Deferred**: splines / 2-D curves, elastic/bounce curves, Newton-based desktop friction,
an "animations disabled" scale factor, a completion future (a completion `Msg` is enough).

---

## 5. Painting & theme
*(scope: box decoration, borders, shadows, edge insets, alignment, border radius,
gradients, text style/spans/painting, colours; theme data, colour scheme, text theme)*

**frus today**: `Scene` has only **2 primitives, `Rect` and `Text`**
([`scene.rs`](../crates/frus-core/src/scene.rs)); `Theme` is a **flat bag of 11 colours +
radius + spacing**, dark/light + lerp
([`theme.rs`](../crates/frus-widgets/src/theme.rs)). Two work items: enrich the scene
primitives, and structure the theme into roles and scales.

### Painting vocabulary → core Rust types (in `frus-core`, sRGB, logical px)
- **`EdgeInsets` {left,top,right,bottom}** plus an `EdgeInsetsDirectional
  {start,top,end,bottom}` variant with **`.resolve(dir) -> EdgeInsets`** (RTL swaps
  start/end — the whole RTL story lives there). Helpers: `all`, `symmetric`,
  `deflate_rect`, `inflate_size`.
- **`Alignment {x,y}`**, a fraction in `[-1,1]²` (`(0,0)` = centre) + `within_rect(r)`,
  `inscribe`.
- **`Radius {x,y}`** (elliptical!) + **`BorderRadius`** (4 corners). `to_rrect` **clamps
  negative radii to 0** before rendering — do the same at the GPU edge.
- **`BorderSide {color,width,style,stroke_align}`**: the load-bearing concept is
  **`stroke_align` ∈ [-1 inside, 0 centred, 1 outside]** (it decides whether the stroke
  eats into the content, and how to inset the fill to avoid anti-aliasing bleed). Uniform
  borders = one `rrect_stroke`; non-uniform = 4 trapezoids (defer).
- **`BoxShadow {color,offset,blur_radius,spread_radius}`**; `sigma ≈ 0.57735·blur + 0.5`.
  Implement an **"analytic blurred rrect" scene primitive** in `frus-gpu` (a shader, not a
  real Gaussian pass).
- **`Gradient`** enum (Linear/Radial/Sweep) unified by `colors + stops? (uniform if None) +
  tile_mode`, with **fractional anchors** (`Alignment`) → size-independent, pixellated only
  in `create_shader(rect)`.
- **`Color`**: add `with_alpha(f32)`, `lerp`, `compute_luminance()` (WCAG, on linearised
  channels), `from_argb_u32`.

### The decoration model (the paintable keystone)
`BoxDecoration { color?, gradient?, image?, border?, border_radius, shadows, shape }` with a
**fixed paint order: shadows → fill (colour/gradient) → image → border**. For now, *lower*
`BoxDecoration → Vec<Primitive>` at paint time (no retained painter), but keep the "the
gradient shader depends on the rect" cache. `content_padding()` (= the border's dimensions)
**feeds taffy** so that a bordered container reserves the room. Defer shape decorations /
shape borders (stadium, superellipse) and decoration images / image providers.

### Text
- **`TextStyle`** (color, font, size, weight, italic, letter/word spacing, `height` = a
  multiple, decoration, shadows) with **`merge()` + `inherit`** = the cascade
  (span > default text style > theme). **`TextSpan`** = a `{text?, style?, children}` tree
  → flatten into `(byte_range, Attrs)` runs for cosmic-text.
- **`TextLayout`**, thin over cosmic-text, exposing: `size`, `min/max_intrinsic_width`
  (lay out at `∞` for max, `0` for min → feeds taffy), `hit_test(p) -> TextPosition`,
  `caret_rect(pos)`, `selection_rects(range)`, `line_metrics()`. cosmic-text already
  provides shaping/line breaking/hit testing/caret; the work is resolving the
  `TextStyle → Attrs` cascade and exposing intrinsics plus caret/selection.

### Theme architecture (Material 3)
- **`ColorScheme` = semantic roles derived from a seed**: `from_seed(seed, brightness)`
  produces ~30 **roles** (`primary/onPrimary/primaryContainer/…`,
  `surface/onSurface/onSurfaceVariant/surfaceContainer*`, `outline/outlineVariant`,
  `error/…`, `shadow/scrim/inverseSurface/surfaceTint`). Widgets reference **roles**, never
  literal colours → swapping the theme recolours everything and guarantees contrast (the
  `X`/`onX` pairs).
- **`TextTheme` = a named type scale**: 15 slots (`displayLarge…labelSmall`). Widgets pick a
  slot, not a hard-coded size.
- **The recommended `Theme`** (in `frus-core` or a `frus-theme`, so widgets can theme
  themselves without pulling in the shell):
  `{ brightness, color: ColorScheme, text: TextTheme, shape: ShapeScheme,
  elevation: ElevationScheme, spacing: SpacingScale }`.
- **Ship a hand-written light/dark ColorScheme first**; add `from_seed` (the HCT algorithm
  from the Material colour utilities, portable to Rust) later.
- Since frus themes at paint time, `paint(theme, status)` resolves role→colour according to
  the `Status` (Default/Hover/Press/Disabled/Focus) and applies a **state layer** (an `onX`
  overlay at 8 %/12 %) — **bake that rule into the theme** so widgets stay declarative.

### sRGB / linear / premultiplied alpha (the GPU edge) — pitfalls
- Author in **sRGB**, convert to linear **only at the GPU edge**, with the real curve
  (`x≤0.04045 ? x/12.92 : ((x+0.055)/1.055)^2.4`), **not** `pow(2.2)`.
- **Interpolation**: discrete UI colours → lerp in gamma space (as CSS does); gradient stops
  on the GPU → better in linear. Pick one and **be consistent** (stable goldens).
- **Premultiplied alpha**: wgpu expects premultiplied (`rgb *= a`) **after** the conversion
  to linear; line the blend state up (`PREMULTIPLIED_ALPHA_BLENDING`) and agree with
  glyphon/images, or you get fringing.
- **Surface format**: an `*Srgb` target → the shader outputs **linear** (the hardware
  encodes); an `Rgba8Unorm` target → encode it yourself. **One `frus-gpu` convention,
  documented and centralised** — this is the number one source of "washed-out / too dark"
  colours.

### Types to define, by priority
1. `Color` helpers + **the sRGB↔linear + premultiplied convention locked down in
   `frus-gpu`** (everything depends on it).
2. `EdgeInsets` (+ directional) & `Alignment`.
3. `Radius`/`BorderRadius` (+ the negative clamp).
4. `BoxShadow` + a blurred-rrect scene primitive.
5. **`BoxDecoration`** (fixed order + `content_padding`→taffy) — the keystone.
6. `BorderSide`/`Border` (uniform first).
7. `Gradient` (Linear first).
8. `TextStyle` (+ merge) & `TextSpan`.
9. `TextLayout` over cosmic-text.
10. **`Theme` = ColorScheme (M3 roles) + TextTheme (15 slots) + Shape + Elevation +
    Spacing**; light/dark by hand, `from_seed` afterwards; the state layer baked in.

---

## 6. Platform, focus, scrolling, navigation & a11y
*(scope: focus management and traversal, scroll position/physics/activity/controller, the
scrollable and its viewport, overlays, the navigator, media queries and safe areas;
hardware keyboard, text input, platform channels; semantics)*

### Focus
A focus-node tree parallel to the widget tree (stable identity → `child_id`), one
`primary_focus` in the shell, **key dispatch from leaf to root** returning a three-state
result (`handled/ignored/skipRemaining`; `ignored` keeps bubbling). A reading-order /
geometric traversal policy for Tab and the arrow keys. **A focus highlight mode
`{traditional, touch}`**: only paint the focus ring if the last interaction was from the
keyboard. Scopes (trapping focus inside dialogs/sheets) can wait.

### Keyboard & input — two independent paths (keep them separate)
- **(a) Hardware keys**: a regularised `KeyDown/Up/Repeat` model carrying a **physical key**
  (HID position, layout-independent) **+ a logical key** (the meaning under the layout)
  **+ a character?**, with a tracker for pressed keys and modifiers. It feeds focus. winit
  provides all of this on desktop.
- **(b) Text/IME**: composed text does **not** come through key events. A "client" owns a
  `TextEditingValue { text, selection, composing }` — **the `composing` region (the IME's
  provisional text) is essential for on-screen keyboards and CJK**. The shell owns the one
  active connection. Desktop: winit's `Ime::Preedit/Commit` → `composing`. **Android: the
  soft keyboard produces no hardware keys at all**; it needs a text-input control on top of
  the FFI (next section) exposing an input connection, and it must **drive `viewInsets`** so
  the content rises above the keyboard.

### Scrolling — the subsystem to copy most carefully
Split into 4 single-responsibility pieces:
- **`ScrollPosition`**: the state — `pixels`, `min/maxScrollExtent`, `viewportDimension`;
  observable; **it starts no motion itself**, it delegates to the *activity*.
- **`ScrollController`**: the façade the app holds — `offset`, `animate_to`, `jump_to`.
- **`ScrollPhysics`**: a composable chain — `apply_physics_to_user_offset` (resistance),
  `apply_boundary_conditions` (clamp at the edges), **`create_ballistic_simulation(metrics,
  velocity) -> Option<Simulation>`** (fling momentum: it returns a spring/friction that the
  ticker samples). `Clamping` (Android) / `Bouncing` (iOS) variants.
- **`ScrollActivity`**: the "how it moves" machine — `Idle/Hold/Drag/Ballistic/Driven`.
- **Adapting to taffy**: taffy lays out **all** the content once →
  `maxScrollExtent = contentSize − viewportDimension`. A frus viewport = **a clip plus a
  translation of the children by `-pixels`**. **No lazy slivers needed for v1** (accept the
  full-layout cost, optimise afterwards). Drive the ballistic activity from your winit
  ticker loop. It fits Elm: `pixels` as retained state, the activity/ticker emitting `Msg`.
  You already have a scroll spring in `runtime.rs` — that is the seed of the physics.

### Overlay, navigator & ambient insets
- **An `Overlay`** = a stack of independent floating layers (`OverlayEntry`, `opaque`,
  `maintainState`) — the substrate for everything "above" (dialogs, sheets, tooltips, drag
  feedback). **A portal entry** anchors an overlay child to a widget's position — directly
  relevant to your bottom sheet.
- **A `Navigator`** = a stack of `Route`s that **push `OverlayEntry`s**; a modal route adds
  a barrier and a focus scope. → **Recast the bottom sheet/modal as an overlay entry + a
  focus scope + a scrim**; a lightweight route stack can come afterwards.
- **Ambient insets**: distinguish **`padding`** (notch/bars — static) from **`viewInsets`**
  (the keyboard — dynamic). A `SafeArea` **consumes then zeroes** them for its descendants
  (`removePadding`) → a SafeArea inside a SafeArea does not pad twice. You already have
  `on_insets`/SafeArea (milestone 51); formalise those two values and the consume-then-zero
  rule, and feed `viewInsets.bottom` from the Android keyboard height.

### The native boundary (channels)
The prior art uses an async bidirectional binary messenger under typed façades (method and
event channels), with centralised named channels (`textinput`, `keyevent`, `platform`,
`lifecycle`, `system`). **For frus**: on desktop, winit *is* the boundary (wire directly).
**Android** is a JNI/FFI seam (you already have `android_main`). Do not build a dynamic
string+codec bus: define **typed Rust enums crossing the FFI, one per subject** (text input,
insets/window/lifecycle, system = clipboard/haptics/orientation/back). Steal the
*discipline*: **one narrow, async, bidirectional boundary**, with app→native and native→app
shapes.

### Semantics / accessibility (plan it, do not over-build)
The prior art builds a **parallel semantics tree** (label/value/hint/flags/role/actions),
batched then flushed to the platform's accessibility node API. **The frus minimum**: an
optional per-widget annotation (`role`, `label`, `value`, `flags`, rect, a few `actions`:
activate/scroll/focus), a flat tree keyed by identity, bridged to Android through a virtual
view provider over the FFI. Reuse the focus tree for reading order at first. **Bake the
`label` hook into the widgets right now** to avoid a massive retrofit.

### Recommended build order (§6)
1. **The focus tree + key routing** (a prerequisite for everything).
2. **The regularised keyboard model** (winit → physical+logical+character).
3. **Scrolling** (the 4 pieces + a clip+translate viewport; clamping first).
4. **Insets: split `padding`/`viewInsets` + consume-then-zero** (a small SafeArea refactor,
   unblocks keyboard avoidance).
5. **An overlay layer for modals/sheets** (recast the bottom sheet).
6. **Input/IME** (desktop winit first, then Android via the FFI + `composing`).
7. **Typed FFI channels for Android**.
8. **Semantics hooks** (annotations now, the tree + the Android bridge last).

---

## 7. Proposed roadmap (the next milestones)

Crossing the 6 briefs with frus's actual state (61 widgets breadth-first, a 2-primitive
Scene, a flat theme, per-widget springs, no generic arena/focus/scroll), here is a
high-leverage order. frus is **broad but shallow**: these milestones add the *engine depth*
missing under the widget shop window.

**Block A — engine foundations (the biggest leverage)**
1. **Frame phases + separate dirty lists** (`build→layout→paint→composite`), each
   `Msg`/`Command` setting the narrowest possible bit. (§1, §0)
2. **A relayout-boundary cache on top of taffy** — `(constraints, size, dirty)`. (§1)
3. **A self-rescheduling animation driver** tied to `request_redraw`, driven by the frame
   timestamp, back to idle at rest; **generalise the bottom-sheet spring into `trait
   Simulation`**. (§1, §4)

**Block B — input & motion**
4. **Gesture tier 0**: a normalised `PointerEvent` (+`Cancel`), a taffy hit test cached by
   id, a pointer router. (§3)
5. **Gesture tier 1**: a tap-or-drag recogniser plus long-press, already speaking the
   arena's vocabulary → `on_tap/on_drag/on_long_press` emitting `Msg`. (§3)
6. **Scrolling**: `ScrollPosition/Controller/Physics/Activity` + a clip+translate viewport
   over taffy, fling driven by the ticker. (§6)

**Block C — design system & text**
7. **Core painting types**: `EdgeInsets`/`Alignment`/`BorderRadius`/`BoxShadow` +
   **scene primitives** for rounded rects/shadows/gradients in `frus-gpu` (lock the
   sRGB/premultiplied convention down). (§5)
8. **`BoxDecoration`** (fixed order, `content_padding`→taffy). (§5)
9. **A structured theme**: `ColorScheme` (M3 roles, light/dark by hand) + `TextTheme`
   (15 slots) + Shape/Elevation/Spacing + a baked-in state layer. Migrate the 61 widgets
   to the roles progressively. (§5)
10. **`TextLayout`** over cosmic-text (intrinsics→taffy, caret, selection). (§5)

**Block D — structure & finishing**
11. **Focus + a regularised keyboard** (leaf→root, three states, highlight mode). (§6)
12. **`padding`/`viewInsets` + consume-then-zero**; an **Overlay** for modals (recast the
    bottom sheet). (§6)
13. **Formalised keys** (`enum {Index,Value,Unique}`) + a **diagnostic dump** of both trees
    (config vs retained). (§2)
14. **Input/IME** (desktop then Android FFI), **typed FFI channels**, **semantics hooks**
    (labels right now). (§6)

**Tier 2+ / deferred**: the real gesture arena (nested scrollables), least-squares
velocity, scale/pinch, lazy slivers, HCT `from_seed`, shape borders/images, compositing
bits, a full semantics tree.

---

# PART II — What is missing to win the market

> Part I (§0–§7) covers the **engine**: it makes sure frus is not architecturally capped.
> But an excellent engine does not win a market. What wins is **developer ergonomics**,
> **reach** (platforms), **tooling**, **the default design**, and **clear positioning
> against the real competitors** — which are **not** the big cross-platform toolkits, but
> the Rust UI ecosystem (iced, egui, Slint, Dioxus, gpui, Xilem…). This part covers all of
> that.
>
> The through line of Part II: **a Rust developer must feel at home from minute 1** —
> cargo, types, exhaustive messages, zero fights with the borrow checker, bearable compile
> times, and a beautiful default with no configuration.

---

## 8. Rust ergonomics — the core (Rust developers must feel at home)

This is **the** priority. A Rust developer judges a framework in 10 minutes on: "does it
compile fast, does the API read well, am I fighting the borrow checker, are the errors
clear". Concrete decisions:

### API: builders that read well + *optional* macros
- **Avoid `Box::new(...)` everywhere.** Provide free functions returning
  `impl Widget<Msg>` and chainable builder methods. The sweet spot (iced's):
  ```rust
  column![
      text("Hello").size(24),
      button("Save").on_press(Msg::Save),
      row![checkbox("Enable", cfg.enabled).on_toggle(Msg::Toggle)].spacing(8),
  ].spacing(12).padding(16)
  ```
  The `row!`/`column!`/`stack!` macros only wrap `vec![...]`, handling the `Box`/`into()`
  — **never** a magic DSL that breaks rust-analyzer. A developer must be able to write it
  all in plain Rust *without* macros if they prefer.
- **Implicit `into()`**: `impl From<&str> for Text`, `impl From<T: Widget> for Element`, so
  children can be written without ceremony.
- **`#[must_use]`** on widgets/commands, **newtypes** throughout (`Spacing(f32)`,
  `Radius(f32)`) — Rust developers love a type that states the intent.

### The `Widget<Msg>` generic: composition through `.map()` is **the** vital point
This is Elm-in-Rust's number one friction and **the** unlock for scalability. A
sub-component emits its own `ChildMsg`; the parent remaps it:
```rust
child.view().map(Msg::Child)   // Widget<ChildMsg> -> Widget<Msg>
```
`Element::map` must be **first-class, perceived as zero-cost, and documented on page 1**.
Without it, everything ends up in one unmanageable "god enum" `Msg`. With it, frus composes
like iced. Check that `map` also traverses `on_edit`/gestures/subscriptions, not just
clicks.

### Zero fights with the borrow checker (a by-value design)
- **The `view` takes `&State` and returns an owned value**; widgets are `Copy`/`Clone`
  friendly or built on the fly. Never an API that forces `Rc<RefCell<>>` on the user.
- **`paint(&self, …)`**, `Copy` ids, retained state in the shell (already your model) → the
  user never holds a mutable reference into a tree. That is *already* the right call;
  protect it as an API invariant.
- **Messages = `enum` + `#[derive(Clone, Debug)]`**; the exhaustive `match` in `update` is a
  **safety net Rust developers love** — the compiler forbids forgetting a case.

### Bearable compile times (otherwise you lose before you start)
- **Crate splitting** (you already have it: core/gpu/layout/text/widgets/shell) → targeted
  incremental recompilation. Keep `frus-demo` **thin**.
- **Feature flags per widget family** (`features = ["forms", "nav", "data"]`): a developer
  compiles only what they use → smaller binary *and* shorter compiles.
- **A dynamically-linked dev mode** (Bevy's `dynamic_linking` style): a feature that loads
  `frus` as a `.so`/`.dll` to cut link time in debug.
- Document using the **Cranelift backend** in debug and `lld`/`mold` as the linker —
  immediate compile-time wins, and much appreciated.

### rust-analyzer & discoverability
- **Concrete, named types**, not opaque `impl Trait` everywhere in the critical public
  signatures (autocompletion and error messages suffer).
- **Runnable doc-tests** and `examples/` launchable with `cargo run --example gallery`. A
  Rust developer learns from a compilable example, not from prose.

---

## 9. Composition & Elm at scale (killing the boilerplate)

Elm is simple on a small app and **verbose** on a large one unless you supply the tools.
What to provide *as a framework* (rather than letting every developer reinvent it):

- **A documented, tooled "component" pattern**: `{ Model, Msg, update, view }` per
  sub-part, plugged into the parent through `child.update(msg)` +
  `child.view().map(Msg::Child)`. Provide a **canonical example** (a three-screen app)
  showing how Msg and Command are remapped — that is the reference developers will copy.
- **Message namespacing**: `enum Msg { Header(header::Msg), List(list::Msg), … }` instead of
  a flat 200-variant enum.
- **Memoisation (`lazy`/`memo`)**: Elm's performance Achilles' heel is that `view` is
  rebuilt entirely every frame. Provide a `lazy(deps, || build_subtree())` (like
  `iced::widget::lazy`) that **only rebuilds a subtree when `deps` changes**; combined with
  your `child_id` reconciliation, that bounds the rebuild cost. **Plan it early** — it is
  what keeps large lists/tables (you already have `table`, `tree`, `list`) tractable.
- **Acknowledge the "signals" tension**: Dioxus/Leptos/floem/Xilem are moving towards
  fine-grained reactivity (signals) for performance and ergonomics. frus chose Elm — **own
  it** (predictable debugging, a single source of truth, trivial tests) and compensate on
  performance with `lazy` + reconciliation, rather than hybridising. Record it as an
  explicit choice, not an oversight.

---

## 10. Async, effects & `Command` (indispensable for real apps)

A real app does IO: network, disk, timers, subprocesses. Without a clean effect model, frus
stays a demo. The Elm/iced model:

- **`Command<Msg>` embeds the future**: `Command::perform(future, |result| Msg::…)` runs a
  `Future` **off the UI thread** and injects a `Msg` back with the result. It is the only
  sanctioned bridge from the impure world into `update`.
- **A pluggable async runtime**: do not marry tokio. An `Executor` trait (implemented for
  tokio / async-std / smol / a homemade pool) that the developer picks at
  `run(app, settings)`. Many Rust developers *already* have a runtime; do not impose a
  second one.
- **`Subscription<Msg>`** for long-lived streams: websockets, clock ticks, system events,
  file watching. Declared in `subscription(state)`, diffed by identity (started/stopped as
  they appear/disappear) — exactly iced's model, and it *also* serves your animation driver
  (§4).
- **Cancellation & batching**: `Command::batch([...])`, plus a `Command` bound to a
  cancellable `id` (a request you abandon when the screen changes). Without cancellation,
  apps leak tasks.
- **The golden rule**: `update` stays **pure and synchronous**; everything impure lives in
  the Commands/Subscriptions the shell runs. That is what keeps `update` testable (§13) —
  an Elm advantage you must never sacrifice.

---

## 11. Reach = market: desktop, mobile, **web**, embedded

Reach is the addressable market. Every missing platform is a lost one.

- **Web (wasm) — the biggest differentiator.** wgpu targets **WebGPU with a WebGL2
  fallback**. A frus `view` running in the browser *without a rewrite* is a massive
  argument (it is what made Dioxus/Leptos take off). winit supports the web canvas. High
  priority: it multiplies the audience and makes demos shareable by URL (adoption).
- **iOS**: you **already have Android** (rare and valuable — iced/egui are weak there).
  Adding iOS makes frus **the** "beautiful desktop + mobile" Rust framework, a nearly empty
  niche.
- **Embedding inside an existing app**: render into a winit sub-window *or* render
  **offscreen into a texture** the host composites. That opens the "a frus view inside a
  Qt/native/game app" market.
- **Multi-window**: several `Window`s, one `Application` — necessary for real desktop tools
  (palettes, inspectors).
- **Embedded/`no_std`**: probably out of scope, but **that is where Slint wins**
  (microcontrollers, industrial HMIs). Decide consciously not to go there — or to go there
  as a distinct line of attack.

Every target shares the same `view`/`update`: reach is mostly work in **frus-shell** (the
winit/platform seam) and `frus-gpu` (the wgpu backends), not in application code. That is a
structural advantage — capitalise on it in the messaging.

---

## 12. Performance engineering (beyond the §1 pipeline)

The phase pipeline (§1) gives you the *when to recompute*. Here is the *how to draw fast* —
what makes for perceived smoothness and "starts and scrolls without jank":

- **GPU batching / minimising draw calls.** Group quads into **a single instanced buffer**
  per pipeline (rounded rects, shadows, glyphs). A typical UI should fit in a handful of
  draw calls, not one per widget. Your scene primitives (§5) must be designed for
  instancing from the start.
- **A glyph atlas.** glyphon already manages one; make sure you **do not re-shape**
  unchanged text (cache by `(text, style, width)` → `TextLayout`). Text is often the number
  one CPU cost.
- **Damage regions (scissor).** Coupled with repaint boundaries (§1): re-render only the
  **dirty rect** through a wgpu scissor rect. A blinking caret must not redraw the screen.
- **Zero allocation per frame** on the hot path: a bump arena reset every frame for the
  `view` tree (against the Elm churn noted in goal 2), reused GPU buffers.
- **An explicit frame budget** (16.6 ms @60 Hz, 8.3 ms @120 Hz) and **built-in profiling**
  (`tracy`/`puffin` behind a feature). A framework that wins *measures* itself.
- **A reproducible benchmark harness**: **offscreen rendering + pixel readback** (you
  already have that pattern on the WSL/llvmpipe side) → perf goldens and non-regression in
  CI.

---

## 13. Tests, tooling & hot reload — the DX that wins

At equal engines, this is often what *decides* adoption.

- **A pure `update` = trivial unit tests** (a massive Elm advantage):
  `assert_eq!(update(state, Msg::Increment).0, expected)` with no GPU and no window.
  **Lead with this**; it is an argument neither egui nor gpui has as cleanly.
- **Headless render tests (golden/snapshot)**: render offscreen → compare against a
  reference image, with a pixel tolerance. You already have the WSL/offscreen
  infrastructure — package it as `frus-test`.
- **Hot reload**: the identified weakness (goal 4). Two levers, to be combined:
  1. **State-preserving reload** — Elm makes this *easier* than anywhere else: the state is
     **a single struct**; serialise it (serde), reload the library, rehydrate.
     Look at `hot-lib-reloader` and especially **`subsecond`** (Dioxus's Rust hot patching)
     — that is the state of the art and it works with plain Rust.
  2. **Live preview** of the `view` (Slint style): a mode where editing `view` reloads
     without a restart. Combined with (1), you close in on best-in-class iteration speed.
- **A runtime inspector**: expose the **diagnostic dump** (§2) as an overlay (tree + rects +
  ids + retained state). A developer who can *see* why their identity breaks on reorder
  stays. Little code, enormous return.
- **Cargo-native**: `cargo new --template frus-app`, `cargo run`, `cargo test` all work with
  no external tool. Rust developers do not want a proprietary CLI (the sin of some
  competitors). Stay inside cargo as much as possible.

---

## 14. i18n / l10n / RTL / accessibility (in depth)

- **RTL & bidi**: the directional base is planned (§5, `EdgeInsetsDirectional::resolve`).
  Extend it to **layout mirroring** (a row reversed in RTL) and **bidi text** (cosmic-text
  handles it — expose it). A `TextDirection` in the `Env` (§2), propagated to paint.
- **Localisation**: integrate **Fluent** (`fluent-rs`, Mozilla's Rust i18n standard) for
  messages, plus per-locale number/date/plural formats. Do not reinvent it.
- **Accessibility: adopt AccessKit — do NOT reinvent it.** AccessKit is the cross-platform
  Rust a11y standard (UIA on Windows, AT-SPI on Linux, macOS, plus Android/web providers);
  **egui, Slint, Xilem and Bevy already use it**. You map your per-widget semantic
  annotation (§6: role/label/value/actions) onto the AccessKit tree, and it talks to the
  native screen readers. That is *the* shortcut to credible a11y and a **compliance
  argument** (the enterprise/public-sector market). Bake the `label` hook into the widgets
  **right now** (§6); wire AccessKit up afterwards.

---

## 15. Distribution & packaging (the last stretch before the user)

- **A single binary, no runtime to install** (against Electron/Tauri-webview) — a selling
  point: "a self-contained `.exe`/`.app` of a few MB". Accept the wgpu weight (goal 3), but
  stay well under the mainstream cross-platform toolkits.
- **Per-platform bundling**: `cargo-apk` (already, for Android), `cargo-bundle`/`cargo-dist`
  for `.app`/`.msi`/`.deb`/AppImage, wasm-bindgen + trunk for the web. Document one command
  per target.
- **Embedded assets** (fonts, images, i18n) through `include_bytes!`/`rust-embed` → **zero
  external files**, deterministic startup (and it serves goal 3: never scan the system
  fonts).
- **Size**: `opt-level="z"`/`s`, `lto=true`, `strip=true`, `panic="abort"` in release;
  document a "minimal" profile. Publish the numbers (a "hello world" at N MB) —
  transparency about size reassures.

---

## 16. Positioning: how frus wins the Rust UI market

**The trap**: aiming at the big cross-platform toolkits. frus's addressable market is the
**Rust** developers choosing a UI toolkit *today*. The real competitors:

| Framework | Model | Rendering | Strengths | Weaknesses (frus's opening) |
|---|---|---|---|---|
| **iced** | **Elm** (the closest!) | wgpu | Mature, clean, cross-desktop | Austere, **weak on mobile**, little design system, few rich widgets |
| **egui** | Immediate | wgpu/gl | Ultra-simple, king of tools/games | Not retained, limited styling, not "consumer app" material |
| **Slint** | A `.slint` DSL | in-house/Skia | **Live preview**, embedded, tooling | A proprietary language (not plain Rust), commercial licence |
| **Dioxus** | React/RSX + signals | webview/wgpu (Blitz) | **Hot reload**, web/mobile, familiar | Young native rendering, not Elm |
| **gpui** (Zed) | Imperative retained | In-house GPU | **Extreme performance**, proven by Zed | Thinly documented/open, steep curve |
| **Xilem** | Reactive (diff) | Vello/wgpu | Linebender backing, the future | Experimental, API in flux |
| **floem / Makepad / Freya** | Signals / shader DSL / Skia+Dioxus | various | Niches (editors, live design) | Small ecosystems |

**The empty slot frus occupies:** *"iced, but with a real Material 3 design system,
physical animations, and mobile that works"*. You **already** have three wedges few have
combined:
1. **Working Android** (milestone 50) — iced/egui/pure-Slint are weak there.
2. **61 widgets**, including rich ones (table, tree, date picker, carousel, autocomplete) —
   broad *before* the others.
3. **Structured theming + spring animations** — the road to a *beautiful* default.

**What to add to convert (in order of impact on adoption):**
1. **A visually superb default, with no configuration** (the M3 theme §5 + the animations
   §4). The first screenshot decides. That is marketing lever number one.
2. **Web (wasm)** (§11) — demos shareable by URL = viral adoption.
3. **DX: hot reload + fast compiles + cargo-native** (§8, §13) — the friction that drives
   people away.
4. **iOS** (§11) — locks in the "the Rust desktop+mobile framework" slot.
5. **AccessKit + i18n** (§14) — unlocks the enterprise/public-sector market.
6. **Docs + an example gallery + a `cargo new` template** (§8) — the front door.

**Strategic honesty**: you do not "win" by beating *everyone everywhere*. You win by
**dominating one slot**: *beautiful Rust apps, desktop + mobile + web, no GC, instant
startup, a premium default design.* That is defensible, it is empty, and goals 2 and 3
(memory, lightness/startup) are its technical proof. The rest of Part II is the list of what
is missing to *hold* that slot.

---

## 17. Strategic synthesis — the 3 pillars & the consolidated roadmap

**The 3 differentiating pillars** (never to be diluted):
- **Pillar A — honest native performance**: no GC, instant startup (bundled fonts, lazy GPU
  init), a controlled footprint, 120 Hz without jank. *(goals 2 & 3; §1, §12, §15)*
- **Pillar B — the most beautiful Rust toolkit, effortlessly**: M3 design by default,
  physical animations, 61+ polished widgets. *(§4, §5)*
- **Pillar C — the only comfortable "desktop + mobile + web" Rust option**: Android already
  there, then iOS + wasm, on a shared `view`/`update`, with a DX that does not drive people
  away. *(§8, §10, §11, §13)*

**The consolidated order** (merging the §7 roadmap with Part II):
1. **Engine foundations** (block A of §7) — phases/dirty lists, the relayout cache, the
   animation driver.
2. **The minimum DX that retains people** — first-class `.map()` (§8), `lazy`/memo (§9),
   async `Command` + `Subscription` (§10), the inspector + headless tests (§13). *Without
   these, nobody stays, whatever the engine.*
3. **A premium default design** — the M3 theme, the scene primitives
   (rrect/shadow/gradient), `BoxDecoration`, `TextLayout` (§5) + generalised animations
   (§4). *Pillar B, your marketing.*
4. **Input & motion** — gesture tiers 0→1, physical scrolling (§3, §6).
5. **Reach** — web/wasm first (adoption), then iOS; multi-window, embedding (§11).
6. **Trust & compliance** — AccessKit, Fluent i18n, focus/keyboard (§14, §6).
7. **Distribution** — per-target bundling, a `cargo new` template, an example gallery, a
   measured minimal binary (§15, §8).
8. **State-preserving hot reload** (`subsecond`) — the finishing blow on goal 4 (§13).

**Deferrable without guilt**: the real gesture arena (nesting), signals, lazy slivers, HCT
`from_seed`, `no_std`/embedded, a full semantics tree beyond AccessKit.

---

## Appendix — what NOT to copy from the prior art (recap)
- Stateful widgets with their own `State`/lifecycle, global keys, ambient-dependency
  widgets and their dependency graph, observable notifiers as app state → **Elm replaces
  them**.
- The flex/grid `performLayout` maths → **taffy**.
- The `markNeeds*` walk over a tree of mutable parent pointers → **an arena/slotmap +
  dirty lists of `Id`**.
- The dual (deprecated) raw-key-event path → one key event model.
- Lazy slivers → defer until the cost of full-layout-on-scroll bites.
- Arena re-entrancy through microtask scheduling → **pure functions returning the
  outcomes**, drained after the borrow ends.
