# Milestone 571 — A quiet console on the Web

The browser's console, pasted while using the web demo, held five lines nobody had asked for. A
sixth was found by capturing it. None is a failure; each is noise on every page load, and one broke
a rule of the repository.

| what the console said | where it came from | now |
|---|---|---|
| `The powerPreference option is currently ignored when calling requestAdapter() on Windows` | the renderer asked for `HighPerformance` on every target; Chrome, on Windows, ignores the hint and says so | a browser is asked for no preference; the desktop and Android keep `HighPerformance` |
| `Adaptateur GPU : AdapterInfo { … }` | a log line in **French**, in a repository that is English everywhere but its French README | `GPU adapter: …` |
| `failed to get system locale, falling back to en-US` (twice) | cosmic-text builds a font system and asks `sys-locale` for the language; `sys-locale` reads the browser's only with its `js` feature, so it answered nothing — once per font system | the feature is on for `wasm32` in `frus-text`; the browser's language is found and the warning is gone |
| `Failed to load resource: 404` | the page has no icon, so the browser asked for `/favicon.ico` | `<link rel="icon" href="data:,">`, an empty one, until the page has an icon of its own |
| `MSAA: 4×` | an info line, kept | unchanged |

`sys-locale` was already in the tree — cosmic-text pulls it — so the only new lines in
`Cargo.lock` are the three crates its browser code uses (`js-sys`, `wasm-bindgen`, `web-sys`), which
a web build carries anyway.

## Verification

The console was captured in headless Edge over the DevTools protocol before and after, for twelve
seconds after the page loads: six messages before (one error, two warnings from the browser and two
from cosmic-text, and two infos), two after — the adapter and the sample count, both info lines.

clippy `-D warnings` (workspace and `frus-demo --features shots`, with `--locked`), the tests of
`frus-text` and `frus-gpu`, and a `wasm32` check pass.

## Left

- The demo page still has no icon of its own.
- Whether the locale the font system now finds is used for anything visible on the Web was not
  looked into: it only feeds cosmic-text's shaping defaults, and the application's own language
  choice (`FrusApp::languages`) is separate.
