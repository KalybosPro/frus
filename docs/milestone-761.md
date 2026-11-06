# Milestone 761 — A plugin for Android Studio

## Objective

frus had a VS Code extension (J666) and nothing for Android Studio, where most people who
build for Android already work. The objective: the same development loop inside Android Studio
— pick a device, Run, edit, save, see the change with the state kept — and, since Android
Studio is an IntelliJ platform IDE, inside IntelliJ IDEA, RustRover and CLion too.

## What it does

`editors/intellij`, a Kotlin plugin built with the IntelliJ Platform Gradle Plugin:

- **A device selector** in the toolbar (the new UI's main toolbar and the classic run group),
  fed by `frus daemon`: devices come and go as the daemon reports them; the menu starts an
  emulator (and selects it once it is up) or creates one; a platform the application does not
  build for says why (`daemon.getSupportedPlatforms`). It shows only in projects that hold a
  frus application, a package whose `Cargo.toml` depends on `frus`.
- **A frus run configuration**, made for each application when the project opens. Run starts
  `frus run --machine` and translates its JSON lines for the Run console: the application's
  log as standard output, its errors as standard error, the tool's progress and the build as
  system text, nothing raw. Stop sends `app.stop` and waits for the application to end before
  it would kill the process.
- **Reload on save**: Ctrl+S on a Rust file sends `app.restart`. When is a setting — by hand
  only (the default, so auto save does not reload at every pause), by hand when a Rust file
  changed, every save, never. Reload and Restart on Ctrl+\ and Ctrl+Shift+\.
- **Tools › frus**: the keys of `frus run` as actions (`app.key`), the emulators, frus Doctor,
  Update frus. **New frus Project** in File › New and on the Welcome screen runs `frus create`.

## Decisions

- **The platform alone.** The plugin depends on `com.intellij.modules.platform` and nothing
  else — not the Android plugin, not the Rust plugin — so the one build installs in every
  IntelliJ IDE from 2025.1 (build 251) on, with no upper bound. Running through `frus` rather
  than through Gradle or Cargo run configurations keeps it so.
- **The daemon and `--machine`, as in VS Code.** Both editors speak the same protocol, so
  device discovery, emulators, reload and the keys behave the same in both, and frus remains
  the one place that knows how to build for each platform.
- **A copy of `frus`.** As in VS Code, the plugin runs a copy kept in the IDE's system folder,
  replaced when the installed program changes, so that `cargo install frus-cli` can update
  the installed one while the IDE has it open (Windows locks a running executable).
- **No JSON library.** A small parser and writer for the protocol's lines, tested both ways,
  instead of depending on whichever JSON library a given IDE ships.

## Tests

`./gradlew test`: the protocol (JSON both ways, devices, emulators, unsupported platforms,
reload modes, run and create arguments, finding applications, project names, the program's
copy); the console's translation; and, inside a headless IDE, a stand-in for
`frus run --machine` (a small Java program) driven through the real process handler — output
types, reload, restart, a key, and Stop ending on `app.stop` without a kill — plus the run
configuration made once per application and kept across a save and a load.

`./gradlew verifyPlugin`: compatible with every Android Studio release the verifier recommends,
2025.1 (AI-251) through 2026.2 (AI-262). 2026.2 schedules every `SimpleListCellRenderer.create`
overload for removal; the plugin's lists render through a `ColoredListCellRenderer` of its own
(`textRenderer`), which no release deprecates.

The headless test first failed now and then: standard error is read on a thread of its own,
so the build's line could arrive after the application had started. It waits for each line
it checks. It also gave the fake four seconds to end after Stop, too few for a JVM on a busy
machine; it now tells "ended on its own" from "killed" by the exit code instead of the clock.

## Not done here

- Not tried by hand in a running Android Studio against a real phone in this milestone; the
  first `./gradlew runIde` session is owed.
- Not on the Marketplace yet: `.github/workflows/intellij.yml` publishes on an `intellij-v*`
  tag, and the Marketplace takes a plugin's first version only by hand.
