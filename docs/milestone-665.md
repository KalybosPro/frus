# Milestone 665 — `frus daemon` and `frus run --machine`

## Objective

`frus run` (milestones 661 to 663) is driven by a person: it prints text and reads keys. An
editor needs the same thing in a form a program can read. It has to know the devices, to fill a
device picker and keep it current as phones are plugged in. It has to start a run on one, show
the application's log in its own console, and ask for a reload or a stop. The reference's tool
answers this with a daemon speaking JSON and a machine mode of its `run`; this milestone gives
frus the same two.

## What was done

- **JSON lines** (`frus-cli/src/json.rs`): a value, read from a line and written to one. The tool
  still has no dependencies.
- **Where output goes** (`report.rs`): every message of `frus run` goes through one place. By
  default it is text; with `--machine`, standard output carries JSON lines only:
  - the tool's messages become `app.progress` events, the application's log becomes `app.log`;
  - the desktop program's output is read through pipes instead of being handed the terminal;
  - the tools frus runs (cargo, cargo-apk, wasm-bindgen) write to standard error.
- **`frus run --machine -d <device>`** reports `app.start`, `app.started`, `app.log`,
  `app.progress`, `app.restarted` and `app.stop`, each with the run's `appId`. It reads commands
  on standard input instead of keys: `app.restart` (with `fullRestart` for a restart afresh) and
  `app.stop`, each answered with its `id`.
- **`frus daemon`** answers `daemon.version`, `daemon.shutdown` and `device.getDevices`. After
  `device.enable` it looks at the devices every three seconds and reports `device.added` and
  `device.removed`; a phone that becomes authorised is removed as it was and added as it is.
- The protocol is documented in the CLI's README.

## Verified

- `cargo test -p frus-cli`: 60 tests. They cover:
  - JSON read and written back, with quotes, tabs and line breaks;
  - broken text refused;
  - device changes;
  - a device described for an editor;
  - an editor's commands mapped to keys;
  - errors told from information in a program's standard error.
- Driven by a script as an editor would, on Windows with a phone connected:
  - the daemon answered its version, listed four devices (the phone among them), refused an
    unknown method with an error, and ended on `daemon.shutdown`;
  - `frus run --machine -d windows` on a copy of the counter example reported `app.start`,
    `app.started`, eight `app.log` lines, answered `app.restart`, reported `app.restarted`
    (3.5 s, the state kept), then `app.stop`;
  - **not one line** of standard output failed to parse as JSON.

## Left

- **The VS Code extension** that uses this: milestone 666.
- **Emulators in the daemon** are listed but not started from it (`emulator.launch`): starting
  one is `frus run -d <avd>`.
