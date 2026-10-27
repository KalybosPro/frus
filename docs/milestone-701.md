# Milestone 701 — System notifications

## Objective

The third part of #219. An application tells the person something while they are elsewhere —
a reminder, a download finished, a message arrived — through the system's own notifications,
and a click on one brings them back to it. frus had no way to send one: an application had to
pick a crate per system, find out that Windows shows nothing from a program it does not know,
and work out how a click on the system's banner could reach its own state.

## What was done

- **`SystemNotification`** (`frus-shell`, re-exported as `frus::SystemNotification`), behind a
  new **`notifications`** feature, off by default like `file-dialogs`:

  ```rust
  Command::perform(|| {
      let response = SystemNotification::new("Water the plants")
          .body("Due today")
          .show();
      Msg::Reminded(response == Ok(NotificationResponse::Clicked))
  })
  ```

  - A title and a `body`, and one way to show it: `show() -> Result<NotificationResponse,
    NotificationError>`, where the response is `Clicked` or `Closed` (dismissed, or taken off the
    screen by the system), and the error says why nothing was shown.
  - The name: the reference uses *notification* for a value that bubbles up the widget tree,
    which frus may want one day; the system's kind says so in its name.
- **It waits, like the file dialogs.** `show` returns when the person clicks the notification or
  it goes away, so it is called from `Command::perform`, or from `spawn` in a component, and the
  answer comes back as a message through paths that already exist. On the interface's thread it
  is refused with an error that says where to call it from. The thread check the file dialogs
  had is now shared (`interface_thread.rs`) rather than written twice.
- **A click brings the window forward.** A `Clicked` answer sets a flag; when the click's
  message reaches the event loop, the window is un-minimized and focused before the message is
  handled. What the click *means* — opening the task — is the application's, in its message.
- **Sent as the application.** The window's title names the sender and its icon goes with it:
  the frus logo unless the application set its own, written once to the application's cache
  folder for the system to read.
  - **Windows** shows no toast from a program it does not know. The first notification of a run
    records the application under the current user, as the system documents for programs that
    are not installed as packages: `HKCU\Software\Classes\AppUserModelId\frus.app.<program>`
    with `DisplayName` and `IconUri`. Nothing else is written, and nothing needs elevation.
  - **macOS**: the bundle's identifier when the program was started as a bundle; otherwise the
    library's default sender, which macOS shows as the Finder.
  - **Linux**: the desktop's notification server over D-Bus, with the click listed as the
    `default` action, which is how the server knows a click on the notification itself is
    wanted.
- **notify-rust 4.18.2** underneath: one crate for the three systems with a typed answer
  (`wait_for_response`) on each. Its D-Bus client is zbus, pure Rust and the same one the file
  dialogs' portal uses, so no system library is needed to build on Linux. Only in the desktop
  dependency table.
- **The demo**: a task's screen has a **Remind me** action. The reminder carries the task's title
  and state; a click on it opens the task, wherever the demo is by then.
- **Found on the way: every icon button was a toggle.** `IconButton` described itself as
  `toggled(selected)` whether or not it was one, so a back arrow, a menu, a close was offered to
  assistive technology as a toggle switched off — on Windows, with the Toggle pattern and no
  Invoke, so it could be switched but not pressed. It is a toggle now only once
  `selected(..)` has been called, which is how the reference tells the two apart.

## Checked on the machine

On Windows 11 (150 %), driven through UI Automation, with no pointer:

| Step | Seen |
|---|---|
| A notification from a small program using the same calls | a toast from **frus — Todo**, with the frus logo; unanswered, `Closed` after 6.5 s |
| The demo's **Remind me**, the demo window in front | no banner; `show` returns `Closed` after about 7 s |
| The same, the window minimized right after the press | the toast: *frus — Todo · Water the plants · Still to do* |
| The toast, read through the system's own tree | `Nouvelle notification de frus — Todo, Water the plants, Still to do.` |
| The toast invoked | `show` returns `Clicked` 3 s after it was shown; the window goes from minimized to normal, on the task's screen |

- **Windows shows no banner for the application in front.** A notification from the program
  whose window has the focus does not pop up; the same call from a program behind it does, and
  so does the demo's once its window is minimized. The documentation says so: a notification is
  for the person who is not looking at the application.
- **The toast is not among the desktop's windows** in UI Automation: it was reached through the
  native window under its position (`Windows.UI.Core.CoreWindow`, *Nouvelle notification*, in
  `ShellExperienceHost`) and `AutomationElement::FromHandle`, and invoked only after checking
  that it held the task's words.

## Left

- **A click on a notification in the notification centre, later.** The in-process click event
  only fires while the toast is on the screen. A click from the centre afterwards starts the
  program through a registered activator (a COM server on Windows), which frus does not
  register; `show` has returned `Closed` by then.
- **Buttons, replies, images, sounds, and updating or withdrawing one that is shown.**
- **Android and the web.** Android needs a channel and, since Android 13, the person's
  permission; the web's Notifications API needs the page's permission too. Both are their own
  milestone.
- **Not run on macOS or Linux.** Linux type-checks from Windows (`clippy --target
  x86_64-unknown-linux-gnu`); macOS cannot, as its notification library compiles Objective-C, so
  it is compiled by CI's macOS job, through the demo, which turns the feature on. Neither was
  run on a machine. The macOS path in particular — the library's `NSUserNotificationCenter`,
  deprecated since macOS 11 and still working — wants a look on a Mac.
- The rest of #219: the tray icon, taskbar badges and progress, global shortcuts.

## Tests

- `notification.rs`: the description is kept; anything but a close is a click (every close
  reason, and a button and a reply as clicks); refused on the interface's thread before
  anything reaches the system; a raise is taken once; the Windows identity keeps to the
  characters an identity may hold, and to its length.
- `file_dialog.rs`: its interface-thread test, on the shared check.
- `iconbutton.rs`: `only_a_selected_button_is_a_toggle`.

## Mutation testing

Nine hand mutants, nine killed, each by a named test: every answer a click, every answer a
close, a raise kept once taken, no interface-thread check, an identity left unsanitized or
unbounded, every icon button a toggle, a toggle never said to be one, any toggle taking the
accent.
