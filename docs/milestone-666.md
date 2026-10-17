# Milestone 666 — The VS Code extension

## Objective

The reference's editor integration is what most developers know it by: a device picker in the
status bar, F5 to run on that device, the application's log in the Debug Console, and a reload
on every save. Milestones 661 to 665 built all of it in the command-line tool; this milestone is
the editor side, in `editors/vscode`.

## What was done

- **A device picker** in the status bar, fed by `frus daemon`: this computer, the phones and
  emulators, the browsers, kept current as phones come and go. The choice is remembered per
  workspace.
- **F5** with no `launch.json`: a `frus` debug type whose configuration is completed with the
  selected device and the application's directory (the folder of the open file's nearest
  `Cargo.toml`). A `launch.json` can pin a device, a release build or arguments.
- **A debug adapter** (`src/adapter.ts`) that drives `frus run --machine`. It needs no library: it
  speaks the Debug Adapter Protocol itself, as the tool speaks JSON, and VS Code runs it inline.
  - `app.log` lines reach the **Debug Console**, errors as errors;
  - the build's output and the tool's messages appear in its console category;
  - the debug toolbar's restart restarts with every state afresh;
  - a ⚡ button, Ctrl+F5, and **saving a Rust file** reload with the kept states;
  - stopping the session stops the application.
- **The `frus` program**: the one `frus.path` names or the one on the PATH, installed with
  `cargo install frus-cli` when there is none. One older than the editor protocol is reported
  with what to do, rather than failing silently.

## Verified

- `npm test`: the adapter with a stand-in for `frus`:
  - the capabilities, and the command line launch builds;
  - log lines to stdout and stderr;
  - reload, restart and stop sent as `app.restart` and `app.stop`;
  - a single `terminated`, and nothing sent after it.
- The same adapter against the **real** `frus run --machine` on a copy of the counter example
  (`FRUS_E2E_FRUS`, `FRUS_E2E_APP`): it launched, reloaded ("Reloaded in … s"), carried the
  application's log, and terminated on disconnect.
- `vsce package` builds `frus-0.1.0.vsix` (25 KB).

## Left

- **Not yet tried inside VS Code itself**: the status bar, the picker and F5 were exercised
  through the adapter and the daemon, not through the editor's own interface.
- **Publishing** to the Marketplace: it needs a publisher account.
- **Starting an emulator from the picker**: an AVD is listed, and F5 starts it through
  `frus run -d <avd>`.
