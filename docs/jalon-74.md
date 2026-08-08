# Jalon 74 — Window insets: `padding` / `viewInsets` split (keyboard avoidance)

The last big §6 item on the platform side: distinguishing **static** insets
(system bars, notch) from **dynamic** ones (the soft keyboard) — the condition
for keyboard avoidance on Android.

## `WindowInsets { padding, view_insets }` (frus-core)

- **`padding`**: areas permanently occupied by the system.
- **`view_insets`**: areas covered by transient UI — the conventional definition:
  `view_insets.bottom` measures the **total** occlusion **from the edge** (the bar
  included), which is what makes combining them with `max` correct. *(My first
  version measured only the keyboard's excess — the safe-area test caught it:
  `max(45, 300) = 300` instead of the real 345.)*
- **`safe()`**: the total area to avoid — the per-side max (the keyboard covers
  the bar, so they are not added).
- **`from_baseline(reference, current)`**: the split derived from a *keyboard-free*
  reference (the bottom excess signals the keyboard).

## The reference on the shell side (a heuristic, self-correcting)

Android only delivers a raw `content_rect`. The shell takes as its **keyboard-free
reference** the first measurement for the current physical size (a rotation
resets it), and **corrects itself downwards** if a barer state appears — covering
the keyboard-open-at-start-up case and hidden bars. Desktop: zero everywhere,
unchanged. *(A documented heuristic, pending real IME insets through FFI — the
"typed channels" item of §6.)*

## API & adoption

- `Application::on_insets(WindowInsets)` (the signature formalised).
- **Demo**: the root safe area becomes `insets.safe()` — when the keyboard opens,
  all the content (input fields included) **rises above it**: keyboard avoidance,
  at the app level.

## Validation

- **251 tests**, all green — the core test pins: keyboard closed (everything in
  padding), open (`view_insets.bottom` = the total occlusion, safe area = 345),
  hidden bars (no negative keyboard); the demo test pins the avoidance (the
  bottom safe area following the keyboard).
- A warning-free build; the demo did not panic. **To be validated on the Android
  device** (there is no keyboard under WSL): open a field → the content rises.

## Not covered (accepted)

- The **consume-then-zero** rule of a nested `SafeArea` widget: the demo handles
  insets at the root (there is no SafeArea widget yet) — that will come with the
  ambient context (`Env`) of §2.
- Real IME insets through FFI (typed Android channels, §6).

## What's next (remaining §6)

A regularised keyboard model (physical + logical + character) — the last §6 item
practicable without Android FFI.
