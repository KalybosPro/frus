# frus-cli

The command-line tool of [frus](https://github.com/KalybosPro/frus), a cross-platform UI
framework in Rust. It installs a program called `frus`, which starts new applications, runs
them on any device, and builds what you ship.

```sh
cargo install frus-cli
frus create my_app
cd my_app
frus run                  # on this computer
frus run -d android       # on the connected phone
frus run -d web           # in a browser
```

| command | what it does |
|---|---|
| `frus create <dir>` | writes a new application |
| `frus devices` | lists what the application can run on now |
| `frus run [-d <device>]` | builds it, starts it there, and shows its log in the terminal |
| `frus build apk\|web\|desktop` | builds what you ship, in release |

`frus create` writes an application that runs on Windows, macOS, Linux, Android and the web
from the same code: a counter built from a component, with its tests, the Android launcher
icons and launch theme, the web page, a CI workflow and a release workflow that publishes every
platform's build when you push a tag. Its `README.md` says how to run and ship it on each
platform.

```text
frus create <directory> [options]

      --name <name>        the package's name (default: the directory's name)
      --org <org>          the reverse domain the Android package starts with
                           (default: com.example; the package is <org>.<name>)
      --frus-path <dir>    build against a checkout of frus, not the published crates
      --no-git             do not make the directory a git repository
```

The directory is made, or must be empty: `frus create` never writes over a file. The name is
checked before anything is written. It must work as a package, a crate, a program, a `.wasm`
file and part of an Android package, so it is lowercase letters, digits, `_` and `-`, starting
with a letter, and neither a Rust keyword nor the name of a crate the application builds with.

The template is built into the program, so `frus create` needs no network. Its files are in
[`template/`](template), which is also a `cargo generate` template.

## Running: `frus run` and `frus devices`

```text
frus devices
frus run [-d <device>] [--release] [--port <port>] [-- <args for the application>]
```

`frus devices` lists this computer, the Android devices `adb` sees (USB debugging on) and the
emulators that can be started, the iOS simulators on a Mac, the browsers installed here, and a
bare web server. `-d` takes a device's id or name, or `android`, `web`, `desktop`; without it the
application runs on this computer.

Whatever the device, **the application's log follows in the terminal**: the desktop program's
output, the phone's logcat (the application's own tag, and Rust panics), and the browser's
console — the page is served with a small script that copies each console message back to the
terminal. The run lasts as long as the application: closing the window, the app or the browser
ends it.

While it runs, keys drive it:

| key | |
|---|---|
| `r` | **reload**: build again, stop the application and start the new build where the old one was — the same page, on a desktop the window in the same place and size — **with the state of every component that keeps one** (`State::save`) |
| `R` | **restart**: the same, but every state starts afresh |
| `q` | quit: the application is stopped |
| `h` | the list of keys |

A build that fails leaves the running application as it was. The restart is the same on every
platform as far as the developer sees, and done three ways underneath:

- **desktop**: the application runs from a copy of the program, so the new build can be written
  while it runs; it reports where it is and where its window sits to `frus run` over a local
  socket, and the next one is started with both;
- **Android**: the same report reaches `frus run` through `adb reverse`; the new APK is
  installed and started with the location as the link it opens;
- **web**: the page asks the server which build it serves, and reloads itself when there is a
  new one — its address, which is the application's location, survives the reload.

iOS simulators are listed, but frus cannot run on iOS yet: its shell does not exist (issue #19).

## Building: `frus build`

```text
frus build apk        # the Android package
frus build web        # web/, with the application compiled into web/pkg/
frus build desktop    # the program for this computer
```

Release by default; `--debug` for a debug build.

## What is missing is installed

A command does not stop at "install X first": what it needs and does not find, it installs, says
so, and carries on.

- **Rust targets** (`wasm32-unknown-unknown`, the Android ones the manifest lists): `rustup target add`.
- **wasm-bindgen**, at exactly the version in the application's `Cargo.lock`. A different version
  on the `PATH` is left alone — other projects may need it — and the pinned one goes in
  `~/.frus/tools/`.
- **cargo-apk**: `cargo install cargo-apk`.
- **The Android SDK and NDK**: the SDK `ANDROID_HOME` names, or Android Studio's, is completed with
  the packages it lacks (`sdkmanager`, licences accepted); with no SDK at all, one is installed in
  `~/.frus/android-sdk`, with a JDK in `~/.frus/jdk` if there is no Java. Downloads use `curl` and
  the system's archive tool.

`FRUS_HOME` moves `~/.frus`; `FRUS_ANDROID_SDK` names the SDK to use (and to install into).

Everything the tool does can still be done by hand with cargo, cargo-apk, adb and wasm-bindgen:
it adds no build system of its own.

## License

MIT or Apache-2.0, at your option.
