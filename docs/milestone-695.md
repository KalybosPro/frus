# Milestone 695 — Files dropped onto the window

## Objective

The first part of #219. On the desktop, winit tells the window when files from the system's
file manager are held over it (`HoveredFile`), taken away (`HoveredFileCancelled`) and dropped
(`DroppedFile`), and the shell ignored all three: an application could not take a file the way
every desktop application does. They should reach `DragTarget` the way an item carried inside
the application does — the target under the pointer lights up while the files are over it, and
is handed their paths on the drop.

## What was done

- **`DragTarget::on_files(|paths: Vec<PathBuf>| …)`** and **`DragTarget::accepts_files(|paths:
  &[PathBuf]| …)`**. The paths come in the order the system listed them. A target only takes
  files once it has `on_files`; `accepts_files` narrows it (only images, say), and a target that
  refuses is not highlighted, so the answer is visible before the button is let go — the same
  rule as `accepts` for items. The highlight is the target's existing one, from
  `Status::drag_over`: nothing new to style.
- **A target for files only turns items away.** A target given `on_files` and neither `on_drop`
  nor `accepts` no longer lights up under a card being carried inside the application: it is a
  place for files. A target with neither handler is unchanged (open to anything), and one with
  both takes both.
- Two new `Widget` methods, `accepts_files` and `on_files_dropped`, descriptive rather than
  structural, forwarded by every wrapper that forwards `on_drop`: `Box<dyn Widget>`, the
  `transparent` macro (`Keyed` and friends) and `Responsive`.
- `Ui::drop_zones()`, every drop target of the frame in paint order.
- **The shell** (`file_drop.rs`, and three arms in `window_event`):
  - The system names the files **one event per file**, for the hover and for the drop. They are
    gathered in a `FileDrag`; a drop starts a fresh list (the files dropped are the ones that
    count) and is handed over **once**, in `about_to_wait`, when the loop has had every file of
    it — one message with every path, not one per file.
  - **Where the files are.** winit 0.30 reports none of the three events with a position, and
    while the system runs the drag the window is sent no pointer movement. On **Windows** the
    shell reads the cursor itself (`GetCursorPos` + `ScreenToClient`) when the files arrive, on
    the drop, and thirty times a second while they are held (the loop wakes for it only during
    such a drag), so the highlight follows the pointer from one target to another. Where the
    position cannot be read, the files go to **the one target that takes them** when there is
    only one, and otherwise to the target under the last place the pointer was seen.
  - The chosen target is `runtime.drag_over`, the very field an in-app drag sets, so the
    highlight is the same code path. Taking the files away or dropping them clears it.
- On the web and on Android the events never come, so nothing happens there; everything
  compiles on every target (`PathBuf` is plain `std`).

## Left

- **macOS and Linux follow the pointer only roughly.** Without a position from winit 0.30, a
  window with several targets that take the same files gets them on the target under the last
  pointer position seen before the drag. winit 0.31 reports positioned drag events
  (`DragEntered`/`DragMoved`/`DragDropped` with a position); moving to it, or reading the
  pointer through each platform's drag API, answers that.
- **Not verified with a real drag from the file manager.** The shell's handling is driven
  through the windowless driver; no desktop run dropped a real file on a window.
- The demo does not take dropped files yet ("dropping a file on the demo's window shows its
  name", from the issue).
- No drop effect is chosen (copy, move, link): the system shows its default.
- Dropping files onto the web canvas (the browser's own drag events), and the rest of #219:
  native file dialogs, system notifications, the tray icon, taskbar badges and progress, global
  shortcuts.

## Tests

- `dragdrop.rs`: a target takes no files unless it says so; nothing dragged is nothing to
  take; the paths reach the drop untouched and in order; a target refuses the files its
  predicate refuses; a files-only target turns items away, unless it takes items too or says
  which, and a target with neither handler is still open to anything; a `Keyed` target still
  takes files.
- `file_drop.rs`: a known position picks the target under it, and nothing below both; a
  refusing target is not offered the files; with no position the only taker takes them, and
  with two the last pointer position decides; a drop replaces what was hovered and names each
  file once.
- `app.rs` (`file_drop_tests`, through the `Driver`): files held over a target set `drag_over`
  and draw exactly one outline, the size of that target; the drop sends one message with every
  path and the highlight goes; the highlight follows the files from one target to the next and
  off both; a target that refuses a picture stays dark and a drop on it sends nothing; files
  taken away leave nothing lit and send nothing; a drop hands over what was dropped, not what
  was hovered; with no position the only taker takes them.

## Mutation testing

Seven hand mutants, each killed by a test:

| mutant | killed by |
|---|---|
| a target that refuses the files is offered them anyway | `a_target_that_refuses_is_not_offered_the_files` |
| with no position, the only taker loses to the target under the last pointer | `with_no_position_the_only_taker_takes_them` |
| a drop keeps the paths hovered before it | `a_drop_replaces_what_was_hovered_and_names_each_file_once` |
| a file named twice is kept twice | the same |
| a target with `on_files` alone still takes in-app items | `a_target_for_files_only_turns_items_away` |
| an empty set of files is delivered | `a_target_takes_no_files_unless_it_says_so` |
| the highlight is not cleared when the files land | `files_held_over_a_target_light_it_up_and_land_on_it` |
