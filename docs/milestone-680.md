# Milestone 680 — Hot reload on Android

## Objective

`frus run --hot` patched each saved change into the running application on the desktop
(milestones 669, 670) and the web (milestone 679). On Android, `r` restarted the application with
its states handed back (milestone 663). Issue #286: patch the running application on the device.

## How it works

- **The engine's Android project, with frus's activity**: `dx serve --android --hot-patch`
  builds the application into a Gradle project of its own, installs it and starts it, then
  pushes a patch library to the device for each saved change. Its project starts the program
  through its own activity; frus writes into the shadow package a `Dioxus.toml` naming a
  `MainActivity.kt` that is a plain `android.app.NativeActivity` — what frus's Android shell
  runs in — and the application's package identifier. The application's own files are not
  touched: those two names are kept out of the shadow's links.
- **The program**: on Android too the shadow compiles `src/lib.rs` as a binary, so `frus::main!`
  adds an empty `main` under the `hot-reload` feature; the application still starts from
  `android_main`.
- **The engine's settings**: the engine cannot give an Android application an environment; it
  pushes its settings (the patch server's port among them) to `/data/local/tmp/dx/.env`, which
  the shell reads before connecting. The same file tells the shell it runs under `frus run`, so
  the keys reach it as without `--hot` (the engine cannot add the launch extra `frus run` uses).
- **What a build needs** is installed as for `frus run` on Android — the Rust targets, the SDK
  and the NDK — and the JDK the engine's Gradle project needs.
- **The log, the keys, screenshots** go as without `--hot`: the device's log is read by
  `frus run`, commands go through the application's own storage, `s` takes the screen.

## What was found

- **The engine could not link for Android on Windows.** It passes the linker its arguments in a
  command file, each one quoted; MSVC's `link.exe` takes backslashes as they are, but clang reads
  the file the GNU way, where a backslash escapes the next character — every path lost its
  separators (`…shadow680/targetaarch64-linux-androidandroid-devlibdeps-….a`). Fixed in frus's
  fork of the engine (branch `frus/v0.7.10-android-windows`, on top of the stub fix): backslashes
  and quotes are escaped for targets that are not Windows. `ENGINE_REV` names that commit.
- **The engine's package says version 1**; the one `frus run` installs without `--hot` takes
  its version from the manifest, and Android refuses to replace it with a lower one
  (`INSTALL_FAILED_VERSION_DOWNGRADE`). `frus run --hot` removes it first and says so.
- **A patch belongs to the process it was made for.** The engine links each patch against the
  running process: its calls into the rest of the program are that process's addresses. A
  process started again by hand — `R` as a quick restart — came back with the code that was
  built; a patch kept and applied again in it crashed (`SEGV_ACCERR`), and a patch the engine
  built for it once it had connected crashed too (a null pointer, inside the patched code). Two
  patches in a row in one process work. So `R` on Android does what it does on the desktop:
  the engine is started again, builds what is saved, installs it and starts it.

## Verified

On a Huawei STK-L21 (Android 10, Mali-G51), the counter application, `frus run --hot -d
<serial>`:

- the first build took about fifteen minutes from nothing; `libmain.so` is 376 MB in debug;
- the app bar's title changed while the count went on, patched in 8–26 s; a second change in
  the same process was patched in too;
- `R` built, installed and started again in about 100 s, with the patched title and the count
  from the start, and the next change was patched into that process;
- the application's log came through `frus run` as without `--hot`.

`cargo test -p frus-cli`: 98 tests, on Windows and on Linux (WSL). clippy for the shell with `hot-reload` on Windows, Android
(`aarch64-linux-android`) and the web. Not run: an emulator, Linux and macOS hosts.
