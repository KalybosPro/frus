# Milestone 679 — Hot reload on the web

## Objective

`frus run --hot` patched each saved change into the running application on the desktop
(milestones 669, 670). On the web, `r` rebuilt and reloaded the page with its states kept
(milestone 666): the page reloaded. Issue #287: patch the running page instead.

## How it works

- **The engine builds the page's module**: `frus run --hot -d web` runs `dx serve --web
  --hot-patch` on the shadow package of milestone 670, on a free port. Only its socket is used;
  its own page is not.
- **`frus run` serves the page as without `--hot`**: the application's `web/index.html` with its
  script (the console, the keys, screenshots, reloads), and the engine's build mounted in it —
  `pkg/` and `wasm/` are the engine's `wasm` folder, so the page loads `pkg/<library>.js` as
  always and a patch is fetched from `/wasm/<patch>.wasm`, the address the engine gives it.
- **`frus run` passes the patches on**: it reads the engine's socket itself (a small WebSocket
  client of its own) and keeps the latest patch message; the page's script asks
  `/__frus/patch?after=<n>` for anything newer, as it asks for the keys' commands, and hands it
  to the shell (`window.__frusPatch`).
- **The page applies the patches**: the shell hands the message's table to
  `subsecond::apply_patch`, which fetches the patch module, compiles it and links it into the
  running module's memory and function table. The next frame builds the interface again
  through the patched functions, every state where it was.
- **A reloaded page keeps the code**: it asks from the start and is given the latest patch;
  patches are cumulative.
- `R` reloads the page afresh; a change no patch can carry is built in full and the page reloads
  onto it, its states kept.

## What was found

- **The application started twice.** The engine's module glue loads the module itself as it is
  imported, and the page then calls its `default()`, which loaded a second instance beside the
  first. The server rewrites the glue under `--hot` so that `default()` waits for the load
  already started.
- **The program needs a `main` on the web too**: the shadow compiles `src/lib.rs` as a binary.
  With the `hot-reload` feature, `frus::main!` adds an empty one for wasm; the application still
  starts from its `#[wasm_bindgen(start)]` function.
- **The engine names the page's module after the program**, so the shadow's program is now named
  after the library (`my_app`, not `my-app`), which is the name the page loads.
- **A page that connects later is not reliably given the latest patch by the engine.** The page
  first opened the engine's socket itself; a page reloaded with `R` came back with the code it
  was built with, the engine having sent it nothing, and a client of the socket's own was sent
  the latest patch at one time and nothing at another. `frus run` now keeps the latest patch and
  gives it to every page that starts, and the page no longer talks to the engine.
- **The test runs had their own traps.** Twice early on the page stopped drawing around the time
  a patch came; once headless Edge was seen reporting the canvas occluded around a screenshot
  (frus stops drawing an occluded window), and the test scripts had left headless browsers
  running that loaded every new run's page beside the one under test. With the page kept
  focused (`Emulation.setFocusEmulationEnabled`) and those browsers stopped, it did not happen
  again; which of the two it was is not established.

## Verified

In headless Edge with WebGPU, on a counter application with its auto-increment running, under
`frus run --hot -d web-server`:

- the app bar's title changed four times in a row, each change saved to the source shown in the
  running page; the count went on from where it was; a value set in the page's script before the
  first change was still there after the last (no reload); each patch was applied once;
- `R` reloaded the page afresh (the value gone, the count from the start) with the last patched
  title, and the next change saved was patched into the reloaded page;
- the first patch of a run took 15–38 s (a 137 MB debug module), the next ones 2.5–5 s.

On the desktop, `frus run --hot` (whose start was shared with the web's) patched the same change
in 3 s, the count going on.

## Also fixed

- **A build folder named with forward slashes** (`CARGO_TARGET_DIR=D:/x/target`, as a shell on
  Windows often writes it) made `frus run --hot` stop at once: the junctions of the shadow
  package are made by `cmd /C mklink /J`, which read `/x` as an option. The paths are given
  with backslashes.

`cargo test -p frus-cli`: 91 tests, on Windows and on Linux (WSL). Not run: the web hot reload
on Linux and macOS.
