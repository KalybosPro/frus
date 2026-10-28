# Milestone 704 — Notifications on the Web, and awaited everywhere

## Objective

Milestone 701's "left": `SystemNotification` in the browser. A page cannot block, so the call
that waits for the person's answer had to become one that is awaited — and, to keep one way of
writing a reminder, the awaited call had to exist on every platform, with a way for components
to await it.

## What was done

- **`SystemNotification::show_async()`**, on every platform: the answer awaited rather than
  waited for. On the Web it is the browser's notification; natively it is `show`, moved to the
  pool the shell keeps for blocking calls (`blocking::unblock`), so it holds no executor worker.
  `show` stays, for `Command::perform` and the native platforms, and is not on the Web.
- **The browser's notifications** (`notification_web.rs`):
  - **The permission** is asked for the first time a notification is shown. A page refused, or
    a person who did not answer, is an error that says so — and that some browsers only ask
    after a click of theirs, so the first notification is best shown from a press.
  - **The answer** is the notification's events — `click`, `close`, `error` — made one awaited
    promise. A click focuses the page's window and closes the notification.
  - **What a page's notification cannot carry**: buttons and a picture are a service worker's,
    which a page alone has not got. They are left out, and the notification shown with its
    title and body; a log line says so. Its icon is the page's own — the tab's, which is the
    application's unless the page names another. `silent` is passed on.
- **`host::spawn_async(work, then)`** for components: the awaited counterpart of `spawn` — a
  future driven by the browser on the Web and by the shell's executor natively (where it must be
  `Send` and must not block, which is `spawn`'s), and `then` run with its answer on the
  interface's thread. A new `Effect::Async`, which the shell turns into `Command::run_async`.
- **The demo's reminder** uses `show_async` and `spawn_async`, so the same code serves every
  platform; it is now in the Web build too.

## Checked in a browser

Microsoft Edge on Windows 11, with a profile of its own, driven over the DevTools protocol; the
page's notification permission granted through the protocol; the demo opened on its task; its
buttons pressed through the accessibility bridge (milestone 564) — which is where **Remind me**
was found, the J700 tappable area at work in the browser.

| Case | Seen |
|---|---|
| **Remind me** | the page builds one notification, *Water the plants*, and the browser reports it shown: Edge draws it in its own pop-up — *Water the plants · Still to do · localhost:8714* |
| The page moved to its list (`#/`); the pop-up clicked | the page sees `click`, then `close`; Edge's window, minimized in the first run, comes back; the address goes to **`#/task/0`** — the click answered, the demo opened the task |
| The page on its list; the pop-up's close button | the page sees `close` only; the address stays `#/` — a dismissal, not a click |

- **Edge drew its own pop-up**, not a Windows toast, for this fresh profile. UI Automation finds
  it (`Chrome_WidgetWin_1`, the check's own Edge), but its body has no Invoke pattern: the
  click was a pointer click on its words, made only after checking the window under that point
  was still the pop-up. Its close button does have one.
- **A page moved while its window was minimized did not move the application**: a minimized
  page paints no frames, and the router had not taken in the new address, so the click's `go`
  to the task changed nothing. The check keeps the window visible for that reason; the
  framework is not involved.

Not checked: the permission question itself (it was granted through the protocol), a refusal,
another browser, and a notification that became a Windows toast.

## Left

- **Buttons and pictures on the Web**, through a service worker — the page's own, which frus
  does not ship.
- The rest of #219: the tray icon, taskbar badges and progress, global shortcuts.

## Tests

- `host.rs`: `an_awaited_answer_reaches_then_once` — nothing before the work is awaited,
  nothing before its delivery runs, the answer once.
- `widget_app.rs`: `an_awaited_work_becomes_an_awaited_command` — one future, not a blocking
  task.

## Mutation testing

Five hand mutants. Four killed: the answer not kept, no delivery answered, the effect dropped by
the shell, the effect run as a blocking task (the last two by the shell's test, added for them —
the first run let them through). One was equivalent and does not count: clearing the slot after
`then` had already taken its value out.
