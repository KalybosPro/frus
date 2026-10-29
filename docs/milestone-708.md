# Milestone 708 — The taskbar's badge and progress

## Objective

The fifth part of #219: "taskbar / dock badges and progress (a count, a progress bar on the
taskbar entry)". A mail client's unread count, a download's progress — seen on the
application's button from whatever window the person is in.

## What was done

- **`Taskbar`** (`frus-shell`, re-exported as `frus::Taskbar`), behind a new **`taskbar`**
  feature, off by default like the tray:

  ```rust
  Taskbar::new().count(3).progress(0.4)
  Taskbar::new().indeterminate()
  ```

  - A **count**; `0` shows none, so a count straight out of a model needs no `if`.
  - A **progress**: a fraction (kept within 0 and 1), `indeterminate`, `paused(f)`, `error(f)`.
- **Declared from the state**, like the title and the tray: `Application::taskbar(&self)`, or
  `FrusApp::taskbar(|| …)` for an application of components, asked again after every change.
  The shell changes the system's button only where the description differs, and only when it
  has a real window — the windowless driver tests run through has none.
- **Windows** — the button's own progress bar through the shell's `ITaskbarList3`
  (`SetProgressState`, `SetProgressValue`): accent-coloured, yellow when paused, red for an
  error, moving when indeterminate. The count is an **overlay icon**, which the system draws
  over the button's icon but does not make: a red disc with the number in white — the scheme's
  error red, as a `Badge` has — drawn here, at 32 pixels, with a small built-in figure font
  (`99+` past 99, which is what fits). The overlay's accessible text is the count.
- **macOS** — the count is the Dock icon's badge (`NSDockTile::setBadgeLabel`). The Dock has no
  progress bar of its own; the progress is not shown there.
- **Linux** — both go to the desktop's launcher through the `com.canonical.Unity.LauncherEntry`
  `Update` signal (count, progress, and whether each is shown), which KDE's task manager,
  Ubuntu's dock, Dash to Dock and others read. The launcher finds the application by its desktop
  entry: the program's name unless `Taskbar::desktop_entry` says otherwise.
- **The demo**: its button shows how many tasks are left and, while the list is partly done,
  how much of it is.

## Checked on the machine

Windows 11, the demo started with three tasks, one done; changed through UI Automation; its
taskbar button photographed after each step:

| Step | The button |
|---|---|
| Three tasks, one done | a red **2** on the icon's corner, and a progress bar a third of the way |
| The done task deleted | **2**; no progress bar — nothing done |
| Another task deleted | **1** |

The shell applied each change once and only when the description changed.

Found on the way, not in this milestone's code: **UI Automation's Toggle on a task's checkbox,
and its SetValue on the new-task field, did nothing** — a checkbox stayed on, the field stayed
empty — while Invoke on a button works. Two accessibility gaps for a later milestone.

## Left

- **macOS progress**, which needs a view of frus's own drawn into the Dock tile.
- **Not run on macOS or Linux.** Both type-check from Windows (`clippy --target
  x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`) and build in CI.
- The rest of #219: global shortcuts.

## Tests

- `taskbar.rs`: the description is kept, the last progress winning; a fraction is kept within
  0 and 1 and NaN is 0; a count reads up to `99+`, and 0 as nothing; the overlay is a red disc,
  transparent at its corners, with white figures within it — more for `99+` than for `7`.

## Mutation testing

Seven hand mutants, seven killed: a fraction not clamped, NaN kept, a zero count shown, `99+`
one early, no disc, no figures drawn, a paused progress kept as a normal one.
