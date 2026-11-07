# Milestone 771 — Editors: hot reload on by default, every run's end said, a sensible default device

## Objective

A developer tried the Android Studio plugin (milestone 761) with frus 0.5.0, the version on
crates.io, and reported four problems:

1. The run configuration's hot reload option read "frus decides: on for a debug build". That is
   true of frus since milestone 754, not of frus 0.5.0, whose default is off: the default
   silently meant off.
2. The hint under the Hot reload box did not change when another option was picked.
3. A hot-reload run stopped after about an hour of building, with no error in the console.
4. The device selector had picked "Android phone", a simulated device, by default.

The directive: hot reload is on by default, wherever the run starts, with every frus.

## What changed

### Hot reload on by default, said explicitly

Both editors now pass `--hot` for a debug run by default, instead of leaving the choice to the
frus installed. frus 0.5.0 (`--hot` known, default off) and frus since 754 (default on) both
patch.

- **Android Studio / IntelliJ**: the run configuration's choices are **On (--hot)**, the
  default; **Off (--no-hot)**; **Let frus decide (no flag)**. Each label says the flag it
  passes, and none promises a default. A configuration saved by the previous plugin with its
  default (`DEFAULT`) reads as On. On leaves a release build without `--hot`, as frus does.
- **VS Code**: `hot` left out passes `--hot` for a debug build; `true` passes it for a release
  build too; `false` passes `--no-hot`; the new `"frus"` passes nothing.
- **`--no-hot` and frus 0.5.0**: 0.5.0 does not know `--no-hot` and refuses to run with it
  ("frus run has no option `--no-hot`"). Both editors already ran `frus help run` to check the
  program speaks the editor protocol; they now also note whether its help mentions `--no-hot`,
  and leave the flag out for a frus that does not know it (off is that frus's default anyway).
- **The CLI**: its own default is unchanged, on for a debug build (`hot_wanted`, milestone
  754). `--hot` on a device it cannot patch (an iOS simulator) is an error naming the device,
  sent as `app.stop`'s error in machine mode, so the editors show it.

### The hint follows the choice

The hint under the box was a static tooltip set once by the form builder. It is now a label of
the editor's own, set from the chosen mode's `hint` at reset and on every selection change.

### Every end is said: the silent stop

Three causes, all fixed:

- **The editors never showed the exit.** The plugin's process handler forwarded `app.stop`'s
  error when there was one, and nothing at all when the process ended otherwise; the VS Code
  adapter ended the session on `exit` without a word. Both now end every run with a console
  line: `frus run ended (exit code 0).`, or `frus run failed (exit code N). The last error: …`
  as an error, the last error being the last line shown as an error (an `app.stop` error, an
  error log or progress line, or a standard-error line that reads `error:`, `Error:` or a
  panic). A run that dies with nothing said reads "It said no error: the lines above are all it
  printed." The VS Code adapter now waits for `close` (all output read) rather than `exit`, so
  the last lines are never lost behind the end.
- **A crash of the application was reported as a success.** Under hot reload the application
  is started by the patch engine, which says `Application [windows] exited with error: exit
  code: 0xc000041d` when it crashes. `frus run` read every "exited" line the same way: it
  printed "<package> has stopped." and exited 0, with no error in `app.stop`. The line now
  carries the engine's text when it says "with error", and the run ends with that as its error.
  This is the most likely cause of the report: a run whose application crashed after the long
  first build ended looking like a normal stop.
- **The engine's own last words were dropped.** Its fatal `Error: …` line and a panic of its
  own were classified as noise, so "the hot-reload engine stopped (exit code: 1)" came without
  its reason. They are now reported as failures.

No timeout is involved: neither editor nor `frus run` limits how long a build may take, and
`frus run --machine` sends the build's output (cargo, the engine) to standard error, which both
editors show.

### Why the first hot run took about an hour

Not a defect of this milestone's scope, but reported here: the first hot run on a machine
installs the patch engine (`cargo install` of a pinned dx from git, about twenty minutes, said
in the console), then builds the whole application into a target directory of its own
(`target/frus-hot/target`), which shares nothing with a normal build. Later runs reuse both;
only the dependency archives (`libdeps-*.a`) are removed before each start, which dx rewrites
in seconds. An hour is consistent with that first run on a slower machine; nothing rebuilds
from scratch each time.

### The default device

The daemon gives a simulated device this computer's category (`desktop`), and both editors
sorted the list by category, then by name: "Android phone" came before "Windows". The plugin
ran on the first available device of that list; the VS Code status bar showed it, while F5
passed no device and frus ran on this computer, so the two disagreed.

Both editors now share one rule, `preferred` / `preferredDevice`: a phone plugged in, else a
running emulator, else this computer, else a browser; never a simulated device or an emulator
that has to be started. Both list devices as `frus devices` does: this computer first, phones,
browsers, then simulated devices. VS Code's F5 passes the preferred device's id when none was
picked, so the status bar and the run agree.

## How it was tested

- **Kotlin** (`./gradlew test buildPlugin -PlocalIde=…Android Studio`, 28 tests, all green):
  `theRunArguments` (On is the default and passes `--hot`, not for release; Off without
  `--no-hot` for an older frus), `hotModesSayWhatTheyDo` (a saved `DEFAULT` is On; no label
  promises a debug-build default), `theDeviceARunGoesToUnlessOneIsChosen`,
  `simulatedDevicesComeLastNotBeforeThisComputer`, `everyEndIsShownWithItsExitCodeAndTheLastError`;
  platform tests `testTheHotReloadHintFollowsTheChoice` (the real settings editor, the hint
  read after each selection) and `testARunThatFailsSaysSoWithItsExitCodeAndTheLastError` (a
  fake `frus` process that prints an error and exits 3: the console's last line names both,
  as an error); the existing clean-stop test now also expects "frus run ended (exit code 0)."
  The plugin zip builds (`build/distributions/frus-intellij-0.1.0.zip`).
- **TypeScript** (`npm test`, 25 tests green, one end-to-end test skipped without a real app):
  the default launch passes `--hot`; `"frus"` and an older frus's `false` pass nothing; a run
  that fails says its exit code and last error; `RunEnding`; `preferredDevice` and
  `sortDevices`. Against the previous sources the new tests do not compile (the functions did
  not exist) and the default launch produced `run --machine` without `--hot`. `npm run package`
  builds the .vsix.
- **CLI**: `dx_says_what_happens` covers an exit with error (carried), a graceful exit (none),
  the engine's `Error:` line and its panic (failures). `cargo clippy -p frus-cli --all-targets
  -D warnings` and `cargo test -p frus-cli` pass.

## Not done

- **Not run against a real frus 0.5.0 inside Android Studio or VS Code.** The behaviour with
  0.5.0 follows from its source (tag `v0.5.0`: `--hot` known, `--no-hot` refused, default off)
  and is covered by unit tests, not by a live session.
- **The crash diagnosis is from the code, not from the reporter's log**: the report had no
  log, and the hour-long build was not reproduced.
- **The CLI's own default device is unchanged**: `frus run` without `-d` still runs on this
  computer even when a phone is plugged in. The editors now always pass a device, so they do
  not depend on it.
- **On never hot-patches a release build in the plugin**; the earlier "On, a release build
  too" choice is gone. VS Code keeps `"hot": true` for that.
- **No version bumps or publishing**: the plugin stays 0.1.0 and the extension 0.2.0; both
  changelogs list the changes under Unreleased. A release of either must bump its version.
