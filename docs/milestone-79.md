# Milestone 79 — State-preserving live reload (§13)

## Analysis

The DX weakness identified (goal 4): iterating on the `view` means relaunching
and re-navigating. The ideas book points at the Elm advantage: "the state is a
single struct; serialise it, reload, rehydrate". The first tier — **relaunching
on recompilation with the state preserved** — without hot-patching (`subsecond`)
or a fragile-ABI dylib: pure cargo.

## Architecture

- **`Application`** gains two hooks with neutral defaults: `save_state() ->
  Option<Vec<u8>>` (a free format, owned by the app) and `restore_state(&[u8])`
  (called **before `init`**; the bytes come from a *different version of the
  code* → tolerate, never panic).
- **`frus-shell/reload.rs`**: under `FRUS_WATCH=1` (debug builds), a
  `ReloadWatcher` polls the **executable's mtime** (700 ms); when `cargo build`
  replaces it: capture the snapshot → a temp file → `spawn` the new binary with
  `FRUS_HOT_STATE=<path>` → `exit(0)`. At start-up, `restore_from_env` reads and
  **consumes** the snapshot.
  - A trap avoided: after replacement, `/proc/self/exe` points at the deleted
    inode ("(deleted)") — so the path is **captured at start-up**.
  - The loop wakes through the centralised `idle_control_flow()` policy (the min
    of the long-press and reload-poll deadlines — replacing the two scattered
    `set_control_flow` calls).
- **Demo**: a versioned line-by-line snapshot (`frus-demo-state v1`) — tasks,
  draft, filter, theme (mode + seed), tab, pushed screen; `init` skips the disk
  reload when the state came from a snapshot (otherwise `Loaded` would overwrite
  it).

## Usage

```sh
FRUS_WATCH=1 cargo run -p frus-demo     # terminal 1
cargo watch -x 'build -p frus-demo'     # terminal 2 — edit, save, watch
```

A caveat on native Windows: the running executable is locked (cargo cannot
replace it) — the flow targets Linux/WSL/macOS; Android is unaffected.

## Validation

- Tests: `restore_reads_and_consumes_the_snapshot`,
  `watcher_requires_opt_in_and_tracks_mtime` (shell);
  `live_reload_state_round_trips` (demo: tasks/draft/filter/theme/seed/route all
  survive, `init` no longer emits `Loaded`, and a corrupted or differently
  versioned snapshot is ignored without panicking). 275 → 278.
- **A real end-to-end run (WSL)**: the demo launched under `FRUS_WATCH=1`,
  `touch` on the binary → the logs `binary recompiled: relaunching` then `state
  rehydrated (68 bytes)`, and a new pid. The loop works.

## The rest of §13

In-process hot patching (`subsecond`) if the need goes beyond relaunching; a
`cargo new` template; and for the inspector: click-to-freeze selection, and the
retained state.
