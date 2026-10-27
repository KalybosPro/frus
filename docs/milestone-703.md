# Milestone 703 — Notifications on Android

## Objective

Milestone 701's "left": `SystemNotification` on Android, with the same calls and the same
answers as on the desktop — so a reminder, a finished download, a message is written once.

## What was done

- **`SystemNotification` on Android**, behind the same `notifications` feature, through
  `FrusNotifications` in the bundled dex (the loader the input bridge already makes):
  - **Channels.** Android 8 and later file every notification under a channel the person can
    tune: one for notifications, one for silent ones (lower importance, no sound), made the
    first time. Before Android 8 the sound is the notification's own default.
  - **The permission.** From Android 13 the person must allow notifications. The first
    notification asks them and waits — up to a minute — for their answer. The question is only
    put to an application whose manifest asks for `android.permission.POST_NOTIFICATIONS`;
    without it the system refuses silently, so `show` checks the manifest and says what is
    missing. With cargo-apk, the manifest's permission is
    `[[package.metadata.android.uses_permission]]`.
  - **A click opens the application** through its launch intent, which brings the running
    activity forward rather than starting a second one. It has to be the activity directly:
    since Android 12 a notification may not open one from a broadcast (a "trampoline").
  - **Buttons and dismissals** are broadcasts to a receiver the Java side registers (not
    exported, on Android 13 and later), which records the answer — and takes the notification
    away after a button, which Android leaves in the shade otherwise.
  - **Telling a click from a dismissal.** A click sends the application nothing — it is only
    the activity opening — so `show` watches the shade: a notification that has left it with
    nothing recorded was clicked, as a dismissal records itself. The broadcast may arrive just
    after the notification leaves, so the answer is read again half a second later.
  - Up to three buttons, the system's limit; a picture shown large (`BigPictureStyle`) and as
    the notification's icon; the application's own icon and name.
- **One `show`, two backends.** `notification.rs` keeps the description, the answer and the
  marking of buttons for every system; the desktop's library and the Android bridge are the
  two ways it is shown. How a recorded answer is read (`left_the_shade`) is shared, so it is
  tested on the host.
- **The demo** asks for the permission in its manifest, and its task screen's **Remind me**
  is on Android too.

## Checked on a device

Infinix X669, Android 12, the demo started with `am start -n` (not from the launcher, which is
what hid the first bug below), driven by `adb` and `uiautomator`:

| Case | Seen |
|---|---|
| **Remind me** | the notification in the shade as *frus demo*, with the application's icon: *Water the plants · Still to do* and a **MARK DONE** button; `dumpsys notification`: channel `frus.notifications`, importance 3, an activity as its click, broadcasts as its dismissal and its button |
| The demo on its list; a tap on the notification | it leaves the shade; the demo comes forward **on the task's screen**, in the same process, with no second activity |
| On the task; **MARK DONE** pressed | it leaves the shade; the task reads **Done** and its action becomes **↺**; the demo stays where it was |
| The demo on its list; the notification swiped away | it leaves the shade; the demo stays on its list — a dismissal, not a click |

Two bugs the phone found, both fixed before this was written:

- **A tap started a second activity.** The click was the launch intent as the package manager
  gives it, which reaches the running activity only when the application was started from the
  launcher; otherwise Android creates a second `NativeActivity`, whose `android_main` stops at
  once ("EventLoop can't be recreated") — the tap did nothing, or worse. The intent now carries
  `FLAG_ACTIVITY_NEW_TASK | FLAG_ACTIVITY_SINGLE_TOP`.
- **An intent from an earlier run was reused.** Notification numbers start at 1 in every run,
  so the intents' request codes repeat, and `FLAG_UPDATE_CURRENT` keeps an existing intent's
  flags — the record `dumpsys activity intents` showed was the very one made by the first build,
  still without the flags above. The intents are now made with `FLAG_CANCEL_CURRENT`, which
  also keeps a leftover notification's button from answering for a new one.

Not checked on a device: the Android 13 permission (the phone runs Android 12; the emulator,
on Android 16, could not start for lack of disk space), and the picture.

## Left

- **The web**, the next milestone.
- **The Android 13 permission on a device**, and the picture.
- **A link opened while the application runs** has the same second-activity problem, through
  the link's own intent filter (see `android_link.rs`); that one is for the application's
  manifest (`launchMode`) and is not changed here.
- **A click on an application that the system had stopped**: the notification opens it again,
  but `show`'s waiting thread died with the process, so the click is not answered. The
  application learns of it only as a fresh start.
- Updating or withdrawing a shown notification; an inline reply.

## Tests

- `notification.rs`: what left the shade is read — nothing recorded is a click, `closed` a
  dismissal, a marked id a button, anything else a dismissal.
