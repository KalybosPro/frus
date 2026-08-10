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

276 milestones in. The framework runs real, non-trivial applications on desktop and Android, and functional ones on the web. What exists is genuinely built, not stubbed: layout, text with IME, drag-and-drop with live reflow, data tables, charts, pickers, navigation with spring transitions, theming, i18n/RTL, accessibility, animation, async effects with typed JSON, and golden-image testing.

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

### Quality

- 🟢 **Clear the clippy backlog.** ~50 warnings, mostly `type_complexity`, `field_reassign_with_default`, and `map_or` simplifications. Then make `-D warnings` blocking in CI.
- 🟢 **Format the tree.** One `cargo fmt --all` commit, then make the fmt check blocking. Do this on its own, never mixed with a feature.
- 🟡 **Widen golden coverage.** Many widgets have logic tests but no pixel test. Every widget should have at least one.
- 🟡 **Benchmarks.** There is no performance harness at all. Frame time, layout cost, scene build, and text shaping all need one before any optimization claim can be honest.
- 🟢 **Fix the broken intra-doc links.** `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` reports ~11 errors — links to private items and one to a type that no longer exists. Then make the strict rustdoc pass blocking in CI.
- 🟢 **Fix the `dead_code` warning** in `frus-demo` (`grid_first_error`) — either use it or drop it.

---

## Medium term

### New platforms

- 🔵 **iOS shell.** The most valuable single contribution available. `frus-shell` is the only crate that would change: a `UIViewController` host, Metal surface via `wgpu`, touch input, IME, lifecycle, safe-area insets. The layering is designed for exactly this, and Android is the worked example to follow.

  **Groundwork landed** (see `docs/milestone-276.md`): `frus-shell` now has named platform `cfg` aliases (`desktop` / `android` / `ios` / `web`) via its `build.rs`, so iOS no longer falls silently into the desktop branch, and a `run()` entry point exists behind `#[cfg(ios)]`. An advisory CI job builds for `aarch64-apple-ios-sim` and `aarch64-apple-ios`. What remains is the actual platform integration — lifecycle, safe-area insets, IME/soft keyboard, `os_log`, UIKit accessibility, and `.ipa` packaging.
- 🔴 **Native macOS shell.** winit already covers macOS as a desktop target; a native shell would be about menu bar, window chrome, and platform conventions rather than rendering.

### Framework depth

- 🟡 **Text rendering edge cases.** Bidi runs, complex scripts, font fallback chains, emoji, vertical metrics. Real bug reports welcome here.
- 🟡 **Scrolling physics — the visible half.** The behaviour landed in `docs/milestone-277.md`: `ScrollPhysics::{Bouncing, Clamping}`, the platform's own fling curves, a real rubber band, and per-app / per-area overrides. What is missing is the **overscroll glow** a clamping platform draws when a fling reaches an end — nothing is painted there today. Also unported: the second bouncing deceleration profile, and a least-squares velocity estimate over a window (the release velocity is still an exponential smoothing of the last two samples).
- 🔴 **Rebuild memoization.** `view` rebuilds the whole tree each frame. It has not been a bottleneck yet, but it will be. Wants benchmarks first, then a design.
- 🟡 **Renderer batching.** Fewer draw calls for scenes with many small primitives; atlas the common shapes.
- 🟡 **More widgets.** Rich text editing, video, maps, virtualized lists for very large datasets.
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
