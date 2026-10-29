# Milestone 706 — The tray icon

## Objective

The fourth part of #219: "a tray / menu-bar icon with a menu, and a way to keep running when
the last window is closed". The applications that live there — a chat client, a sync tool, a
timer — need both, and frus had neither.

## What was done

- **`Tray`** (`frus-shell`, re-exported as `frus::Tray`), behind a new **`tray`** feature,
  off by default like the file dialogs and the notifications:

  ```rust
  Tray::new("Tasks")
      .item("New task", Msg::NewTask)
      .separator()
      .show_item("Open Tasks")
      .quit_item("Quit")
      .keep_running()
  ```

  - A tooltip, an icon (the application's unless `icon(..)` says otherwise), and a menu of
    items, disabled items and separators.
  - **An item sends its message.** `show_item` and `quit_item` do what every tray's menu
    offers, with no message to write and no command to add: bring the window forward, end the
    application.
  - **A click** on the icon brings the window forward, unless `on_click(message)` says
    otherwise. The menu opens on the other button; on macOS on the click, as the menu bar does.
  - **`keep_running`**: closing the window hides it, and the application goes on in the tray
    until a quit item.
- **Declared from the state**, like the title and the subscriptions:
  `Application::tray(&self) -> Option<Tray<Msg>>`, or `FrusApp::tray(|| …)` for an
  application of components, asked again after every change. The shell compares what the
  system is shown — the tooltip, the icon, each item's label and whether it is enabled — and
  changes only what differs; the messages are never shown to the system, so they need no
  comparing. When the person picks an item, the description is asked for again and the item at
  that place does what it says *now*, so a message is never stale and never needs cloning.
- **tray-icon 0.26.1** underneath (with its menus, muda): the notification area on Windows, the
  menu bar on macOS, and on Linux the desktop's status area over D-Bus (its `ksni` backend,
  not the older indicator library a desktop may not have). The menus are made with GTK 3 on
  Linux, so building there needs `libgtk-3-dev`; the demo turns the tray on for Windows and
  macOS only, and CI builds and tests the Linux side on its own, in the Linux test job.
- **Waking the loop.** The library's events arrive on the event loop's thread on Windows and
  macOS, and on a thread of their own on Linux: each one is queued and asks the window for a
  redraw, which brings the event loop round on every system, and the shell acts on the queue
  there.
- **The demo** has a tray on Windows and macOS: how many tasks are left (kept up to date), a
  **Clear completed** item that is only enabled when there is something to clear, **Open frus
  demo**, **Quit**, and it keeps running once its window is closed.

## Checked on the machine

Windows 11, the demo started with one task done and one to do, driven through UI Automation:
the icon was invoked through it, the menu opened with one right-click — made only after
checking the window under the point was the tray's own flyout — and its items chosen with the
keyboard, after checking the window in front was the demo's.

| Step | Seen |
|---|---|
| The demo starts | its icon, *frus demo*, among the hidden icons of the taskbar's corner |
| The window closed | the window hidden; the process still running |
| A click on the icon | the window back |
| The menu | *1 task to do* (disabled) · **Clear completed** · — · **Open frus demo** · **Quit** |
| **Clear completed**, then the menu again | the done task gone; **Clear completed** now disabled — the menu followed the state without being asked |
| **Quit** | the process ends |

Found on the way, and not frus's:

- **UI Automation sees the menu's window but none of its items**, so a screen reader cannot read
  the tray's menu. The items are plain Win32 menu items (muda does not draw them itself); why
  they are not exposed was not found.
- **The menu does not open from the keyboard.** The library opens it on the right button
  coming up, and does not answer the message the system sends a tray icon for the menu key or
  Shift+F10.
- **UI Automation gives the tray's tooltip the icon's name**, and in the overflow flyout places
  it where the icon is not — a trap for anyone scripting the tray.

## Left

- The two accessibility gaps above, which belong upstream (tray-icon, muda) or to a menu of
  frus's own.
- **Not run on macOS or Linux.** macOS type-checks only in CI; Linux builds and tests in CI.
- Icons that change with the state (a badge on the tray icon), a submenu, checkable items.
- The rest of #219: taskbar badges and progress, global shortcuts.

## Tests

- `tray.rs`: each place in the menu does what it says (an item its message, a disabled item and
  a separator nothing, the built-in items theirs, past the end nothing); a click shows the
  window unless told otherwise; the shape is what the system sees and leaves the messages out.

## Mutation testing

Six hand mutants, six killed: a click that always shows the window, items counted from one, a
disabled item that acts, a shape blind to whether an item is enabled, a shape blind to the
icon, `keep_running` not kept.
