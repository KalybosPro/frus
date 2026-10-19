# Milestone 677 — `frus run --hot` on Linux and macOS

## Objective

`frus run --hot` was built and tried on Windows. Its code had a branch for each system; this
milestone runs it on Linux (in WSL, with its own display) and looks at what differs.

## What was found

- **`q` would have left the application running.** On Windows, `frus` stops the engine and every
  process under it (`taskkill /T`). Elsewhere it stopped the engine alone, and the application the
  engine started went on.
- **The engine does not build on a Linux without OpenSSL's headers.** Installing it ended in a
  page of build output from `openssl-sys`. frus cannot install them: a package manager needs the
  administrator's password.
- `frus doctor` ran on Linux as it does on Windows: Rust and its targets, the desktop, the web,
  VS Code through WSL, the devices.

## What was done

- **The engine leads a process group of its own** on Linux and macOS, and stopping it stops the
  group: the application goes with it, on `q` as on `R`.
- **The OpenSSL headers are checked before the engine is built**, through `pkg-config`; without
  them `frus run --hot` stops at once with one line — `sudo apt install pkg-config libssl-dev`, or
  the Fedora command — and `frus doctor` says the same under hot reload.

## Verified

- In WSL (Ubuntu 24.04), which has no OpenSSL headers and no password-less `sudo`: `frus doctor`
  listed the missing headers with the command; `frus run --hot` stopped with the same line.
  frus-cli's tests and clippy, on Linux and on Windows.

## Left

- **The full hot reload on Linux was not run**: it needs the headers installed, which takes the
  machine's password. With them, the run is the same as on Windows, the process group aside.
- macOS: not tried.
