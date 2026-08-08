# Milestone 81 — Android InputConnection bridge (§6, stage 2)

## Analysis

Stage 1 (J80) opened the keyboard but stayed in `TYPE_NULL` mode: NativeActivity
provides no `InputConnection`, so IMEs only send Latin keys — no composition, no
swipe, no suggestions, no CJK — and some (SwiftKey) misbehave when faced with a
null connection. The established solution is a Java View offering a real
`InputConnection` wired to the engine; we follow exactly that step — **without
Gradle**.

## Architecture

1. **`FrusTextBridge.java`**: a focusable 1×1 View added on top of the native
   content (`addContentView`), whose `onCreateInputConnection` returns a real
   `BaseInputConnection`. Every IME operation — `commitText`,
   `setComposingText`, `finishComposingText`, `deleteSurroundingText`,
   `performEditorAction`, `sendKeyEvent` — is relayed to `native*` methods.
2. **Bundled dex**: compiled once (`scripts/build-input-dex.sh`, javac + d8) and
   versioned (`frus-shell/assets/frus_input.dex`, ~5.4 KB); loaded at runtime
   through `InMemoryDexClassLoader` + `RegisterNatives` (the `jni` crate). **No
   packaging change** — cargo-apk never needs javac.
3. **Queue + wake-up**: the natives (on the Java UI thread) push `ImeEvent`s into
   a shared queue and wake the winit loop (`AndroidAppWaker`); the shell drains
   it in `new_events` and applies to the focused field through `apply_key`
   (commit → `Text`, composition → replacing the current region, action/`\n` →
   `Enter`, deletions → `Backspace`/`Delete`).
4. **Switching**: `sync_soft_input` goes through the bridge when it is installed
   (`startInput`/`stopInput`: Java focus on the bridge view + IMM), otherwise it
   falls back to J80's `TYPE_NULL`. When the bridge is active, winit's keyboard
   editing path is **cut off** (otherwise every hardware key would arrive twice:
   the native queue and the bridge view).

## The bug flushed out along the way — positional identity, for real

The user's symptom: "the keyboard leaves and comes back in a loop". The diagnosis
(transition logs): the tree was alternating between **54 ↔ 53 widgets** —
keyboard open → short screen → the conditional *Tip* banner unmounts → all the
siblings' positional ids shift → the focused id resolves to the
`SegmentedControl` → the shell believes focus has left a field → closes the
keyboard → the screen grows back → the Tip remounts → the field is focused again
→ reopening… **This is exactly the class of bug predicted in §2** ("reordering or
conditioning loses state") — fixed by the canonical remedy: **keys**
(`keyed(...)`) on the siblings next to the conditional banner. Focus now survives
the Tip mounting and unmounting.

## Validated on the device (STK-L21, SwiftKey)

- `input bridge installed (real InputConnection)` at boot; the keyboard opens
  **explicitly** (`mShowExplicitlyRequested=true`) and is stable (a single
  transition in 8 s — the loop is dead).
- **Real typing on the touch keyboard** (SwiftKey keys) → `commitText` → the text
  appears in the field; the IME's **blue Enter** (`performEditorAction`) →
  submission, the task added, the field cleared. Double-checked: the user typed
  "This is" by hand during the session.

## Limits (a possible stage 3)

- Composition is materialised in the field **without styling** (no underline on
  the composing region) and `getTextBeforeCursor` returns an empty context (less
  relevant suggestions). A real stage 3: synchronising the complete editing state
  to the connection.
- `deleteSurroundingText` is applied around the caret without accounting for a
  non-contiguous composing region.
