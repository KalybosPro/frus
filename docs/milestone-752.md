# Milestone 752 — The keyboard covers the bottom bar; focus rings follow the input; a swipe on a field scrolls

## Objective

Three faults found on the phone (Huawei STK-L21, Android 10) during milestone 750:

1. **With the on-screen keyboard up, `Scaffold` held its bottom navigation bar and persistent
   footer above the keyboard.** In landscape that left the page about 60 px, too little to show
   the focused field, and the floating button was pushed up over the status bar. In portrait the
   bar above the keyboard took its height from the page as well. On a phone, the keyboard covers
   the bottom bar, and the page ends at the top of the keyboard.
2. **After Back popped a page, the button the focus fell back to (the drawer's menu button) had
   the keyboard focus ring**, on a touch screen, where nobody had used a keyboard.
3. **A swipe that started on a text field focused it and drag-selected its text** instead of
   scrolling the form. On a phone a swipe across a form scrolls it, and a field is focused only
   by a tap.

## Root causes

1. `Scaffold` computed how far the bottom slot is held off the window's edge as
   `max(insets.bottom, view_insets.bottom)`, which is the safe area *or the keyboard*. This
   value placed the bottom bar and the footer, so they stood on top of the keyboard. The body
   was then shortened by the keyboard and also by the bars above it.
2. Two things turned the ring on. The shell treated every key as keyboard use, and that includes
   the system's Back (`KEYCODE_BACK`). Separately, a `Command::focus`, which is how a popped page
   hands the focus back, set `focus_visible = true` unconditionally.
3. `pointer_down` focused a field and placed its caret as soon as the finger came down, and then
   started a `Drag::TextSelect`. That happened before anyone could know whether the press would
   be a tap, a hold or a scroll.

## What was done

**The keyboard covers the widgets below the body** (`scaffold.rs`).

- The bottom slot is now held off the edge by the permanent intrusion only: the padding the
  keyboard leaves uncovered when the scaffold resizes, or `view_padding` when it declines to.
- When the scaffold resizes for the keyboard, the footer and the bottom bar go at the bottom of a
  block (`KeyboardFloor`) at least as tall as the keyboard. The keyboard covers them where they
  stand, and the body stops at whichever reaches higher, the keyboard or those widgets:
  `content bottom = max(bottom widgets, keyboard)`. The two are never added up.
- A body extended under the bars is shortened by the keyboard too.
- The floating button is placed from that same content bottom, so it stays in sight above the
  keyboard. It is never placed above the status bar: when the keyboard leaves too little room it
  stops below it.
- `resize_to_avoid_bottom_inset(false)` still lets the keyboard cover everything, unchanged.

**Focus rings follow the kind of input used last** (new `focushighlight.rs`, exported):

- `FocusHighlightMode` is `Touch` (no ring; where an application starts) or `Traditional` (the
  focused widget shows its ring).
- `FocusHighlightStrategy` is `Automatic` (the default), `AlwaysTouch` or `AlwaysTraditional`.
  An application chooses it with `Application::focus_highlight_strategy`.
- `Runtime::focus_highlight_mode` reads the current mode.
- The shell records each input through one place (`note_input`):
  - A touch or a click turns the rings off.
  - A key turns them on, and so does a focus move asked for by an assistive technology.
  - The system's **Back** is navigation, and changes nothing.
  - **Keys from the on-screen keyboard** typing into the field it was opened for change nothing.
    On Android, that is the soft input being shown over a focused widget that takes typing.
  - A focus moved by `Command::focus` (including the one a popped page hands back) no longer
    turns the rings on. It is shown or not as the mode already says.

**A finger on a field is not yet a tap** (`app.rs`). When a finger comes down on a widget that
places a caret (a text field or a selectable text), the shell records the press (`FieldPress`)
and moves neither the focus nor the caret.

- **Tap:** a release within the touch slop, with no gesture having taken the pointer, focuses
  the field and places the caret where the finger came down.
- **Hold:** the long press focuses the field and selects the word. If the scroll had taken the
  gesture, the field takes it back.
- **Drag:** the scroll the field sits in takes it, and nothing is placed.
- **Cancelled press:** nothing is placed.
- **Mouse:** a press still focuses at once, a drag still selects, and a double click still
  selects the word. Only a finger waits.

What used to happen on the press now lives in `settle_press_focus`, shared by the mouse path
(at once) and the touch path (at the tap or the hold).

## How it was tested

- `scaffold.rs` tests lay a shell out with an app bar, a filling body, a bottom bar and a 56 px
  floating button, and read the painted rectangles:
  - `the_keyboard_covers_the_bottom_bar_instead_of_lifting_it`, in portrait and landscape: the
    bar stays at the window's bottom, under the keyboard, and the body ends at the keyboard's
    top.
  - `without_a_keyboard_the_bar_holds_the_body_off`.
  - `a_keyboard_lower_than_the_bar_moves_nothing`.
  - `a_shell_that_declines_to_resize_is_covered_whole`.
  - `the_button_stays_below_the_status_bar`.
  - `the_keyboard_covers_the_footer_too`.
- `app::focus_highlight_tests` (the shell driven on a simulated Android phone):
  - `a_page_popped_with_back_shows_no_ring_under_touch`.
  - `a_key_shows_the_ring_and_a_touch_hides_it`.
  - `typing_on_the_on_screen_keyboard_leaves_the_rings_off`.
  - `a_focus_moved_by_the_application_after_a_tap_shows_no_ring`.
  - `the_strategy_can_pin_the_mode`.
- `app::field_drag_tests`:
  - `a_swipe_starting_on_another_field_scrolls`: no focus, no selection, and the page scrolled.
  - `a_swipe_starting_on_the_focused_field_scrolls`.
  - `a_tap_on_a_field_focuses_it_when_it_lifts`: nothing on the press, a caret and no selection
    at the release.
  - `a_hold_on_a_field_selects_the_word`.
  - `a_mouse_drag_on_a_field_selects`: the mouse path is unchanged.
- After merging `master`, the whole `frus-shell` library suite (446 tests) and the
  `frus-widgets` suites (2131 unit tests, plus its integration tests and doc tests) pass.
- Before that merge, all the `frus-test` suites passed, goldens included (run under WSL). No
  golden changed, and none was regenerated.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -D warnings` pass, and
  so does clippy for `frus-shell` on `aarch64-linux-android` (`--no-deps`) and for `frus-shell`
  and `frus-widgets` on `wasm32-unknown-unknown`.

## Checked on the phone

frus-demo was built with `cargo apk build --release` (7 MB, signed with the debug keystore) and
installed over the version already on the phone (0.2.1). It was driven with `adb` (taps,
`input swipe`, `input text`, held `motionevent`s, key events) and checked with screenshots and
`dumpsys input_method`.

1. **A swipe that starts on a field scrolls.** An `input swipe` from the unfocused "New task"
   field up 500 px scrolled the page to its end. The field was not focused, nothing was
   selected, and no keyboard opened (`mInputShown=false`).
2. **A tap focuses.** A tap on the field focused it and the keyboard opened
   (`mInputShown=true`). `input text` typed into it.
3. **A hold selects.** A finger held for a second on the word selected it, with both handles and
   the Cut / Copy / Paste bar.
4. **Portrait, keyboard up.** The bottom navigation bar is under the keyboard. The page runs
   down to the keyboard's top, the focused field is in sight, and the floating button sits just
   above the keyboard.
5. **Landscape, keyboard up** (`user_rotation` 1). The bottom bar is under the keyboard and the
   page runs down to the keyboard's top. The keyboard on this phone (SwiftKey) takes most of
   the landscape height, so about 80 px of page are left. The focused field and its selected
   word can be seen there. The floating button is below the status bar, but it now sits over the
   right end of the app bar. Back closed the keyboard: the page took its full height back, with
   the bottom bar at the bottom.
6. **No ring after a page pop.** The overflow menu, then "Settings →", pushed a page, and Back
   popped it. The drawer's menu button, where the focus falls back, has no ring.
7. **Background and foreground, keyboard up.** Home, five seconds, `am start`: the same page,
   field text, focus and keyboard (`mInputShown=true`).

After the session the version that was on the phone before (0.2.1) was installed back. The
rotation settings were put back as found (`accelerometer_rotation=1`, `user_rotation=0`), and
no `adb` forward or reverse mapping was made.

A test artefact: a "swipe" made of separate `input motionevent` calls came in too slowly. Each
call starts a process, so the first move arrived after the long-press delay. The demo took it as
a hold, as it should: the field was focused, and the bar offered Paste over the empty field.
`input swipe` sends the moves at once.

## Not done

- **The floating button in landscape with a tall keyboard** stops below the status bar, so it
  now covers the end of the app bar (the overflow button and "Settings →" on the demo). Phones
  leave it there or hide it; whether to hide the button while the keyboard leaves it no room was
  not decided here.
- **The page in landscape** keeps only what the keyboard leaves, about 80 px with this phone's
  keyboard. The app bar is not collapsed for the keyboard.
- **Selectable text and `SelectionArea` on a phone** were not exercised on the device. A finger
  on a selectable text now waits as a field does, and the driver tests cover only fields.
- **The Web** was not run on a phone. The touch path is the same shell code, linted for
  `wasm32`.
- The floating button's docked placement without a bottom bar now measures from the window's
  edge (0) rather than from the safe area. No test or golden covers that combination.
