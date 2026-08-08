# Milestone 80 — Android soft keyboard (opening the §6 input work)

## Analysis

An on-device observation (the J74+ session): the soft keyboard never opens, and
input does not seem to reach the fields. Investigating in the sources:

- **winit 0.30 on Android already maps characters**: each `KeyEvent` goes
  through the device's `KeyCharacterMap` (JNI, with dead-key composition) →
  `Key::Character` — so the shell's existing (desktop) editing path is **already
  wired** for Android key events. The observed failure most likely came from
  there being no focus (the field had never been tapped) — to be confirmed on the
  device.
- **The keyboard does not open** because nobody asks it to: NativeActivity has no
  `InputConnection`, so `InputMethodManager` has to be called — exposed by
  `android-activity` through `AndroidApp::show_soft_input`/`hide_soft_input`
  (the egui/game-activity approach).

## Implementation

`frus-shell`: `sync_soft_input()` called at the end of the frame (any focus
change already redraws) — the keyboard is **requested when focus is inside a text
field** (`cursor_at` → `Some`, the same criterion as the arrows), and closed
otherwise (blur, Escape, back, a modal closing…). Transitions are deduplicated
(`soft_input_shown`). Compiled as a no-op off Android.

## Known limits (the rest of the §6 work)

- `TYPE_NULL` mode: with no `InputConnection`, the IME sends **key events** —
  enough for Latin text (Gboard handles it), but with no composition
  (suggestions, swipe, voice, rich emoji, CJK). The next stage is a JNI
  `InputConnection` + composition events (the real §6 FFI).
- Keyboard avoidance (J74, `view_insets`) will be validated in the same pass.

## On-device validation (STK-L21, SwiftKey IME) — and two fixes

- Tapping the field → **the keyboard comes up** and the content **moves above
  it** (the first real proof of J74's keyboard avoidance). Blur → closed.
- Two real bugs flushed out by injected input (`adb input text`):
  1. **Key bursts**: keystrokes closer together than a frame were being applied
     to the **retained** tree (the field's value a frame behind) — so each
     keystroke overwrote the previous one ("Hello" → "o"). Fix: `apply_key`
     refreshes the tree (`view`) as soon as the edit message is dispatched;
     `build_dirty` stays raised for the next full pass.
  2. **Android's Enter** arrives as `Character("\n")` (KeyCharacterMap), not as
     `Named(Enter)` → so it was being inserted as text instead of submitting.
     Fix: `"\n"`/`"\r"` mapped onto `Key::Enter` (a repeat is not re-submitted).
- After the fixes: "World" arrives whole (including the capital through the Shift
  meta key), and Enter adds the task and clears the field. ✔
- Injection artefacts (not frus bugs): with the keyboard **open**, SwiftKey
  consumes some of the injected events (a null connection) — real user input goes
  through the IME itself; and `adb input text` stops at the first space (use
  `%s`).
