# Changelog

## Unreleased

- **Hot reload by default, with every frus**: a debug run passes `--hot`, so that a save is
  patched into the running application with frus 0.5.0 too, whose own default is off.
  `"hot": false` restarts to reload instead (`--no-hot`, left out for a frus that does not know
  it); `"hot": "frus"` passes nothing and lets frus decide; `true` patches a release build too.
- **Every run ends with a line in the Debug Console**: its exit code and, when it failed, the
  last error it printed. A run used to end with nothing said.
- **The device F5 runs on, until one is picked**: a phone plugged in, else a running emulator,
  else this computer, and the status bar shows that one. It used to show the first device by
  name (a simulated "Android phone") while F5 let frus choose this computer; simulated devices
  are now listed last and used only when picked.

## 0.2.0

- **Simulated phones and tablets** (frus 0.5): the picker has a **Simulated** section — an
  iPhone, an iPad, an Android phone, and the devices a project describes in its `Cargo.toml` —
  and F5 runs the application in a window shaped like the one chosen.
- **frus: Create New Project**: a folder, a name and an organisation, then `frus create`, and
  the new project opened. Also a **Create frus Project** button in an empty window.
- **frus: Run Frus Update**: `cargo install frus-cli` in a terminal, the newest `frus`.
- **frus: Run Doctor** is now **frus: Run Frus Doctor**.

## 0.1.1

Not published on its own: it reached the Marketplace with 0.2.0.

- A clearer Marketplace page: how to start in three steps, what the extension does, its keys
  and settings, and links to frus.

## 0.1.0

The first release on the Marketplace.

- A device picker in the status bar: this computer, phones, emulators and browsers, followed as
  they come and go; emulators started and created from it.
- F5 runs the application on the selected device, with its log in the Debug Console.
- Reload on save, with each component's state kept; restart, stop, and the keys of `frus run`
  as commands (debug paint, inspector, platform, brightness, performance overlay, the trees,
  screenshots).
- **frus: Run Doctor**.
- Installs `frus` (`cargo install frus-cli`) when it is missing.
