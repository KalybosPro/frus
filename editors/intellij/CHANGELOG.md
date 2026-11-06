# Changelog

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
