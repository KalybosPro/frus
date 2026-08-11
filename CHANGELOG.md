# Changelog

All notable changes to frus are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) — with the usual 0.x caveat that
any release may break.

> frus is **pre-alpha** and **not on crates.io**. Releases are tagged source releases:
> depend on them by `path` or by git revision. For the reasoning behind any individual
> decision, the milestone notes in [`docs/milestone-*.md`](docs/) remain the authoritative
> record — one per step, 277 so far, each documenting the objective, the alternatives
> weighed, and the decision.

## [Unreleased]

### Added

- **The overscroll glow** (J279). A platform that clamps now answers a refused drag or a
  fling landing on an edge with an arc of light, instead of with silence: `OverscrollGlow`,
  fed by the movement `apply_boundary_conditions` refuses, by a ballistic stopped at an edge,
  and by a wheel notch past the end. Bouncing physics feeds none of it — the bounce is
  already the answer. `Path::oval` and `Curve::Decelerate` in `frus-core` came with it.
- **Real velocity estimation** (J278). `VelocityTracker` keeps the last 20 pointer
  positions and fits a quadratic through them, so a gesture that slows down as the finger
  lifts is still read as the throw it was; the platforms that bounce instead use a weighted
  average of the last three sample velocities, picked by `VelocityTracker::platform_default`.
  `VelocityEstimate` carries the travel alongside the speed, and `PolynomialFit` exposes the
  least-squares solver.
- **Scroll physics per platform** (J277). `ScrollPhysics::{Bouncing, Clamping}` names how a
  scrollable behaves at its edges and after a fling, and defaults to what the running
  platform does. Clamping follows the platform's spline deceleration and stops dead at the
  edge; bouncing resists progressively past the edge (a real rubber band) and springs back.
  Set it app-wide with `Application::scroll_physics`, or per area with `Scroll::physics` /
  `List::physics`.
- `ClampingScrollSimulation` and `BouncingScrollSimulation` in `frus-core`, plus
  `FrictionSimulation::time_at_x`.

### Fixed

- **A scroll offset now has one owner at a time** (J279). The edge spring kept retracting an
  overscrolled offset *while a finger was still holding it*, so a rubber band was dragged
  home as fast as it was stretched — on a slow drag it never appeared at all. Found on a
  physical device, not by a test.

### Changed

- A fling now needs **distance as well as speed**: a fast twitch that covered less than the
  drag threshold no longer throws the content, per axis (J278).
- The drag threshold follows the pointer: 18 logical px for a finger, 1 px for a mouse or
  trackpad, where it used to be 8 px for everything (J278).
- The scroll registry exposes `Ui::scroll_regions()` / `scroll_region(id)` returning a
  `Scrollable` (id, viewport, bounds, physics); `Ui::scroll_hit` returns one too, instead of
  a tuple.
- The wheel's elastic overshoot now exists only where the physics allows overscroll.

### Removed

- `gesture::fling_destination` and its constants, superseded by the physics layer.

## [0.1.0] - 2026-08-08

The first tagged release: a point in the history that can be cited, cloned and diffed. It
is not an API commitment — nothing here is stable, and the public surface will move.

277 milestones went into it, so this entry groups **what exists**, not what changed since a
previous release; there is no previous release.

**What it is not, yet:** nothing is published to crates.io, there is no MSRV, the web target
has no clipboard, accessibility or live reload, iOS compiles but does not run, and `view`
rebuilds the whole tree every frame (no memoisation). See [ROADMAP.md](ROADMAP.md).

### Added

**Platforms**

- **Desktop** (Windows / Linux / macOS) on winit + wgpu — clipboard, screen-reader
  accessibility through AccessKit, live reload in dev.
- **Android** — native activity, Vulkan, a real IME (composition and swipe), window insets,
  and the full application lifecycle. Validated on a physical device (J50).
- **Web** (wasm + WebGPU) — rendering, input, animation, subscriptions, and asynchronous
  effects including `fetch`.
- **iOS** — groundwork only (J276): named platform `cfg`s, a `run()` entry point, and CI
  compiling both Apple targets.

**Application model**

- The **Elm architecture**: `Application { update, view }`, with `update` pure and
  synchronous. Every effect goes through `Command`; every long-lived stream through
  `Subscription`. This is what makes 717 tests run with no GPU and no window.
- **One entry point** (J267) — `frus::main!(App::default())` generates the desktop, Android
  and wasm entry points from a single declaration.
- **The `frus` facade** (J268) — an application declares exactly one dependency.
- **Asynchronous effects** (J270) — `Command::perform_async` / `run_async`, so a real
  `await` works even on the single-threaded web target.
- **Networking** behind the `net` feature (J271–J272) — a cross-platform `fetch` with one
  signature on all three targets, and a `Request` builder for methods, headers, bodies and
  timeouts.
- **Typed JSON** behind the `json` feature (J275), and **`RemoteData<T, E>`** (J274), the
  Elm idiom for the four states of a request.

**Rendering and layout**

- A **wgpu** renderer: rounded rects, borders, shadows, gradients, vector paths tessellated
  with `lyon`, images, opacity, clipping, layer compositing, MSAA, and offscreen rendering.
- **Flexbox and grid** layout over `taffy`, with a relayout-boundary cache and frame phases
  so a hover repaints without relaying out.
- **Text** shaped by `cosmic-text`: rich spans, wrapping, intrinsic sizes fed back into
  layout, caret, hit-testing, selection, and bidirectional scripts.

**Widgets and interaction**

- A widget library of ~80 modules — text fields with IME, data tables, an editable grid,
  charts, date and time pickers, a kanban board, trees, lists, tabs, toasts, modals and
  portals, a drawer, navigation with a scaffold, and a validated multi-step form.
- **Interaction**: focus and keyboard navigation, long press, fling, drag-and-drop
  reordering with a live preview, the back gesture, and spring-driven navigation.
- **Theming** — a Material 3 colour scheme with roles, a type scale, and state layers.
  Every widget's styling and slots are overridable; the defaults are themed, never
  hardcoded.
- **Animation** — a unified `trait Simulation` (spring, friction, clamped) driving scroll
  momentum, sheets and transitions from one path.

**Cross-cutting**

- **i18n / l10n / RTL** via Fluent bundles, locale negotiation, and layout mirroring.
- **Accessibility** — a semantics tree bridged to AccessKit.
- **`frus-test`** — headless rendering, snapshots, and golden-image comparison (87 goldens).
- **Tooling** — a `cargo generate` template, a runtime tree inspector, and state-preserving
  live reload in dev.

### Project

- `README.md`, `README.fr.md`, `CONTRIBUTING.md`, `ARCHITECTURE.md`, `ROADMAP.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, dual MIT/Apache-2.0 licences, issue and PR templates,
  and CI covering the three platforms plus an iOS compile job.
- The repository is entirely English — code, comments, documentation and commit messages —
  with `README.fr.md` as the one deliberate exception.

<!--
For releases, use this shape:

## [0.1.0] - YYYY-MM-DD

### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
-->

[Unreleased]: https://github.com/KalybosPro/frus/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/KalybosPro/frus/releases/tag/v0.1.0
