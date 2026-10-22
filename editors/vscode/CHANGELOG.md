# Changelog

## 0.2.0

- **frus: Create New Project**: a folder, a name and an organisation, then `frus create`, and
  the new project opened. Also a **Create frus Project** button in an empty window.
- **frus: Run Frus Update**: says when `frus` is the newest already; otherwise
  `cargo install frus-cli --force` in a terminal, which also replaces a `frus` installed from a
  checkout (cargo refuses that without `--force`).
- **frus: Run Doctor** is now **frus: Run Frus Doctor**.

## 0.1.1

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
