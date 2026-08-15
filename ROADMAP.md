# Roadmap

Where frus is, where it's going, and — the point of this document — **what is unclaimed and ready for you to pick up**.

Items are tagged:

- 🟢 **Good first issue** — self-contained, clear success criterion, no deep context needed
- 🟡 **Help wanted** — meaty, well-scoped, needs some familiarity with a subsystem
- 🔴 **Design first** — open a discussion before writing code; the shape isn't settled
- 🔵 **Claimed / in progress** — talk to the maintainers before duplicating

If something here interests you, **comment on the matching issue** (or open one) before starting.

---

## Where we are

304 milestones in. The framework runs real, non-trivial applications on desktop and Android, and functional ones on the web. What exists is genuinely built, not stubbed: layout, text with IME, drag-and-drop with live reflow, data tables, charts, pickers, navigation with spring transitions, theming, i18n/RTL, accessibility, animation, async effects with typed JSON, and golden-image testing.

What does not exist is everything around it: distribution, tooling, more platforms, and the ecosystem.

| | Desktop | Android | Web | iOS | macOS native |
|---|---|---|---|---|---|
| Rendering | ✅ | ✅ | ✅ | — | — |
| Input & gestures | ✅ | ✅ | ✅ | — | — |
| Text input / IME | ✅ | ✅ | ⚠️ basic | — | — |
| Animation & subscriptions | ✅ | ✅ | ✅ | — | — |
| Async effects / `fetch` | ✅ | ✅ | ✅ | — | — |
| Clipboard | ✅ | ✅ | ❌ | — | — |
| Accessibility | ✅ AccessKit | ⚠️ partial | ❌ | — | — |
| Lifecycle | ✅ | ✅ | ⚠️ partial | — | — |
| Live-reload (dev) | ✅ | ⚠️ | ❌ | — | — |

---

## Near term

The things standing between frus and someone else being able to use it.

### Distribution

- 🟡 **Publish to crates.io.** Everything resolves through local `path` dependencies today. This needs real versions, per-crate `README`s, metadata (`repository`, `keywords`, `categories`, `documentation`), a publish order that respects the dependency graph, and a release script. Once done, the `cargo generate` template drops `{{frus_path}}` and becomes `frus = "0.1"`.
- 🟢 **Per-crate `README.md`.** Each crate needs a short one for its crates.io page.
- 🟢 **Pin an MSRV.** There is no `rust-version` in any manifest and no minimum supported Rust version has ever been tested. Find the oldest stable that builds the workspace, put `rust-version` in `[workspace.package]`, and add that toolchain to the CI matrix.
- 🟡 **docs.rs-quality rustdoc.** Crate-level docs with a runnable example on every public crate, and `#![warn(missing_docs)]` turned on crate by crate.

### Web parity

The web target renders and animates but is missing its platform integrations. Each of these is an independent, self-contained job.

- 🟡 **Clipboard** via the async Clipboard API, behind the same interface desktop uses.
- 🟡 **Accessibility.** AccessKit has web support; the semantics tree already exists and is populated. This is a bridging job, not a from-scratch one.
- 🟡 **IME / soft keyboard** on mobile browsers — a hidden input overlay, composition events, viewport insets.
- 🟡 **Live-reload** for the wasm target.
- 🟢 **A proper web example page** — the current `index.html` is minimal.

### Size

Milestone 292 took a release APK from 286 MB to 4.9 MB by building `--release` at all. What is left is real work, not settings.

- 🟡 **Subset the bundled faces at build time.** DejaVu covers far more of Unicode than any one application draws, and the four `bundled-*` features are a coarse instrument next to a `pyftsubset` step over the glyphs a build actually references. This is where the next megabyte is, and it needs a tool in the build.
- 🟢 **A size regression check in CI.** Nothing notices today if the floor doubles. Build `frus-hello` for `aarch64-linux-android` in release, compare the stripped `.so` against a committed budget, fail on a jump.
- 🟢 **Document `--split-per-abi`-style packaging.** The examples build `aarch64` only; an application targeting more than one ABI wants a split rather than a fat APK, and nothing says so.

### Quality

