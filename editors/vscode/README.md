# frus for VS Code

Run [frus](https://github.com/KalybosPro/frus) applications from VS Code, on any device.

- **A device picker** in the status bar: this computer, the connected phones and emulators, the
  browsers. It follows phones as they are plugged in and out.
- **F5** builds the application for the selected device, starts it there and shows **its log in
  the Debug Console**: the desktop program's output, the phone's logcat, the browser's console.
  Errors are shown as errors.
- **Reload on save**: saving a Rust file builds the application again and brings it back on the
  same page, with the state of every component that keeps one (`State::save`). The ⚡ button in
  the debug toolbar, or Ctrl+F5, does the same; the toolbar's restart button starts it afresh.

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

## Settings

| setting | |
|---|---|
| `frus.path` | the `frus` program to use; empty for the one on the PATH |
| `frus.reloadOnSave` | reload the running application when a Rust file is saved (default on) |

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
