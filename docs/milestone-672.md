# Milestone 672 — `frus doctor`

## Objective

The reference's first command on a new machine is its doctor: a line for each part of the
toolchain, what is missing and how to get it. frus installs what a command needs when it needs
it, but nothing said up front whether a machine was ready, and a missing linker or SDK showed up
as a failed build. `frus doctor` says it before.

## What was done

- **The parts**, each `[✓]` ready, `[!]` usable but incomplete, or `[✗]` missing:
  - frus itself, its version and where it is;
  - Rust: `rustc`, `cargo`, `rustup` (frus adds targets through it);
  - the desktop build: Visual Studio's C++ tools on Windows (found with the installer's own
    `vswhere`), `pkg-config` on Linux;
  - Android: the SDK, platform-tools, a platform, build-tools, the NDK, Java, the Rust target,
    cargo-apk, the emulator;
  - the web: the wasm target, wasm-bindgen (the one on the `PATH` and frus's pinned ones), the
    browsers frus knows;
  - the hot-reload engine of `frus run --hot`;
  - iOS: said plainly, not yet possible;
  - VS Code and the frus extension;
  - the devices, and a hint when no phone is connected.
- **It changes nothing** without `--fix`. **`-v`** shows everything it found; without it, only
  what is wrong.
- **`--fix`** installs what is missing with the installers the other commands already use: Rust
  targets, the Android SDK's parts, cargo-apk, the hot-reload engine. It never downloads what the
  doctor found fine: it completes the Android SDK only when that check found a problem, for the
  application's target level when run in one. iOS and the VS Code extension are reported and not
  counted as issues a fix could take away.
- **`--machine`**: the same as JSON, for an editor.
- **VS Code**: **frus: Run Doctor** opens `frus doctor -v` in a terminal.

## Verified

- On this machine: everything ready, iOS reported; `-v` listed the SDK's parts, both
  wasm-bindgen versions, the engine, the extension and the devices.
- With `FRUS_HOME` on an empty folder: the engine reported missing and counted as an issue.
- `--fix` on a complete machine: 2.6 s, nothing downloaded. A first version would have
  downloaded an Android platform: the SDK installer looks for `android-36`, the installed one is
  `android-36.1`. Gating the fix on the check's own finding is what avoids it.
- `--fix` with the engine missing installed it.
- Unit tests: a part's status from its findings, the summary and the details, an issue a fix
  cannot reach, the JSON.