- 🟢 **Clear the clippy backlog — done, milestone 298.** 71 warnings, now zero, and CI runs `cargo clippy --workspace --all-targets -- -D warnings` as a **blocking** check. Six `too_many_arguments` carry a targeted `#[allow]` with the reason at the site; everything else was rewritten.
- 🟢 **Format the tree — done, milestone 298.** One `cargo fmt --all` commit over 40 files, on its own, and the fmt check is **blocking** now.
- 🟢 **Widen golden coverage — done, milestones 296 and 297.** 58 of the 86 widget modules had no pixel test at all; **all 86 have one now**. 296 added 27 goldens for the widgets whose picture follows from their arguments; 297 added `frus_test::Stage`, a harness that holds the retained state and steps the frame loop the way the shell does, and 12 more for the ones whose picture is a gesture in flight — a swipe half done, a pull past the top, a glow, a page between two pages.
- 🟡 **Make the goldens a required check.** The step stays advisory, and an advisory check went red for five milestones without anyone noticing (J294). The reason recorded here — "lavapipe rasterises differently from hardware" — is not the real one: the goldens are *blessed* under lavapipe too (llvmpipe in WSL is the only adapter there), so both sides run the same rasteriser. What actually differs is its **version** — mesa 25.2 locally against whatever `apt` gives ubuntu-latest. So the job is either to pin the rasteriser in CI (a container image, or a mesa PPA) and drop `continue-on-error` outright, or to measure the version-to-version drift and absorb it through `assert_golden_with`. Pinning is the honest one; a tolerance guessed without measuring is a number nobody can defend.
- 🟢 **A real async runtime — done, milestone 303.** `Command::perform_async` existed, and natively it was a thread per effect plus `pollster::block_on`, with **no reactor** — so a future that waited on a timer or a socket parked its thread and nothing ever woke it. Asynchrony was in the type system and absent underneath it. There is now one executor on four workers, each running it inside `async_io::block_on`, and every asynchronous effect and every subscription is a task on it. New: `Command::after`. Left, and this is the interesting part: **accept a runtime instead of owning one** — an `Executor` trait an application implements, so a codebase already running tokio hands it over and the whole HTTP ecosystem becomes reachable; and **subscriptions as streams**, since `Subscription` still has exactly one kind, `Every`, while a WebSocket or a file watcher is now expressible.
- 🟢 **Pictures in the README — done, milestone 304.** A GIF of the demo crossing four screens, four stills and one photograph of a phone, plus the tool that renders them (`--features shots`), so a picture is regenerated after a change rather than left to go stale. The starting issues live in `.github/issues/` with a script that opens them.
- 🟢 **Benchmarks — done, milestone 299.** `crates/frus-bench`: `build_ui`, text measurement and wrapping, and batch planning, on shared fixtures, under the release profile. `cargo bench -p frus-bench`; CI runs `-- --test` so they cannot rot. The baseline and what it found are in `docs/milestone-299.md`. Left: nothing measures the **GPU** side of a frame, only the CPU side.
- 🟢 **Fix the broken intra-doc links — done, milestone 298.** Twelve of them: links to private items, two to types that never existed (`ClipRect`, `RepaintBoundary`), one to an entry point renamed away, and five in `form`'s module docs that resolve only when written in full. `ValueAnim` became public, since it is the type of a public field. The strict rustdoc pass is **blocking** now, and it turned up an unparseable `ignore` code block behind them.

---

## Medium term

### New platforms

- 🔵 **iOS shell.** The most valuable single contribution available. `frus-shell` is the only crate that would change: a `UIViewController` host, Metal surface via `wgpu`, touch input, IME, lifecycle, safe-area insets. The layering is designed for exactly this, and Android is the worked example to follow.

  **Groundwork landed** (see `docs/milestone-276.md`): `frus-shell` now has named platform `cfg` aliases (`desktop` / `android` / `ios` / `web`) via its `build.rs`, so iOS no longer falls silently into the desktop branch, and a `run()` entry point exists behind `#[cfg(ios)]`. An advisory CI job builds for `aarch64-apple-ios-sim` and `aarch64-apple-ios`. What remains is the actual platform integration — lifecycle, safe-area insets, IME/soft keyboard, `os_log`, UIKit accessibility, and `.ipa` packaging.
- 🔴 **Native macOS shell.** winit already covers macOS as a desktop target; a native shell would be about menu bar, window chrome, and platform conventions rather than rendering.

### Framework depth

