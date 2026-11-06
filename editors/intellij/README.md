# frus for Android Studio

**Build, run and reload [frus](https://github.com/frus-rs/frus) applications from Android
Studio — on your computer, your phone or your browser — without leaving the IDE.** The same
plugin works in IntelliJ IDEA, RustRover, CLion and the other JetBrains IDEs, from 2025.1 on.

frus is a cross-platform UI framework written in Rust: one codebase, drawn by its own GPU
renderer, running natively on **Windows, macOS, Linux, Android and the web**. This plugin is
its development loop: pick a device, press Run, edit, save, and watch the change appear with
your application's state intact.

## Get started

1. **Install Rust** from [rustup.rs](https://rustup.rs). That is the only thing to install by
   hand: the plugin fetches the `frus` tool, and `frus` fetches what each platform needs (Rust
   targets, the Android SDK, wasm-bindgen).
2. **Install the plugin**: Settings › Plugins › Marketplace, search "frus". Or, from a build
   of this folder, Settings › Plugins › ⚙ › Install Plugin from Disk, and pick
   `build/distributions/frus-intellij-<version>.zip`.
3. **Create an application**: File › New › **New frus Project…**, or the same entry on the
   Welcome screen. Pick a location, a name and your organisation; the new project opens. From
   a terminal it is `frus create my_app`.
4. **Press Run** (Shift+F10). The application starts on the device shown in the toolbar, its
   log in the Run console. Change some code, press **Ctrl+S**, and it comes back with its
   state.

Not sure your machine is ready? **Tools › frus › frus Doctor** lists what it has for each
platform and what it lacks. **Tools › frus › Update frus** brings the `frus` tool to its
latest version.

## What it does

### Every device, one selector

The toolbar shows where your application will run: this computer, the phones plugged in, the
running emulators, the browsers, and **simulated phones and tablets** — shown in a window of
this computer, with their screen, safe areas and keyboard, and no phone needed. The list
follows phones as they come and go. From the same menu you can **start an emulator** (it is
chosen once it is up) or **create an Android emulator**, which installs whatever is missing. A
device your application does not build for says why.

The selector appears in projects that hold a frus application (a package whose `Cargo.toml`
depends on `frus`); elsewhere the IDE's own selector stays as it is.

### Run, and its log

Opening a project with a frus application makes a **frus run configuration** for it (one per
application when there are several). **Run** builds the application for the chosen device,
starts it there and streams **its log into the Run console**: the desktop program's output, the
phone's logcat, the browser's console. The application's errors are shown as errors, the
build's progress apart from them. **Stop** stops the application.

A configuration can name its own device (an id from `frus devices`, or `android`, `web`,
`desktop`), ask for a release build, turn hot reload on or off, and pass arguments to a
desktop application.

### Reload on save, state kept

Saving a Rust file patches the change into the **running application, without restarting it**:
every state stays where it was (desktop, web, Android; the first run installs the patch engine).
**Restart** brings every state back afresh. With hot reload off in the run configuration, a
save rebuilds the application instead and brings it back on the same page, with the state of
every component that keeps one.

When a save reloads is a setting (Settings › Tools › frus): by hand only (Ctrl+S, the
default, so that auto save does not reload at every pause), by hand and only when a Rust file
changed, on every save, or never.

### The framework's tools

Under **Tools › frus**: Toggle Debug Paint, Toggle Inspector, Toggle Performance Overlay,
Toggle Platform (see it as on another platform), Toggle Brightness (light and dark), Print
Widget Tree, Print Focus Tree, Print Semantics Tree (what a screen reader sees), Save
Screenshot, and Detach (leave the application running). What they print appears in the Run
console.

## Keys

| Key | Does |
|---|---|
| **Shift+F10** | run on the chosen device |
| **Ctrl+S** | reload, after saving a Rust file |
| **Ctrl+\\**, ⚡ in the Run console | reload |
| **Ctrl+Shift+\\**, the Run console's restart | restart, every state afresh |
| **Ctrl+F2** | stop |

On macOS, Cmd takes the place of Ctrl.

## Settings

Settings › Tools › frus:

- **frus program**: the `frus` to use. Empty (the default): the one on the PATH or in
  `~/.cargo/bin`, installed with `cargo install frus-cli` when there is none.
- **Reload on save**: as above.

The plugin runs a copy of `frus`, kept in the IDE's system folder, so that `cargo install
frus-cli` can replace the installed one while the IDE is open; the copy follows a new install
on its own.

## Building the plugin

It needs a JDK 21 (Android Studio's own, in its `jbr` folder, will do).

```sh
./gradlew test           # the tests
./gradlew buildPlugin    # build/distributions/frus-intellij-<version>.zip
./gradlew runIde         # an Android Studio with the plugin, in a sandbox of its own
./gradlew verifyPlugin   # checked against the IDEs it claims to support
```

`-PlocalIde="C:/Program Files/Android/Android Studio"` builds and runs against an installed
IDE instead of downloading one.

## Licence

MIT or Apache-2.0, at your choice.
