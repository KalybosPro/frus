# frus for VS Code

Run [frus](https://github.com/KalybosPro/frus) applications from VS Code, on any device.

- **A device picker** in the status bar: this computer, the connected phones and emulators, the
  browsers. It follows phones as they are plugged in and out. Below them, the emulators that can
  be started (**Start …**: once up, the emulator is selected) and **Create an Android emulator**,
  which installs what it needs. A device the application does not build for says why. The
  commands **frus: Launch Emulator** and **frus: Create Emulator** do the same from the palette,
  and what frus does on the way is in the **frus** output channel.
- **F5** builds the application for the selected device, starts it there and shows **its log in
  the Debug Console**: the desktop program's output, the phone's logcat, the browser's console.
  Errors are shown as errors.
- **Reload on save**: saving a Rust file (Ctrl+S) builds the application again and brings it
  back on the same page, with the state of every component that keeps one (`State::save`).
  Auto save does not reload, unless `frus.reloadOnSave` says `all`.
- **While it runs**:

  | | |
  |---|---|
  | Ctrl+S | reload (a Rust file) |
  | Ctrl+F5, ⚡ in the debug toolbar | reload |
  | Ctrl+Shift+F5, the toolbar's restart | restart, every state afresh |
  | Shift+F5 | stop |

  and in the command palette, **frus: Toggle Debug Paint**, **Toggle Inspector**, **Toggle
  Platform**, **Toggle Brightness**, **Toggle Performance Overlay**, **Print Widget Tree**,
  **Print Focus Tree**, **Print Semantics Tree**, **Save Screenshot** and **Detach** — the keys
  of `frus run` in a terminal. What they print appears in the Debug Console.

No `launch.json` is needed: F5 in a frus application runs it on the selected device. To pin a
device or pass arguments, add a configuration:

```json
{
  "type": "frus",
  "request": "launch",
  "name": "frus on the phone",
  "cwd": "${workspaceFolder}",
  "deviceId": "android"
}
```

## Requirements

The extension drives the `frus` command-line tool (`frus daemon` and `frus run --machine`). If it is
not installed, the extension installs it with `cargo install frus-cli`; `frus` installs what it
needs in turn (Rust targets, the Android SDK, wasm-bindgen).

The extension runs a copy of `frus`, kept in its own storage, never the installed program
itself: `cargo install frus-cli` can update it while VS Code is open (Windows refuses to replace
a program that is running). When the installed `frus` changes, the device list restarts from
the new one, and the next F5 uses it.

## Settings

| setting | |
|---|---|
| `frus.path` | the `frus` program to use; empty for the one on the PATH |
| `frus.reloadOnSave` | when saving a Rust file reloads: `manual` (by hand, the default), `manualIfDirty` (by hand, with changes), `all` (auto save too), `never` |

## Developing it

```sh
npm install
npm test        # the debug adapter, with a stand-in for frus
FRUS_E2E_FRUS=/path/to/frus FRUS_E2E_APP=/path/to/an/app npm test   # and with the real one
# inside a VS Code of its own (downloaded into FRUS_VSCODE_CACHE, or FRUS_VSCODE=<Code.exe>),
# never the one you work in: activation, F5, the log, a reload, the end of the session
FRUS_E2E_FRUS=/path/to/frus FRUS_E2E_APP=/path/to/an/app npm run test:integration
npm run package # frus-<version>.vsix
```
