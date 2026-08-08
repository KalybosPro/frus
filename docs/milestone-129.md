# Milestone 129 — Web target (wasm + WebGPU)

## Analysis

frus targeted desktop and Android through winit + wgpu — the two native backends. The
**most universal platform** was missing: the browser. winit and wgpu both support
`wasm32-unknown-unknown` + **WebGPU**; the app (`view`/`update`) is already pure and
platform-independent. The work is **entirely in the shell layer**.

This milestone's goal: that the whole stack (**core → widgets → gpu → shell → app**)
**compiles** for `wasm32-unknown-unknown` and that an input-driven app (`frus-hello`, the
counter) runs in the browser, with no native regression.

## Technical decisions

- **Three platforms, not two.** The shell distinguished desktop (`not(android)`) from
  Android. The Web becomes a 3rd target: the **desktop-only** subsystems (the `arboard`
  clipboard, AccessKit accessibility, the `env_logger` logger, live reload) move from
  `not(android)` to `not(any(android, wasm32))`; their no-ops cover Android **and** Web.

- **A portable clock.** `std::time::Instant` **panics** on `wasm32-unknown-unknown`. The
  whole shell switches to `web-time::Instant` (an alias for `std` natively,
  `performance.now()` on the Web) — one API, zero `cfg` in the bodies.

- **Asynchronous GPU init.** The Web cannot **block** (`pollster::block_on`). On wasm,
  `resumed` launches `Renderer::new` (already `async`) through
  `wasm_bindgen_futures::spawn_local`; the ready renderer is dropped into an
  `Rc<RefCell<Option<…>>>` and picked up on the first frame. Native keeps `block_on`.

- **The canvas managed by winit.** `WindowAttributesExtWebSys::with_append(true)`: winit
  creates and appends a `<canvas>` to the `<body>`. No manual DOM plumbing.

- **A browser entry point.** `frus_shell::run_web` hands the loop to the browser
  (`EventLoopExtWebSys::spawn_app`, driven by `requestAnimationFrame`). The app exposes
  `#[wasm_bindgen(start)] fn start()` — no other difference.

## Implementation

- `frus-shell`: `Cargo.toml` (Web deps: `wasm-bindgen(-futures)`, `web-sys`,
  `console_error_panic_hook`, `console_log`; `web-time` throughout; `pollster` restricted
  to native; desktop-only excluded from wasm); `run_web`; asynchronous
  `resumed`/`RedrawRequested`; the `web-time` clock in
  `app`/`gesture`/`subscription`/`reload`.
- `frus-hello`: a `#[wasm_bindgen(start)]` entry, `run_desktop` restricted to native, a
  no-op binary outside native; a `web/` folder (`index.html`, a build `README.md`,
  `.gitignore`).

## Verification

- **Compiles** for `wasm32-unknown-unknown` (debug **and** release): the whole stack,
  WebGPU included. `frus_hello.wasm` ≈ 7.5 MB raw (before `wasm-bindgen` + `wasm-opt` +
  gzip).
- **Native intact**: `cargo test --workspace` stays **green** (no regression across the
  ~330 tests), the desktop build is as before.
- The browser build: `wasm-bindgen --target web` → `web/pkg/`, served from `localhost`
  (see `crates/frus-hello/web/README.md`).

## What's left

- **Verification in a real browser** (the *seeing* step: I cannot launch a browser here)
  — Chrome/Edge 113+ on `localhost`.
- **Effects & subscriptions** on the Web: native threads (`std::thread::spawn`) do not
  run on wasm → an app with a subscription (an `every` animation) would not tick yet. To
  be ported through `spawn_local` + browser timers.
- **Clipboard / IME / accessibility** on the Web (browser APIs) — separate pieces of
  work.
- Slimming the `.wasm` down (`wasm-opt -Oz`, `panic=abort`, splitting the wgpu features).
