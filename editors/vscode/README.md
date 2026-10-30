# frus for VS Code

[![Marketplace](https://img.shields.io/visual-studio-marketplace/v/kalybospro.frus?label=Marketplace)](https://marketplace.visualstudio.com/items?itemName=kalybospro.frus)
[![Installs](https://img.shields.io/visual-studio-marketplace/i/kalybospro.frus)](https://marketplace.visualstudio.com/items?itemName=kalybospro.frus)
[![frus on crates.io](https://img.shields.io/crates/v/frus?label=frus)](https://crates.io/crates/frus)

**Build, run and reload [frus](https://github.com/KalybosPro/frus) applications from VS Code — on
your computer, your phone or your browser — without leaving the editor.**

frus is a cross-platform UI framework written in Rust: one codebase, drawn by its own GPU
renderer, running natively on **Windows, macOS, Linux, Android and the web** (iOS is on its
way). This extension is its development loop: pick a device, press F5, edit, save, and watch
the change appear with your application's state intact.

## Get started in three steps

1. **Install Rust** from [rustup.rs](https://rustup.rs). That is the only thing to install by
   hand: the extension fetches the `frus` tool, and `frus` fetches what each platform needs
   (Rust targets, the Android SDK, wasm-bindgen).
2. **Create an application**: **frus: Create New Project** in the command palette
   (Ctrl+Shift+P), or the **Create frus Project** button of an empty window. Pick a folder, a
   name and your organisation, and the new project opens. From a terminal it is
   `frus create my_app`.
3. **Press F5.** The application starts on the device shown in the status bar, its log in the
   Debug Console. Change some code, press **Ctrl+S**, and it comes back on the same page.

Not sure your machine is ready? **frus: Run Frus Doctor** lists what it has for each platform
and what it lacks. **frus: Run Frus Update** brings the `frus` tool to its latest version.

## What it does

### A new project in a few clicks

**frus: Create New Project** asks where, the name (a Rust package name: lowercase letters,
digits, `_` and `-`) and the organisation the Android package starts with (`com.example` unless
you say otherwise; the last one is remembered), runs `frus create`, and opens the new project —
in a new window when this one already has a folder. The project builds for the desktop,
Android and the web as it is, with its tests in `tests/`.

### Every device, one picker

The status bar shows where your application will run: this computer, the phones plugged in, the
running emulators, the browsers, and **simulated phones and tablets** — an iPhone, an iPad, an
Android phone, or a device your project describes — shown in a window of this computer, with
their screen, safe areas and keyboard, and no phone needed. The list follows phones as they come
and go. From the same menu you can **start an emulator** (it is selected once it is up) or **create an Android emulator**,
which installs whatever is missing. A device your application does not build for says why.

### F5 runs it, and shows its log

**F5** builds the application for the selected device, starts it there and streams **its log
into the Debug Console**: the desktop program's output, the phone's logcat, the browser's
console. Errors are shown as errors. No `launch.json` is needed.

### Reload on save, state kept

Saving a Rust file rebuilds the application and brings it back **on the same page, with the
state of every component that keeps one**. A restart, when you want every state afresh, is one
key away. With `"hot": true` in a launch configuration, desktop runs use **true hot reload**:
each save is patched into the running program without restarting it.

### The framework's tools, in the palette

Inspect a running application without leaving the editor: **Toggle Debug Paint**, **Toggle
Inspector**, **Toggle Performance Overlay**, **Toggle Platform** (see it as on another
platform), **Toggle Brightness** (light and dark), **Print Widget Tree**, **Print Focus Tree**,
**Print Semantics Tree** (what a screen reader sees) and **Save Screenshot**. What they print
appears in the Debug Console.

## Keys while it runs

| Key | Does |
|---|---|
| **F5** | run on the selected device |
| **Ctrl+S** | reload, after saving a Rust file |
| **Ctrl+F5**, ⚡ in the debug toolbar | reload |
| **Ctrl+Shift+F5**, the toolbar's restart | restart, every state afresh |
| **Shift+F5** | stop |

**frus: Detach** leaves the application running and ends the session.

## A launch configuration, when you want one

To pin a device or pass options, add one to `.vscode/launch.json`:

```json
{
  "type": "frus",
  "request": "launch",
  "name": "frus on the phone",
  "cwd": "${workspaceFolder}",
  "deviceId": "android",
  "hot": false,
  "release": false,
  "args": []
}
```

`deviceId` pins the device, `hot` turns on true hot reload (desktop), `release` builds
optimised, and `args` are passed on to `frus run`.

## Settings

| Setting | |
|---|---|
| `frus.path` | the `frus` program to use; empty for the one on the PATH |
| `frus.reloadOnSave` | when saving a Rust file reloads: `manual` (a save you make, the default), `manualIfDirty` (only with changes), `all` (auto save too), `never` |

## Keeping frus up to date

**frus: Run Frus Update** runs `cargo install frus-cli` in a terminal: the newest `frus` from
crates.io, or nothing to do when you have it. The extension picks the new one up by itself.
**frus: Run Frus Doctor** runs `frus doctor -v`: what this machine has for each platform, and
what it lacks.

## How it works

The extension drives the `frus` command-line tool (`frus daemon` for the devices, `frus run
--machine` for a session). When `frus` is missing it runs `cargo install frus-cli`. It runs its
own copy of `frus`, so updating the tool while VS Code is open just works: the device list
restarts from the new one and the next F5 uses it.

## Links

- **frus**: [repository](https://github.com/KalybosPro/frus) ·
  [getting started](https://github.com/KalybosPro/frus/blob/master/docs/getting-started.md) ·
  [crates.io](https://crates.io/crates/frus)
- **Found a bug or missing something?** [Open an issue](https://github.com/KalybosPro/frus/issues).
- **Like it?** A rating on the Marketplace helps other people find it.

Licensed under MIT or Apache-2.0, at your option.

## Developing the extension

```sh
npm install
npm test                 # the debug adapter, with a stand-in for frus
npm run package          # frus-<version>.vsix
```

`FRUS_E2E_FRUS=/path/to/frus FRUS_E2E_APP=/path/to/an/app npm test` runs the same tests with
the real `frus`, and `npm run test:integration` runs them inside a VS Code of its own
(downloaded into `FRUS_VSCODE_CACHE`, or `FRUS_VSCODE=<Code.exe>`), never the one you work in.
