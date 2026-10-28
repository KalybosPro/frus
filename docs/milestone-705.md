# Milestone 705 — A refused notification answers at once on Android

## Objective

Milestone 703's "not checked": the Android 13 permission on a device. Checking it on the
Android 16 emulator showed the question working, the *yes* working — and, from the code read
against what the *no* does, that a refusal could not be told from a question still open.

## The bug

`FrusNotifications.allowed` answers 1 (allowed), -1 (refused for good: turned off, or not in the
manifest) or 0 (not allowed yet). After the person taps **Don't allow**, the permission is
exactly what it was before the question — not granted — so `allowed` keeps answering 0, and the
loop that waited for an answer kept waiting: **a full minute**, after which `show` said the
person *did not answer*. The same for the person who had already refused twice, whom Android no
longer asks at all.

## What was done

- **The question's dialog tells the answers apart.** It is a window over the application's, so
  it takes the focus while it is up and gives it back once answered. `show` now watches the
  focus as well as the permission (`FrusNotifications.focused`, `Activity.hasWindowFocus`):
  - the permission granted, or refused for good → settled;
  - the focus lost, then back, the permission still not granted → the person said no;
  - the focus never lost for two seconds → no dialog came: Android refused without asking;
  - a minute with the dialog still up → not answered.
- The decision is one function, `notification::question`, shared and tested on the host; the
  Android side only feeds it what it sees.
- The error now says which: *the person did not allow notifications*, or *did not answer*.
- **The demo** logs a reminder that was not shown, with the reason, as an application should say
  it rather than drop it.

## Checked on a device

The Android 16 emulator (`google_apis_playstore`, x86_64, its disk on D:), the demo installed
fresh, the permission reset before each case with `pm revoke` and `pm clear-permission-flags`:

| Case | Seen |
|---|---|
| **Remind me**, the question | *Allow **frus demo** to send you notifications?* — the system's dialog, with **Allow** and **Don't allow** |
| **Allow** | `POST_NOTIFICATIONS: granted=true, USER_SET`; the reminder posted: channel `frus.notifications`, its title, *Still to do*, **Mark done** |
| **Don't allow** | `granted=false, USER_SET`; nothing posted; the demo's warning **0.4 s** later: *the person did not allow notifications* |
| Refused twice before (`user-fixed`) | no dialog; nothing posted; the warning **3.6 s** after the press — the two seconds given a dialog to come, and the polling |

This also closes milestone 703's "not checked": the question, and the *yes*, on Android 13 and
later. Still not checked there: the picture.

## Tests

- `notification.rs`: `the_question_is_settled_by_the_focus_coming_back` — settled by an
  answer, by the focus coming back, by no dialog after two seconds; open while the dialog is up;
  unanswered after a minute.

## Mutation testing

Five hand mutants, five killed by the test: the old behaviour (only the permission settles the
question), the focus coming back ignored, no dialog never settling, settled while the dialog is
up, never unanswered.
