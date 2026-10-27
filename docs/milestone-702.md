# Milestone 702 — Buttons, a picture and the system's sound on a notification

## Objective

The first of milestone 701's "left": a notification that can be answered from where it is —
*Mark done*, *Reply later* — without opening the application, with a picture when one says more
than a line of text, and with the sound every other application's notification makes.

## What was done

- **`SystemNotification::button(id, label)`**, one call per button, in order. A press answers
  **`NotificationResponse::Button(id)`**, a new case beside `Clicked` and `Closed` (the enum is no
  longer `Copy`, as it now carries the id).
  - **A button does not bring the window forward.** A button such as *Snooze* is answered
    without coming back, which is what it is for; a click on the notification itself still
    raises the window. The application raises itself when a button means to open it.
  - **Ids are marked on the way out** (`button:<id>`) and unmarked on the way back. Linux
    reports the click on the notification itself as the action `default`; an application's own
    button called `default` would otherwise be read as that click.
  - Windows shows up to five side by side; Linux as many as the notification server offers;
    macOS one — with several, a menu of them on its single button, whose answer the library maps
    back to the chosen id.
- **`SystemNotification::image(path)`**: a PNG or JPEG shown in the notification — under the
  text on Windows, beside it on macOS, where the server puts one on Linux.
- **The system's sound, and `silent()`.** Milestone 701's notifications were **silent on
  Windows**: the library names no sound unless told one, and a toast with no sound named is
  written with `<audio silent="true"/>`. Every notification now makes the system's own sound —
  `Default` on Windows, `NSUserNotificationDefaultSoundName` on macOS, where an unnamed one is
  silent too; Linux's server plays its own — and `silent()` turns it off (on Linux, through the
  `suppress-sound` hint).
- **The demo**: the reminder has a **Mark done** button while the task is still to do. Pressing
  it marks the task done and leaves the window where it is; a click on the reminder still opens
  the task.
- The module's documentation says what milestone 701 found: Windows shows no banner for the
  application whose window is in front.

## Checked on the machine

Windows 11, driven through UI Automation, with no pointer:

| Step | Seen |
|---|---|
| **Remind me**, the window minimized | the toast, read through the system's tree: `frus — Todo · Water the plants · Still to do`, and a button **Mark done** |
| **Mark done** pressed on the toast | `show` returns `Button("done")`; the window **stays minimized**; restored, the task reads **Done** and its action has become **↺** |
| The toast as Windows stored it (its notification database, read from a copy) | 701: `<audio silent="true"/>` on every one; 702: no audio element — the system's sound — and `<action content="Mark done" arguments="button:done"/>` |
| A toast with a picture (a small program making the same call) | the frus logo, large, under the title and the text |

## Left

- **Updating or withdrawing a notification that is shown**: Linux and the newer macOS API allow
  it, Windows needs the toast's tag, which the library does not expose.
- **A reply typed into the notification** (Windows and macOS have one).
- Everything else of milestone 701's "left": a click from the notification centre later,
  Android, the web, a run on macOS and Linux, and the rest of #219.

## Tests

- `notification.rs`: buttons keep their order and the picture and the silence are kept; what
  the system reports is read — a close is a close, a marked action a button (one called
  `default` included), anything else a click; the system's sound unless silent, per system.

## Mutation testing

Eight hand mutants, eight killed, each by a named test: buttons answered as a click, the mark
left on the id, `silent` ignored, no sound named on Windows, a button not kept, buttons
reversed, the picture not kept, the silence not kept.
