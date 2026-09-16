# Milestone 535 — A page that says what it is before the canvas does

Answers [#11](https://github.com/KalybosPro/frus/issues/11).

`crates/frus-hello/web/index.html` is the first thing anyone sees when they try frus in a
browser, and until now it was a canvas on a flat background and nothing else — no title on the
page beyond the `<title>` tag, no line saying what was about to run, no way back to the
repository, and a blank rectangle for as long as the wasm module took to fetch and start. The
`navigator.gpu` check the issue's first bullet asked for had already landed since the issue was
opened; what was left were the other two.

## The decision

**A small header, not a landing page.** The point of this example is a single static file
`python3 -m http.server` can serve, and the counter itself is the thing to look at — so the
addition is one `<header>` fixed to the top: the name, one line saying what is running and how
(a counter, through WebGPU, no build step beyond `wasm-bindgen`), and a link back to the
repository. It sits over the canvas rather than pushing it down, `pointer-events: none` on the
bar itself so it never steals a click meant for the app underneath, and `auto` on the link alone
so it stays reachable.

**The loading state is the markup, not a flag.** `#loading` carries no `hidden` attribute in the
document — it is what a viewer sees the instant the page paints, before any script has run — and
the script's job is only to take it away once there is something to show instead: the wasm
module fetched, instantiated and started (`import(...).then((m) => m.default())`). A spinner and
one line of text, the same size and place `#unsupported` already used, so the eye lands in the
same spot whichever one shows.

**A failed fetch or start gets a message, not a silent hang.** The `navigator.gpu` branch was
already a dead end reported in place; the module import had none. A `.catch` on the import chain
now reuses the `#unsupported` panel's markup — its heading and paragraph are rewritten in place —
for the one failure that panel wasn't built for: the module missing or throwing on start, which
on this example most often means `web/pkg/` hasn't been built yet (see `web/README.md`).

## Verification

Built and served for real, not read as markup. `cargo build -p frus-hello --target
wasm32-unknown-unknown --profile web-release`, then `wasm-bindgen` into `web/pkg/`, then
`python3 -m http.server` from `crates/frus-hello/web/`, served on `localhost` — a secure context,
so `navigator.gpu` is present to check against.

**Headless Edge, screenshotted.** The header renders where it should and stays legible over the
dark canvas; a DOM dump right after load shows `#loading` and `#unsupported` both `hidden`,
which is the resolved state — the import chain finished, past `.then` and its inner `.then`,
without needing the `.catch`. The page's own script did what it was written to do.

**The `#unsupported` branch is unchanged code**, apart from the one line added beside it
(`loading.hidden = true`) — its condition is synchronous and was already exercised before this
issue. I did not manage to force `navigator.gpu` out of a real headless Chromium build to
re-photograph it (`--disable-features=WebGPU` and `--disable-gpu` both leave the API in place in
this Edge build; only actually running without WebGPU support, which none of the machines here
lack, removes it) and did not want to fake the condition by editing the shipped script for a
screenshot. The branch is one `if` with two assignments on each side and was read rather than
re-proven.

**Found on the way, not fixed.** With WebGPU present, `requestDevice` in this Edge build (headless,
software-backed) fails: `The limit "maxInterStageShaderComponents" with a non-undefined value is
not recognized.` — logged by `frus-shell`'s own `log::error!("failed to initialise the renderer
(Web): …")`, so the module's `start()` still resolves (the failure is async, inside the renderer
the loop only tries to build once running) and the loading overlay is correctly taken away,
leaving a canvas that never draws. Whether a real, non-headless Chrome or Edge on real hardware
hits the same limit mismatch is not established here — the README's own "Chrome or Edge 113+" is
about a machine this repository does not have a way to test from. Left exactly as found; worth
its own issue if it turns out to be more than this environment's software WebGPU stack lagging
the spec `wgpu` targets.

## What is left

- **A renderer that fails after the module has started** — the case just above — still leaves a
  blank canvas with nothing on the page to say so. Catching that needs a channel from
  `frus-shell`'s async Web init back to the page, which is `frus-shell` work, not this file's.
- The loading overlay was never caught mid-flight in a screenshot: a local server has no latency
  to catch it against. The markup guarantees it is shown before the script runs; a slow
  connection was not built to prove it further.
