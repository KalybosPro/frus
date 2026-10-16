# Milestone 661 — `frus devices`, `frus run`, `frus build`

## Objective

Running a frus application meant a different command on each platform: `cargo run` on the
desktop, `cargo apk run --lib` on a phone, and three commands for the web (a wasm build,
`wasm-bindgen`, a static server), plus knowing that the browser's console is where the web's log
goes and that logcat needs a tag filter. Nothing said what the application could run on, and a
machine missing one tool stopped with an error naming it.

The reference has one tool for all of it: a device list, `run -d <device>` that builds, installs,
starts and stays attached with the application's log in the terminal, and `build <target>` for
what is shipped. This milestone gives frus the same three commands.

## What was done

- **`frus devices`** lists, in aligned columns of name, id and detail:
  - this computer;
  - the Android devices `adb devices -l` reports, with their model, ABI and Android version, and
    those that cannot be used yet with the reason (`unauthorized: allow USB debugging`);
  - the emulators that can be started (`emulator -list-avds`), minus those already running;
  - the iOS simulators, on a Mac (`xcrun simctl list devices available`);
  - the browsers installed (Edge, Chrome), and a bare web server.
- **`frus run [-d <device>] [--release] [--port <n>] [-- <args>]`**:
  - `-d` takes an id, a name (or part of one), or `android`, `web`, `desktop`, `ios`. With no
    `-d` it runs on this computer. A vague name lists the candidates.
  - **Desktop**: `cargo build --message-format=json-render-diagnostics`, the executable read
    from the artifact message, started with the terminal as its output.
  - **Android**: cargo-apk builds the APK (only one written by this build is installed, never a
    stale one), `adb install -r`, the previous instance stopped, logcat read from the device's
    current time and filtered on the application's tag and Rust's panic tags, then the activity
    started. The run ends when the application's process is gone. An emulator named by `-d` is
    started first and waited for until Android has booted.
  - **Web**: the library built for wasm, wasm-bindgen's glue written into `web/pkg/`, and `web/`
    served on `localhost`, which browsers count as a secure context, so WebGPU is available. The
    served `index.html` gets a script at the top of its `<head>` that copies every console message
    and uncaught error to `/__frus/log`, and the server prints them. The browser is a fresh
    instance on a profile of its own, so closing it ends the run without touching the
    developer's windows.
  - **iOS**: a simulator named by `-d` gets a plain answer: frus has no iOS shell yet (#19).
- **`frus build apk|web|desktop [--debug]`**: release builds, with the result's path and size.
  The web build uses the `web-release` profile when the manifest declares it.
- **What is missing is installed** rather than reported:
  - Rust targets, with `rustup target add`;
  - cargo-apk, with `cargo install`;
  - wasm-bindgen, at exactly the version the application's `Cargo.lock` holds. A different
    version already on the `PATH` is not replaced, since another project may need it. The pinned
    one is installed in `~/.frus/tools/<crate>-<version>` and used from there;
  - the Android SDK. The one `ANDROID_HOME` names, or Android Studio's, is completed with the
    platform, build-tools and NDK it lacks, through `sdkmanager` with its licences accepted. With
    no SDK at all, one is installed in `~/.frus/android-sdk`, with a JDK in `~/.frus/jdk` if
    there is no Java. Downloads use `curl` and the system's archive tool, so the tool still has
    no dependencies.
- **Documentation**: the CLI's README, the getting-started guide's "Running on a device", and
  the template's README, whose table now gives `frus run` and `frus build` for each platform.

## Verified

On this Windows machine, with a Huawei STK-L21 (Android 10) connected:

- `frus devices` listed the desktop, the phone (`android-arm64 (Android 10)`), Edge and the web
  server.
- `frus run` on `frus-examples/hello_world`: the window opened, and the application's log
  appeared in the terminal.
- `frus run -d edge`: wasm-bindgen 0.2.127, the lockfile's, was installed in `~/.frus/tools`
  while the 0.2.129 on the `PATH` was left alone. Edge opened the page, and the application's
  console (`[INFO frus_shell] target platform: Windows, from the browser`, the WebGPU adapter)
  reached the terminal.
- `frus build apk --debug` built and signed the APK natively on Windows with the machine's SDK
  and NDK; it had only ever been built in WSL before.
- `frus run -d android`: installed, started, the phone's logcat in the terminal (the input
  bridge, the Mali-G51 adapter); stopping the application ended the run.

## Found on the way

- `adb shell` hands its arguments to the device's shell joined by spaces, so a value holding a
  space (the date given to `logcat -T`) must be quoted for that shell; `adb logcat` does not, and
  a quoted value there is taken literally.
- A version-pinned tool must not be reinstalled over the developer's own: the first version of
  this milestone did `cargo install --force` and would have downgraded a wasm-bindgen other
  projects needed.
- cargo-apk warns that `ANDROID_SDK_ROOT` is deprecated; only `ANDROID_HOME` and
  `ANDROID_NDK_ROOT` are set.

## Left

- **Hot restart, hot reload, a daemon for editors, the VS Code extension**: milestones 662 to 665.
- **A debug APK is about 270 MB**, nearly all of it debug symbols, so the install over USB takes
  most of a run's time. Stripping the symbols for `frus run` would make it a few MB.
- **The from-nothing SDK install** (no SDK and no Java on the machine) was written but not run
  end to end here: this machine has both.
- **macOS and Linux** paths (browsers, simulators) are written but were not run.
- **iOS** waits for its shell (#19).
