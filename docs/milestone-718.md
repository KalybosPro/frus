# Milestone 718 — The system around the application, in the guide

## Objective

The guide (milestones 715–717) covered what happens inside the window. What an application
does with the system around it — open a file the person picks, notify them when they look
elsewhere, sit in the tray — was spread over the rustdoc of `frus-shell`'s modules and the
milestone notes of #219, with no page that said which platform has what.

## What was done

`docs/guide/platform.md`, *The system around the application*:

- a table of the services, the feature each needs, and the platforms that have it;
- file dialogs, run off the interface's thread with `host::spawn` (and why: a dialog asked for
  on that thread is refused);
- files dropped on the window, with `DragTarget::on_files` and `accepts_files`;
- notifications, through `show_async` in an `async` handler, with the Android 13 permission and
  the browser's first-click rule;
- the tray, the taskbar badge and progress, and global shortcuts, each described from the
  application's state, with what a shortcut can and cannot take;
- the application's icon and a single instance.

The page is listed in the guide's index and in both READMEs' *Learn* tables.

## Every example is compiled

The page's seven examples need every platform feature, so it is included in the `frus` crate's
doctests only when `file-dialogs`, `notifications`, `tray`, `taskbar` and `global-shortcuts`
are all on. CI's Linux step that already installs GTK for the tray now also runs
`cargo test -p frus --features "file-dialogs notifications tray taskbar global-shortcuts" --doc`.
All seven compiled on the first run here.

## Not done

- **Permissions in general** (the camera, the location, the microphone): frus has no API for
  them yet, so the page does not pretend to; the Android notification permission is the one it
  documents, because notifications need it.
- **The guide in French.**
