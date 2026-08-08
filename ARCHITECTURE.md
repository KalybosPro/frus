# Architecture

This document is the map you should have in your head before making a non-trivial change. It explains what each crate owns, how a frame actually gets to the screen, and — most usefully — **where a given kind of code belongs**.

For the reasoning behind any individual decision, the milestone notes in [`docs/milestone-*.md`](docs/) are the authoritative record. This is the overview; those are the minutes.

---

## The shape of the thing

frus is four layers of crates. **Dependencies only point downward.** Nothing below `frus-shell` is allowed to know which platform it is running on — that single rule is what makes one codebase target desktop, Android, and the web.

```
      ┌────────────────────────────────────────────────────────────┐
      │  APPLICATION                          what you write       │
      │  frus (facade) · frus-hello · frus-demo · frus-transforms  │
      │  frus-fetch-example · frus-test                            │
      └────────────────────────────────────────────────────────────┘
                                   ▲
      ┌────────────────────────────────────────────────────────────┐
      │  SHELL                                platform layer       │
      │  frus-shell                                                │
      │  window · event loop · lifecycle · IME · a11y · net        │
      │  Application · Command · Subscription · main!              │
      └────────────────────────────────────────────────────────────┘
                                   ▲
      ┌────────────────────────────────────────────────────────────┐
      │  WIDGETS                              UI & interaction     │
      │  frus-widgets                                              │
      │  Widget trait · Ui/scene build · hit-testing · focus        │
      │  scrolling · drag & drop · theme                           │
      └────────────────────────────────────────────────────────────┘
                                   ▲
      ┌────────────────────────────────────────────────────────────┐
      │  FOUNDATIONS                          render & measure     │
      │  frus-core   geometry, color, paths, animation, Scene      │
      │  frus-layout flexbox (taffy)     frus-text  shaping        │
      │  frus-gpu    wgpu renderer       frus-image decode         │
      │  frus-l10n   Fluent i18n                                   │
      └────────────────────────────────────────────────────────────┘
```

## Crate by crate

### Foundations

**`frus-core`** — the vocabulary everything else speaks. Zero heavy dependencies.

- `geometry.rs` — `Point`, `Size`, `Rect`, insets
- `color.rs`, `hct.rs` — color, plus the HCT perceptual color space used for theme generation
- `path.rs`, `decoration.rs` — vector paths, borders, shadows, gradients
- `text_style.rs` — font, weight, size, decoration
- `scene.rs` — **`Scene` and `Primitive`**: the flat, backend-agnostic display list. This is the contract between widgets and the GPU. A widget paints by pushing primitives; the renderer only ever consumes them.
- `animation/` — spring physics, curves, `AnimationController`
- `semantics.rs` — accessibility annotations (role, label, value, state)
- `responsive.rs`, `image.rs` — breakpoints, `ImageData`

