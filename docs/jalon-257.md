# Jalon 257 — Android keyboard fix: reopening the keyboard when a field is tapped again

## Analysis

A bug observed **on device** (a Huawei STK-L21): you tap in a field → the soft keyboard comes up; you
press the **system back button** → the keyboard goes away; you tap in **the same field** again → the
keyboard **no longer comes up**.

The cause: the shell drives the keyboard through `sync_soft_input`, which only acts on a **change**
(`editing != self.soft_input_shown`). The faulty sequence:
1. A tap on the field → `editing = true`, `soft_input_shown` becomes `true`, the keyboard shown.
2. **System back** → Android closes the IME, but **the app is not notified**: `soft_input_shown` stays
   `true` and **the focus does not change** (the field stays focused).
3. Another tap on the field → the focus unchanged, `editing` still `true`, `soft_input_shown` still
   `true` → the diff sees **no change** → the keyboard is **never re-requested**.

## Technical decision

The native behaviour: **tapping in a field shows the keyboard** — unconditionally, not only on a
transition. We add `request_soft_input()`, which **re-requests** the IME for the focused field
(`start_input` through the InputConnection bridge, or a `show_soft_input(true)` fallback), called when
the user **taps in a text field** (creating a `Drag::TextSelect`). Independent of `sync_soft_input`'s
diff: it covers the "closed by system back with no notification" case.

## Implementation

- `frus-shell/src/app.rs`: the new `request_soft_input` method (a `#[cfg(target_os = "android")]` body);
  called from `pointer_down` right after arming `Drag::TextSelect` (a tap **inside** a field).

## Verification

- **Desktop**: compiles; shell 27 (the method is a no-op outside Android, no regression).
- **On device** (Huawei STK-L21): **confirmed** — tap → keyboard; system back → keyboard hidden;
  **tap again → the keyboard comes back**. A sequence previously broken, now correct.

## Notes

- The project memory said "no soft keyboard/IME": **out of date** — the IME integration
  (`android_ime.rs`: commit/composing/delete/key) and the keyboard **inset** handling
  (`WindowInsets::from_baseline`, `on_insets`, `reveal_caret`) exist. This milestone fixes a **specific**
  defect in that integration, not an absence.
- A lead: detecting the **external** closing of the IME (system back) to reset `soft_input_shown` to
  `false` — belt and braces, should other paths reopen a field without going through a tap.

## What's left (carried over from previous milestones)

- Coverage of the **same-column** reflow (source/target overlap → a net zero shift).
- **Vertical** inertia/spring for the slide (parity with the horizontal).
- Unifying `Card`/`Toast`'s shadow onto `theme.scheme.shadow`.
