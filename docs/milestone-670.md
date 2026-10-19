# Milestone 670 — `frus run --hot`

## Objective

Milestone 669 made frus patchable and showed a running application patched in place. This one
puts it behind one flag, for an application as `frus create` writes it: `frus run --hot`.

## What was done

- **The engine is installed by frus.** `dx` 0.7.10 with the Windows stub fix (DioxusLabs/dioxus
  #5966, pull request #5967), from a pinned commit of a fork, into
  `~/.frus/tools/dx-<commit>`. Built once, about 20 minutes. The `git` program fetches it: the
  repository is large, and cargo's own client gave up on a connection that dropped for a
  moment.
- **A shadow package**, `target/frus-hot/<package>/`, so that the application's manifest is never
  changed:
  - its manifest is the application's, with the library and the binaries replaced by one
    program built from `src/lib.rs`, relative paths made absolute, and a `[workspace]` of its
    own;
  - its folders are links to the application's (junctions on Windows, symbolic links elsewhere),
    **except `src`**: `dx` does not see a change through a link, neither through the link's path
    nor through the folder's own. So `src` is a copy, and `frus run` keeps it in step four times a
    second (`Mirror`): a file copied again when its size or time changes, and only if its bytes
    did; a file removed when the application's is gone;
  - a `src` link left by an earlier version is taken away — the link alone, never what it points
    to.
  - Writing the program's path into the application's own manifest (`[[bin]] path =
    "src/lib.rs"`) was rejected: cargo then warns, on every build, that the file belongs to two
    targets.
- **The run**: `dx serve --hot-patch` on the shadow, with frus's `hot-reload` feature and its own
  build folder. Its output becomes the run's: the application's log as it would be in a normal
  run, "Patched `<file>` in 3.5 s." for each patch (`app.restarted` for an editor), and the
  failures. The keys are a normal run's: `dx` hands the application `FRUS_DEV`, and `w`, `p`,
  `s` and the others reach it. `r` says that a save is all it takes; `R` builds and starts it
  afresh; `q` stops `dx` and the application together.
- **A half-written archive is cleared first.** `dx` links the dependencies into a
  `libdeps-*.a`; stopped while writing it, it leaves one the next build cannot read ("unknown
  file type"). The run deletes them before starting `dx`, which writes them again in seconds.
- **`main!` writes its `main` only with the `hot-reload` feature** (on #275). Written always, it
  broke any crate calling `main!` beside its own `main`, the README's doctest first. The choice is
  made inside frus-shell, so the application's crate never sees an unknown condition.
- **VS Code**: `"hot": true` in a launch configuration.

## Verified

- On Windows 11, a copy of the counter example as `frus create` would write it (no `[[bin]]`),
  counting on a timer: `frus run --hot` installed the engine, built, and started it. The label
  was changed in the source: `Patched src\features\counter\widgets\count_display.rs in 3.5 s.`,
  the window showed the new text, and the count went on (1 → 11). `w` printed the widget tree.
  After `q`, neither `dx` nor the application was left running.
- Unit tests: the shadow manifest; `dx`'s output read line by line; the mirror (copies, changes,
  removals, a fresh run over a copy already in step); the archives cleared; the VS Code launch.

## Left

- Android and the web: subsecond supports both; `--hot` says so and points to `r` for now.
- macOS and Linux: the flags are there, not tried.
- Compiler errors point at the shadow's copy of `src`, not the application's.
- The engine comes from a fork until a released `dx` has the fix.