**`frus-layout`** — flexbox over [`taffy`](https://github.com/DioxusLabs/taffy). Owns `Style` and turns a style tree into resolved rects. It does not know what a widget is.

**`frus-text`** — shaping and measurement over [`cosmic-text`](https://github.com/pop-os/cosmic-text). Given a string, a style, and a width constraint, returns laid-out glyph runs and a measured size.

**`frus-gpu`** — the renderer. Consumes a `Scene`, produces pixels.

- `renderer.rs` — the `wgpu` device, surface, and frame loop
- `painter.rs` — primitive → draw-call translation, batching
- `path.rs` — [`lyon`](https://github.com/nical/lyon) tessellation of fills and strokes into triangles
- `text.rs` — glyph atlas and text draw via `glyphon`
- `compositor.rs` — layers, clips, transforms
- `offscreen.rs` — headless rendering to a buffer; this is what makes golden tests possible
- `shaders/` — WGSL

**`frus-image`** — PNG/JPEG decode to `ImageData`. Deliberately narrow: fewer formats, smaller dependency tree.

**`frus-l10n`** — Fluent bundles and locale negotiation. We use Mozilla's Fluent rather than inventing a message format.

### Widgets

**`frus-widgets`** — the largest crate, ~80 modules. Two halves:

*The tree.* The `Widget<Msg>` trait is the core abstraction — generic over the app's message type, which is how interaction stays type-safe end to end:

```rust
pub trait Widget<Msg> {
    fn style(&self) -> Style;                     // → frus-layout
    fn children(&self) -> &[Box<dyn Widget<Msg>>];
    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene);
    fn on_click(&self) -> Option<Msg>;
    fn key(&self) -> Option<u64>;                 // stable identity across rebuilds
    fn semantics(&self) -> Option<Semantics>;     // accessibility
    // …
}
```

*The runtime.* `ui.rs` builds a `Ui<Msg>` from a widget tree: it runs layout, walks the tree painting into a `Scene`, and populates the registries that make interaction work — hit-testing, focus order and directional focus, scrollable viewports and extents, draggables, reorderables, interactive bounds, semantics.

> **This is the subtlety most new contributors hit.** `Ui` resolves a widget's rect through *per-kind registries*. A new interactive widget that isn't registered in the right one will pass its unit tests and still silently do nothing on live hover, drag, or scroll paths. If you add an interactive widget, wire it into the registry it belongs to.

Everything else in the crate is widgets — `button.rs`, `scroll.rs`, `datatable.rs`, `kanban.rs`, `textinput.rs`, `navigator.rs`, and so on — plus `theme.rs` (the overridable design tokens), `interaction.rs` (`Status`, `Cursor`, `Key`), `dsl.rs` (the `column!` / `row!` macros), and `inspector.rs` (tree dumps for debugging).

### Shell

**`frus-shell`** — the only crate that knows about platforms. Everything platform-specific is `#[cfg]`-gated here and nowhere else.

- `application.rs` — the **`Application` trait**: `update`, `view`, `subscription`, `init`, `on_lifecycle`, `title`, theming. This is the app-facing contract.
- `app.rs` — the driver: event loop, frame scheduling, animation ticking, input dispatch into `Ui`
- `command.rs` — `Command<Msg>`: effects. Sync tasks, `perform_async` futures, batching. Native runs them on a thread; the web runs them through `spawn_local`.
- `subscription.rs` — continuous message sources (timers, streams), started and stopped by diffing what `Application::subscription` returns each cycle
- `gesture.rs` — taps, long-press, drag, fling, back-gesture, inertia
- `a11y.rs` — AccessKit bridge (desktop)
- `android_ime.rs` — real Android IME: composition, swipe input, insets
- `net.rs` — `fetch` / `Request` behind the `net` feature; `ureq` natively, browser `fetch` on wasm. `json` adds serde-typed bodies and responses.
- `reload.rs` — dev live-reload
- `remote.rs` — remote/debug channel
- `lib.rs` — the **`main!` macro**, which generates the desktop `main`, the Android `android_main`, and the wasm `#[wasm_bindgen(start)]` from one declaration

### Application layer

**`frus`** — the facade. Re-exports shell + widgets + `main!` so an app declares exactly one dependency. Features `net` and `json` propagate downward. It contains no logic; keep it that way.

**`frus-test`** — headless rendering, `Snapshot`, and `assert_golden` / `assert_golden_with` against reference PNGs.

**`frus-hello`**, **`frus-demo`**, **`frus-transforms`**, **`frus-fetch-example`** — sample apps. `frus-hello` is also the source of the `cargo generate` template in [`templates/app`](templates/app); if you change one, change the other.

## How a frame happens

```
  event (click / key / touch / timer / async result)
        │
        ▼
  frus-shell   dispatch: hit-test through Ui → Msg
        │
        ▼
  Application::update(&mut self, msg) -> Command<Msg>      ← pure. no GPU, no window.
        │                                    │
        │                                    └─→ effect runs (thread or spawn_local)
        │                                          └─→ produces another Msg, loops back
        ▼
  Application::view(&self, theme, w, h) -> Box<dyn Widget<Msg>>
        │
        ▼
  frus-widgets  build_ui:
        │         style tree ──→ frus-layout (taffy) ──→ resolved rects
        │         text nodes ──→ frus-text (cosmic-text) ──→ measured runs
        │         paint walk ──→ Scene (flat primitive list)
        │         registries ──→ hit / focus / scroll / drag / semantics
        ▼
  frus-gpu      Scene ──→ batched draw calls ──→ wgpu ──→ Vulkan / Metal / DX12 / WebGPU
        │
        ▼
  pixels
```

Two properties fall out of this and are worth protecting:

- **`update` never touches the GPU.** That's why most of the test suite runs headless.
- **`Scene` is the only thing crossing into the renderer.** Widgets can't reach the GPU directly, so a new backend would only need to consume `Scene`.

## Where does my code go?

| I want to… | It goes in |
|---|---|
| Add a widget | `frus-widgets/src/<name>.rs`, exported from `lib.rs`. Register it if it's interactive. |
| Add a paintable shape or effect | A new `Primitive` in `frus-core/src/scene.rs`, handled in `frus-gpu/src/painter.rs` |
| Change how things are measured or positioned | `frus-layout`, or the widget's `style()` |
| Change text shaping or metrics | `frus-text` |
| Add a design token / theme knob | `frus-widgets/src/theme.rs` — and make sure the widget reads it instead of a literal |
| Handle a new gesture | `frus-shell/src/gesture.rs`, plus a registry in `frus-widgets/src/ui.rs` |
| Add an async capability (network, storage) | `frus-shell` behind a Cargo feature, exposed as a `Command` |
| Support a new platform | A new `#[cfg]` module in `frus-shell` and an arm in `main!`. Nothing else should need to change — if it does, that's a bug in the layering. |
| Add a shared type used by more than one crate | `frus-core` |

## Deliberate constraints

Things that look like limitations and are in fact choices:

- **No retained widget tree between frames.** `view` rebuilds; identity is carried by `key()` where it matters. This keeps state in one place — the app struct — at the cost of rebuild work. If rebuild cost becomes real, the fix is memoization, not a mutable tree.
- **`Box<dyn Widget<Msg>>` everywhere.** Dynamic dispatch is a real cost, and it's what makes heterogeneous children and a stable trait boundary possible. Measured, it has not been the bottleneck.
- **One renderer backend.** `wgpu` covers Vulkan, Metal, DX12, and WebGPU. There is no software fallback, and adding one is not currently a goal.
- **Layout is taffy, text is cosmic-text.** These are hard, well-solved problems. We wrap them; we don't reimplement them.
- **`unsafe` only at the platform edge.** The framework core is safe Rust.

## Reading the milestone notes

276 numbered documents in [`docs/`](docs/README.md), one per milestone, each recording the objective, the alternatives weighed, the decision, the implementation, how it was verified, and what was deliberately left out.

When you're about to ask "why on earth is it done this way?" — `grep docs/` first. The answer is usually there, along with the option that was rejected and the reason. They are currently written in French; translating them is a genuinely valuable contribution.

Useful entry points:

- `milestone-0.md` — the GPU context and the very first frame
- `milestone-129.md`, `milestone-131.md` — the web target, and shrinking the wasm payload
- `milestone-267.md`, `milestone-268.md` — the single entry point and the facade crate
- `milestone-270.md`–`milestone-275.md` — async effects, `fetch`, `RemoteData`, typed JSON
