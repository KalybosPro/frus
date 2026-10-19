# Milestone 674 — `frus run --hot` says where an error is

## Objective

Under `frus run --hot`, a saved change that did not compile was said without its place, and at
times not at all. A developer saves, the window does not change, and nothing says why or where.

## What was found

- **The engine names an error and drops its place.** `dx` prints `cannot find value `x` in this
  scope`, then `Build failed`: no file, no line. Neither `--verbose` nor `--json-output` carries
  the span either.
- **An edit saved just after the application came up was missed.** The engine starts watching a
  few seconds after it launches the application; a change made in between reached the shadow
  copy, and no event told the engine. The next save was patched as usual. This was what looked
  like "no error at all".
- `FRUS_HOT_TRACE=1` (added here) printed the engine's own lines and showed both.

## What was done

- **The errors, located.** When the engine says a build failed, frus runs `cargo check
  --message-format=short` on the shadow package (same features, same build folder) and says
  each error with its place, the path made the application's own:
  `<app>\src\features\counter\widgets\count_display.rs:40:34: error[E0425]: …`, which an
  editor's terminal turns into a link. The application goes on running the code it had.
- **The blind moment covered.** Eight seconds after the application is up, every copied source
  changed since the launch is written again, unchanged: the engine, watching by then, sees it.
- **`FRUS_HOT_TRACE=1`**: every line the engine prints, on standard error.

## Verified

- On a copy of the counter example: an error saved six seconds after the start was reported —
  the engine's message, then the error at `count_display.rs:40:34` in the application's own
  `src`; fixing it patched the application in 2.8 s.
- Unit tests: the path of an error made the application's (relative, absolute in the shadow, any
  other left alone; warnings and other lines ignored), and a file changed after the launch
  written again.
