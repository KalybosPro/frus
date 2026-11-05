# Milestone 754 — Hot reload is how `frus run` runs

## Objective

`frus run` reloaded a change by restarting the application: `r` (or a save in VS Code) rebuilt
the program, stopped it, and started the new one on the same page with the state of every
component that keeps one (`State::save`, milestone 663). True hot reload — the change patched
into the running process, nothing stopped, every state where it was — has worked on the desktop,
the web and Android since milestones 669 to 680, but only behind `--hot`. A developer who did not
know the flag saw the application close and reopen at every change, and took that for frus's hot
reload.

The patching becomes the default, as a developer expects from a framework's development loop.

## What changes

`frus run` decides whether to patch once the device is known (`run::hot_wanted`):

| asked | debug build | release build | device without it (iOS simulators) |
|---|---|---|---|
| nothing | patches | restarts | restarts |
| `--no-hot` | restarts | restarts | restarts |
| `--hot` | patches | patches | error, as before |

- **A release build restarts by default.** It is the program that ships, and is run as it
  ships; the patching build keeps every symbol and is not one. `--hot` still asks for it.
- **A machine that cannot install the engine still runs.** By default, if the engine is missing
  and cannot be installed (Linux without OpenSSL's headers, a failed `cargo install`), the run
  says why once, names `frus doctor --fix`, and goes on with restarts. Asked for with `--hot`,
  the same failure is an error, as before.
- **`r` under hot reload** told the person there is nothing to press. An editor asks for a
  reload at every save (VS Code's reload on save), so the line would have come at every save:
  in machine mode it is not answered.
- `--hot` and `--no-hot` together, or either twice, is an error.

The VS Code extension's `hot` launch attribute has no default any more: left out, frus decides;
`false` passes `--no-hot`, `true` passes `--hot`.

## The engine

The engine is still the fork of Dioxus's CLI that frus installs into `~/.frus/tools`
(`hot::ENGINE_REPO`, `hot::ENGINE_REV`): it carries the Windows stub fix (DioxusLabs/dioxus#5967)
and the Android command-file fix (DioxusLabs/dioxus#5977). When both are in a release of
`dioxus-cli`, the engine becomes that release, and nothing else changes.

## Costs, said where they are met

- The first run installs the engine: about twenty minutes, once. The message that announces it
  now gives the length and names `--no-hot`.
- On Android, the engine's package replaces the one `frus run --no-hot` or `frus build apk`
  installs, and the other way round: Android will not replace one with the other (the engine's
  Android project has its own version code, milestone 680), so switching uninstalls, and the
  application's data on the device goes with it.

## Tests

- `run::tests::hot_reload_is_the_default_of_a_debug_build`: the table above.
- `tests::run_options_parse`: `--hot`, `--no-hot`, nothing, and the conflicts.
- The extension's `adapter.test.ts`: `hot: false` → `--no-hot`, `hot` left out → nothing.
