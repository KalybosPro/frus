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

The directory is made if it is not there, and `frus create` never writes over a file. In a
directory with files already, only the template's missing files are written: `frus create .`
in an application brings back what was deleted (`web/`, the icons, the desktop binary, the
workflows), keeps its `Cargo.toml` and its code, adds no sample code beside them, and takes
the application's name from its manifest. The name is checked before anything is written. It must work as a package, a crate, a program, a `.wasm`
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
| `h` | the list of keys |
| `d` | **detach**: `frus run` ends and the application goes on running (desktop, Android) |
| `c` | clear the terminal |
| `q` | quit: the application is stopped |
| `s` | **screenshot**, saved beside the application as `frus_01.png`, `frus_02.png`… (desktop: the application's frame; Android: the device's screen) |
| `w` | print the **widget tree**, every widget with its box |
| `f` | print the **focus tree**: the focusable widgets in tab order, the focused one marked |
| `S` | print the **semantics tree**: what a screen reader is told, in reading order |
| `i` | the **inspector**: outlines, and a card for the widget under the pointer (F12 in the window) |
| `p` | **debug paint**: outlines around every widget |
| `o` | **switch the platform** the widgets behave as: Android, iOS, Windows, macOS, Linux, Fuchsia (debug builds) |
| `b` | **switch the brightness**: light or dark, for an application with a theme for each |
| `P` | the **performance panel**: what the last frame cost, building, laying out and painting, rendering, on the GPU; and the frame statistics in the log |

The keys that look into the application reach it through the channel it reports on, and it
carries them out on its next frame: a socket on a desktop, a file in its own storage on Android
(written with `run-as`, so a debug build only), and the page's own script on the web. A key
pressed while the application is still starting waits for it.

A build that fails leaves the running application as it was. The restart is the same on every
platform as far as the developer sees, and done three ways underneath:

- **desktop**: the application runs from a copy of the program, so the new build can be written
  while it runs; it reports where it is and where its window sits to `frus run` over a local
  socket, and the next one is started with both;
- **Android**: the same report reaches `frus run` through the application's log; the new APK is
  installed — unless the build wrote the same package again, which is only started again — and
  started with the location as the link it opens;
- **web**: the page asks the server which build it serves, and reloads itself when there is a
  new one — its address, which is the application's location, survives the reload.

iOS simulators are listed, but frus cannot run on iOS yet: its shell does not exist (issue #19).

## For editors: `frus daemon` and `frus run --machine`

An editor drives the same commands through **JSON lines**: one object a line, commands on
standard input, answers and events on standard output. Nothing else is written to standard
output; the tools `frus` runs (cargo, cargo-apk, wasm-bindgen) write to standard error.

`frus daemon` runs as long as the editor does, and knows the devices:

```text
→ {"id":1,"method":"device.getDevices"}
← {"id":1,"result":[{"id":"windows","name":"Windows","platform":"windows","category":"desktop","detail":"windows-x64","emulator":false,"available":true,"unavailableReason":null}, …]}
→ {"id":2,"method":"device.enable"}
← {"id":2,"result":true}
← {"event":"device.added","params":{"id":"XMJNW19B23011768","name":"STK L21","platform":"android", …}}
```

| method | answer |
|---|---|
| `daemon.version` | the tool's version |
| `daemon.shutdown` | `true`, and the daemon ends |
| `device.getDevices` | the devices, as `frus devices` lists them |
| `device.enable` / `device.disable` | `true`; `device.added` and `device.removed` events while enabled |
| `device.takeScreenshot` | the screen of an Android device (`deviceId`), a PNG in base64 |
| `daemon.getSupportedPlatforms` | for the application in `projectRoot`: `platforms`, and in `platformTypes` each platform with `isSupported` and, if not, `reasons` (`reasonText`, `fixText`) |
| `emulator.getEmulators` | the emulators that can be started: `id`, `name`, `category`, `platformType` |
| `emulator.launch` | starts `emulatorId` (`coldBoot`: without its saved state); it arrives as `device.added` once up |
| `emulator.create` | makes an Android emulator named `name` (or `frus_emulator`), installing a system image if none is there: `success`, `emulatorName`, `error` |

On the way, the daemon says what it does as `daemon.logMessage` events (`level`, `message`), and
what the developer should see now as `daemon.showMessage` (`level`, `title`, `message`).

`frus run --machine -d <device>` is one run, with every event carrying its `appId`:

| event | when |
|---|---|
| `app.start` | the run begins: `deviceId`, `deviceName`, `directory` |
| `app.progress` | what the tool says: `message`, and `error: true` for a failure that did not end the run |
| `app.started` | the application is up |
| `app.log` | a line of the application's own log: `log`, `error` |
| `app.restarted` | a reload or restart is done: `fullRestart`, `seconds` |
| `app.stop` | the run ended: `error` when it failed |

and takes `{"method":"app.restart","params":{"fullRestart":false}}` (`r`; `true` is `R`),
`{"method":"app.stop"}`, `{"method":"app.detach"}`, and any other key by its letter:
`{"method":"app.key","params":{"key":"p"}}`.

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
