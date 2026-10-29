# Milestone 709 — Global shortcuts

## Objective

The last part of #219: "global shortcuts, with a documented position on the security
implications". A combination that reaches the application from whatever window has the
keyboard — a quick-add, a push-to-talk, the shortcut that brings a tray application back.

## What was done

- **`GlobalShortcut`** (`frus-shell`, re-exported as `frus::GlobalShortcut`), behind a new
  **`global-shortcuts`** feature, off by default:

  ```rust
  vec![
      GlobalShortcut::new("Primary+Shift+Space", Msg::QuickAdd),
      GlobalShortcut::show_window("Primary+Alt+T"),
  ]
  ```

  - A combination is its modifiers and one key: `Ctrl`, `Alt` (`Option`), `Shift`, `Super`
    (`Cmd`, `Meta`), and **`Primary`** for the platform's own — `Cmd` on macOS, `Ctrl`
    elsewhere — so one declaration is right on every system.
  - It sends its message, or, with **`show_window`**, brings the window forward and shows it if
    the tray hid it: the shortcut a tray application most often has.
- **Declared from the state**, like the tray: `Application::global_shortcuts(&self)`, or
  `FrusApp::global_shortcuts(|| …)`. The shell registers what is declared and gives back what is
  withdrawn — only with a real window, as the tray. A press asks the declaration again, and the
  shortcut with those keys does what it says now.
- **One spelling per combination**: the modifiers in a fixed order, the key upper case, so
  `"shift+ctrl+k"` and `"Ctrl+Shift+K"` are one shortcut, not two registrations.
- **global-hotkey 0.9** underneath: `RegisterHotKey` on Windows, the Carbon hot keys on macOS,
  a key grab on X11 — pure Rust, no system library to build against.

## The position on security

Written into the module's documentation, as #219 asked:

- A global shortcut is **one combination, registered with the system**; the application is
  told when it is pressed and of nothing else. It reads no other key — it is not a keyboard
  hook — and on macOS needs none of the permissions a hook does.
- While registered, the combination is **taken from every other application**. So a
  combination must have a modifier other than `Shift`: a bare key, or `Shift` and a key, would
  take typing away from the whole system, and is **refused**, with a line in the log (once).
- A combination another application holds **cannot be taken**: the system refuses it, the log
  says so, and the application keeps its other shortcuts.
- It is given back when withdrawn, and when the application ends.
- **Wayland** lets no application grab a combination for itself; its desktop portal asks the
  person instead. That is not done here: under Wayland there are no global shortcuts.

## Checked on the machine

Windows 11; the demo declares **Ctrl+Alt+Shift+D** to bring itself forward. The keys were sent
only after the log showed the system held the combination — so that they could reach no other
window:

| Step | Seen |
|---|---|
| The demo starts | `CONTROL+ALT+SHIFT+D registered` |
| Minimized, then the shortcut | the window back, no longer minimized |
| Closed — the tray keeps the demo — then the shortcut | the hidden window shown again |
| A second demo started beside the first | `could not be registered — another application may hold it`, the second running on |

## Left

- **Wayland**, through the desktop portal's `GlobalShortcuts`, which asks the person.
- **A key released** as well as pressed — what a push-to-talk wants.
- Not run on macOS or Linux; both type-check from Windows and build in CI.

With this, every item of #219 is either done (file drops 695, dialogs 698, notifications
701–705, tray 706, taskbar 708, shortcuts 709) or written down above as left.

## Tests

- `shortcuts.rs`: a combination has one spelling, whatever the order and case; `Primary` is
  `Cmd` on macOS and `Ctrl` elsewhere; what would take typing is refused — a bare key, `Shift`
  and a key — and so is what is not a combination (no key, two keys, an empty part); every
  canonical spelling parses as the library's hot key.

## Mutation testing

Six hand mutants, six killed: bare keys allowed, `Shift` counted as a modifier, `Primary`
always `Ctrl`, two keys allowed, an empty part allowed (it survived the first run — `Ctrl++K`
is refused anyway, as two keys — and `Ctrl+`, an empty key, is now in the test), the modifiers'
order not fixed.
