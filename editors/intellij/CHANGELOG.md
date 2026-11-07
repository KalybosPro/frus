# Changelog

## Unreleased

- **Hot reload is on by default**, passed as `--hot`, so that it is on with frus 0.5.0 too,
  whose own default is off. The run configuration's choices are On (`--hot`, the default), Off
  (`--no-hot`, left out for a frus that does not know it) and Let frus decide; the old "frus
  decides: on for a debug build", untrue for frus 0.5.0, is gone, and a configuration saved
  with it is on. On leaves a release build without hot reload.
- The hint under the Hot reload box follows the choice.
- **Every run ends with a line in the console**: its exit code, and when it failed, the last
  error it printed. A run used to stop with nothing said.
- **The device runs go to, until one is chosen**: a phone plugged in, else a running emulator,
  else this computer. A simulated device ("Android phone") was picked because its name sorts
  first; simulated devices are now listed last and used only when chosen.

## 0.1.0

The first release, for Android Studio, IntelliJ IDEA, RustRover and the other JetBrains IDEs
from 2025.1 on.

- A device selector in the toolbar, fed by `frus daemon`: this computer, phones, running
  emulators, browsers and simulated devices; start or create an emulator from it.
- A frus run configuration, made when a project with a frus application opens: Run builds and
  starts the application on the chosen device, its log in the Run console.
- Reload on save (by hand only, by hand with changes, every save, or never), Reload
  (Ctrl+\\) and Restart (Ctrl+Shift+\\).
- Tools › frus: debug paint, inspector, performance overlay, platform, brightness, the widget,
  focus and semantics trees, screenshot, detach, emulators, frus Doctor, Update frus.
- New frus Project, in File › New and on the Welcome screen.
- `frus` found on the PATH or in `~/.cargo/bin`, installed when missing, and run from a copy so
  that an update can replace it.
