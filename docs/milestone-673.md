# Milestone 673 — `frus emulators` and `frus clean`

## Objective

Two commands the reference's tool has and frus did not: one for the emulators — milestone 671
gave the daemon the methods, but nothing on the command line used them — and one to start a build
over from nothing.

## What was done

- **`frus emulators`**: the emulators that can be started, as a table, with what to type next.
  `--launch <id>` starts one (`--cold`: without its saved state); `--create [--name <name>]`
  makes an Android emulator, installing the emulator and a system image first if they are
  missing. The work is the daemon's (`emulator.*`, milestone 671), said for a person.
- **`frus clean`**:
  - when the build folder is the application's own, all of it goes (`cargo clean`);
  - when it is shared — `CARGO_TARGET_DIR`, a workspace's — only this application's builds go
    (`cargo clean -p <package>`, for debug and release, for this computer, the web and Android).
    `cargo clean` there would delete every other project's build too;
  - the web build's generated glue (`web/pkg`) and `frus run --hot`'s shadow package go as well.

## Verified

- On this machine: `frus emulators` with none ("No emulators available", and how to make one);
  `--create --name "frus test"` made `frus_test` from the installed Android 16 image; the list
  showed it; `--launch frus_test --cold` started it, and `frus devices` showed it booting. It was
  stopped and deleted afterwards.
- `frus clean` in a copy of the counter example with the repository's shared build folder: its
  builds went (nine entries down to three, the three left a program the manifest no longer has),
  the 61 of the frus crates stayed. In a copy with its own `target/`: all of it went (4 GB).
- Unit tests: the list and its hints, the options (`--launch` with `--cold`, `--create` with
  `--name`, the combinations refused), whether a build folder is the application's own.
