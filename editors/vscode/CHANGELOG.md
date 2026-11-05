# Changelog

## Unreleased

- **Hot reload by default**: with a frus that patches by default (milestone 754), a save is patched
  into the running application. `"hot": false` in a launch configuration restarts to reload
  instead; `"hot"` left out lets frus decide.

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
