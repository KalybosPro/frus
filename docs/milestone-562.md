# Milestone 562 — The demo runs in a browser

## Objective

Issue #31 asked for a "done when" that could be watched: the demo, served on the web, opened
directly at a second screen, its address changing as it is used. The demo did not build for the
web — a fact carried since milestone 556 — so it could not be the example.

## What changed

- **The demo compiles for `wasm32-unknown-unknown`.** What stopped it: `frus_shell::main!` names
  `wasm_bindgen`, which the crate did not depend on (the same line `frus-hello` has), and its
  `Save` and `Load` reached for `std::env::temp_dir`, which **panics** where there is no
  filesystem — at start-up, because the demo loads its tasks then.
- **The tasks are kept in `localStorage` in a browser**, the file they were kept in elsewhere.
  `storage.rs` keeps its two functions and their signatures; the path is the name of the
  entry there. The encoding is shared and has a test of its own.
- **A page to serve it** (`crates/frus-demo/web/index.html`, from `frus-hello`'s) and a README
  that says what the address does.
- **It was built, served and driven**: milestone 561.

## Left out

The demo's Save and Load are synchronous work run through `host::spawn`, which on the web is
run by `spawn_local` on the loop's own thread; a `localStorage` write is quick, so nothing waits.
A slow effect on the web would want `Command::perform_async`, which `host::spawn` does not use.

The layout was looked at in a headless browser whose narrowest window is 500 pixels. Nothing
about how the demo looks at a phone's width, or in a real browser window, was judged here — only
that it starts, draws, takes a tap and a typed line, and follows its address.