- 🟡 **Text rendering edge cases.** Bidi runs, complex scripts, font fallback chains, emoji, vertical metrics. Real bug reports welcome here.
- 🟢 **`Scaffold` and `body` — reviewed, milestone 288.** `extend_body`, `extend_body_behind_app_bar`, `resize_to_avoid_bottom_inset`, `window_insets`, a leading `drawer` beside the trailing one, and `persistent_footer`. The FAB gained a **location** in milestone 290 — three ends × float or docked.
- 🟢 **A bottom app bar with a notch — done, milestone 291.** `BottomAppBar` in the scaffold's bottom slot, cut around a docked FAB by the scaffold, which is the only party that knows where both are. Left: the bar has no elevation or surface tint, for the same reason the app bar has none.
- 🟢 **The renderer composites in scene order — done, milestones 294 and 295.** It used to draw one pass per kind — rectangles, then images, then paths, then text — so *every* path covered *every* rectangle in the frame, wherever the two sat in the scene. Found on a device in milestone 291 (a filled button on a notched bar); it applied to `CustomPaint`, the charts, `ClipPath` and the overscroll glow just as much. Primitives are now given a **level** from what they cover, and a level costs one draw call per kind it holds — a twelve-row list is 3 draw calls, where ordering primitive-against-primitive would have cost 25. Text was left out of the plan in 294 and folded in by 295, once `Primitive::Text` learned to carry the box it was laid out in. `frus_gpu::draw_calls(scene)` reports the cost. Left: a primitive's footprint is its widget's box, so text over a wide, mostly-empty label costs a level it need not; tightening it to the shaped extents wants the measurement to come back from the text painter.
- 🔴 **A widget cannot measure its children, nor describe anything to its subtree.** Children are built by the application *before* the widget that holds them sees them, so a container can neither ask how big they are nor put an ambient value where they would read it. The reference builds lazily and does both. Three known symptoms, all deferred for this one reason: the FAB's height must be **declared** to dock it (`fab_size`, milestone 290) and its `*Top` placements are missing for want of the app bar's height; `AppBar` is a builder rather than a `Widget` because it needs the available width before it can decide anything; and a scaffold cannot tell an extended body what it is running under, so that body pads its own content by hand. Wants a design — lazy children, or a resolution pass between build and layout — not a parameter.
- 🟢 **`AppBar` is a builder, not a `Widget`** — it must be finished with `.build()` because it needs the available width before it can decide anything. Every other widget in the framework *is* a `Widget`. Revisiting it is an API change with call sites, not a fix.
- 🟢 **The app bar's title carries no `Role::Heading`.**
- 🟢 **`NavBar` shrinks around its back button** when its parent gives it no width (found by milestone 296's first golden of it). Its `paint` centres the title in `bounds`, which only makes sense at full width, but its `style` asks for `Dimension::Auto`, so with nothing to fill it hugs the button and paints the title underneath. Every screen happens to give it a width, which is why it has never shown.
- 🟢 **A wrapping text that is shrunk to fit reported one line's height — fixed, milestone 289.** The diagnosis recorded here (measure-pass ordering) was wrong: the cause was half a pixel. Text measurements now round up, so the box the layout rounds to is one the text still fits in. Left, and general: the layout engine rounds every box to whole pixels while the reference does not, so anything that measures itself must round up too.
- 🟢 **Scrolling physics — done, with two loose ends.** `ScrollPhysics::{Bouncing, Clamping}` with the platform's own fling curves, a real rubber band and per-app / per-area overrides (`docs/milestone-277.md`); a fitted velocity estimate (`docs/milestone-278.md`); the overscroll glow, device-verified (`docs/milestone-279.md`) and rebuilt as an actual **fade** in milestones 301 and 302 after a second device report — it was a flat fill, and a hard curved edge across the page reads as the page being bent rather than as light. Paths carry a gradient now, straight or radial. Left: the **stretch** overscroll effect newer platform versions use instead of a glow (needs a render-target effect), and the second bouncing deceleration profile.
- 🟢 **Cache text measurement — done, milestone 300.** Milestone 299 found that **three quarters of the cost of building a frame was `frus_text::measure`**, re-shaping every string through cosmic-text on every call for strings that had not changed. A cache keyed on `(text, size, weight, italic, max_width)` — with the weight and style **resolved**, so a Medium asked for on a family that only ships Regular hits the same entry as the Regular — took `measure/line` from 16.3 µs to **79 ns**, and took building a twelve-row frame from **4.1×** the cost of the same tree with no strings in it to **1.20×** — text is a sixth of a frame now, where it was three quarters. Eviction is two generations with a promotion on hit, so a string still on screen never falls out and one that has gone leaves with its generation; registering a font empties the cache, an answer from before that being wrong rather than stale. Left: the batch planner is still O(n²), and nothing measures the GPU side of a frame.
- 🔴 **Rebuild memoization.** `view` rebuilds the whole tree each frame. Milestone 299 sized it: the same tree with every string replaced by a fixed box costs a quarter as much, so rebuilding is real but it is the *small* half — measure text first. Wants a design, not a parameter.
- 🟡 **The batch planner is O(n²).** Each primitive scans the levels for something it overlaps: 80 primitives plan in 4.7 µs, 1302 in 597 µs (16× the primitives, 127× the time). About 4% of a 60 Hz frame at 1302, so not urgent — but it grows the wrong way, and milestone 294 should not have implied the plan was free. Wants a spatial index, or levels that carry a bounding box per kind.
- 🟡 **Renderer batching.** Fewer draw calls for scenes with many small primitives; atlas the common shapes.
- 🟡 **More widgets.** The catalogue is broad but not complete. Landed in milestone 280:
  the ambient surface description (`MediaQuery`, `SafeArea`) and the widgets that withhold
  part of the frame (`IgnorePointer`, `AbsorbPointer`, `Visibility`, `Offstage`,
  `ExcludeSemantics`). Still unclaimed, roughly in order of how often they are reached for:

  - 🟢 **Pull-to-refresh — done** (milestone 281): `Refresh`, device-verified under both
    physics. Left: the same machinery on the **bottom** edge for load-more, and a way for
    an application to start the indicator itself.
  - 🟢 **Swipe-to-dismiss — done** (milestone 282): `Dismissible`, with the shared-gesture
    arbitration that lets it live inside a list. Left: a confirmation step (an undo window
    needs the message to be able to refuse) and cross-axis drift. The on-device check is
    **done** (2026-08-12).
  - 🟢 **A paged view — done** (milestone 283): `PageView`, virtualised, snapping on the
    milestone-277 physics, with `page`/`on_page_changed` binding both directions to one
    number, device-verified in both directions. Left: per-page transformations (parallax,
    depth), padded ends below a `viewport_fraction` of 1, and keyboard paging.
  - 🟢 **Shared-element transitions — done** (milestone 286): `Hero`, device-verified.
    The "two trees" half turned out to be free — the navigator already holds both screens
    in one frame. Left: a curved flight path, a cross-fade between the two contents, and
    shared elements that move *within* a screen rather than between routes.
  - 🟢 **The constraint boxes — mostly done** (milestone 284): `SizedBox`,
    `ConstrainedBox`, `Intrinsic`, `OverflowBox`, and `max_width`/`max_height` on `Style`.
    Two are left, and both need something the layout does not yet surface:
    **`LimitedBox`** needs to know whether the incoming constraint was *unbounded*, which
    taffy owns and never tells a widget; **baseline alignment** needs the text measurement
    to report a baseline, which the measure hook cannot return.
  - 🟢 **A general drag-and-drop pair — done** (milestone 285): `Draggable` /
    `DragTarget`, with a `u64` payload, a ghost lifted out of the frame, and a lift that
    yields to any scrollable under it (`long_press()` inside a list). Device-verified.
    Left: auto-scroll at a viewport's edges while carrying, a typed payload, a fly-back
    on a refused drop, and migrating `Kanban`/`Table` onto it.
  - 🟡 Rich text editing, video, maps, virtualized lists for very large datasets.
- 🔴 **Router / deep linking.** `navigator.rs` handles in-app navigation; URLs, deep links, and browser history are not modelled.
- 🟡 **State persistence.** A blessed way to save and restore app state across lifecycle transitions.

### Developer experience

- 🔴 **DevTools.** `inspector.rs` can already dump the widget tree. A live inspector — tree view, layout overlay, rebuild counts, frame timing — is a large but very high-leverage project.
- 🟡 **Better error messages.** Layout and constraint failures should say what went wrong and what to change, not just produce a wrong-looking screen.
- 🔴 **Hot reload** beyond the current live-reload, with state preservation.

---

## Long term

The original brief aims at a full framework, not just a UI toolkit. These are real goals, not yet real work:

- **Plugin system** — a stable ABI for third-party platform integrations (camera, sensors, storage), with the thin-native-adapter rule enforced.
- **FFI story** — embedding frus in an existing native app, and calling native code from frus.
- **Package/ecosystem conventions** — how a community widget library is published and discovered.
- **Static analysis** — lints specific to frus's architecture (impure `update`, hardcoded styling, misplaced platform code).
- **Documentation site** — the milestone notes are excellent raw material, but they are unindexed and unsearchable outside `grep`.

---

## Non-goals

Stated so nobody spends a weekend on them:

- **A DSL or markup language.** Rust with builder methods and the `column!`/`row!` macros is the API. No `.frus` files, no macro that reinvents syntax.
- **A bespoke CLI.** `cargo` is the tool. `cargo generate`, `cargo apk`, `wasm-bindgen` are existing tools; we don't wrap them in `frus doctor`.
- **A software rasterizer.** `wgpu` covers every target we care about.
- **Framework logic in another language.** Non-Rust code is limited to thin platform adapters with no logic of their own.
- **Cloning another framework's API surface verbatim.** We port what works and fix what doesn't. "Framework X does it this way" is a starting point in a discussion, not an argument that ends one.

---

## How to pick something up

1. Find an item here or in [issues](https://github.com/KalybosPro/frus/issues).
2. Comment saying you're taking it. If there's no issue, open one first — especially for 🔴 items.
3. Read the relevant part of [ARCHITECTURE.md](ARCHITECTURE.md) and `grep docs/` for prior decisions in that area.
4. Build it, test it, and follow [CONTRIBUTING.md](CONTRIBUTING.md).

Small first PRs are welcome and are the fastest way to learn the codebase's conventions. So is an issue that just says "this API is worse than the one I'm used to, here's why" — that is useful work.
