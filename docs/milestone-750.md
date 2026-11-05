# Milestone 750 — Scrolling with the keyboard up keeps it up

## Objective

With the on-screen keyboard open in a text field, the person must be able to scroll the page or
the list without the keyboard closing. On the phone (Huawei STK-L21, Android 10), on the Web and
in the native Android build, a press anywhere outside the focused field took the focus away the
moment the finger came down: every scroll that started outside the field closed the keyboard.

## Root cause

`App::pointer_down` settled the focus on the press, for every pointer: the focus went to whatever
focusable the press landed on, or to nothing, before anyone could know whether the press would
be a tap or the start of a scroll. A finger's press is not yet a tap. The field's keyboard
followed the focus at the end of the frame (`sync_soft_input`), so it went down with the first
touch of every scroll.

On the Web there was a second path to the same result: winit focuses the canvas on every
`pointerdown`, which takes the browser's focus off the editing element of milestone 711, and a
phone's browser lets its keyboard go with it. The shell gave the element its focus back a frame
later, too late to be sure the keyboard stays.

## What was done

**A finger's press outside the focused widget decides nothing until it is a tap.** In
`pointer_down`, when a widget holds the focus and the press lands neither on it nor on a widget
that places a caret (another field, a selectable text), the shell works out where the focus would
go and, for a finger, only records it (`PendingFocus`, with where the finger came down). At the
release, the recorded move is made if the gesture was a tap: lifted within the touch slop of
where it landed, and not taken by a scroll, a pan, a swipe or any other drag. A press that became
a scroll or a fling leaves the focus, and the keyboard, where they were. A cancelled press or a
long press that took the gesture drops it.

A press on another field still moves the focus at once — it places the caret there, and the
keyboard is handed from one field to the other without closing. A mouse press is already the
click: on a desktop the decision is made when the button goes down, as before.

**Where a tap outside sends the focus is the field's to say.** `TapOutside` (new, exported):

- `TapOutside::Unfocus`, the default — the focus goes where the tap went: to the button or switch
  tapped, or nowhere, and the keyboard goes.
- `TapOutside::Keep` — the field keeps the focus and its keyboard through a tap on empty space or
  on a button; the button is still pressed.

Set per field with `TextField::tap_outside`, or for every field with
`TextFieldTheme::tap_outside`. Any widget may answer through `Widget::tap_outside(&Theme)`; the
walk resolves it under the theme in force and records it on the `Focusable`, which the shell
reads with `Ui::tap_outside(id)`. The existing `on_tap_outside` message is unchanged: still sent
on the press, before the focus moves.

**A scrollable may put the keyboard away when it is dragged.** `KeyboardDismissBehavior` (new,
exported): `Manual`, the default, and `OnDrag`, which lets the focus go as soon as a finger starts
scrolling the area — the moment the drag passes the slop and the area is claimed. Set on
`SingleChildScrollView`, `ListView` and `GridView` with `keyboard_dismiss_behavior`, or for a
subtree with `ScrollBehavior::with_keyboard_dismiss_behavior`; every registered `Scrollable`
carries the resolved answer (`Scrollable::keyboard_dismiss`). Transparent wrappers and
`Responsive` forward both new trait methods.

**The Web keeps its element focused through a press.** On a touch page the editing element
listens to its own `blur`: when the focus is leaving for the canvas — winit's `focus()` on a
press — it takes the focus straight back in the same event, before the browser acts on the
change. Whether the press ends the editing is the shell's to say; when it does, the shell removes
the element, which closes the keyboard as before.

## Found on the phone, and fixed

**The window kept the keyboard's room after the keyboard closed.** Tapping outside the field (or
tapping a button) closed the keyboard, but the application stayed laid out above where the
keyboard had been, an empty band at the bottom of the screen, until the next touch. winit drops
Android's content-rect change ("TODO: find a way to notify application of content rect change"),
so the shell only reads the new insets on a frame — and a field that has just lost the focus has
no caret left to blink, so no frame came. While the keyboard was opening the caret's blink hid
the same fault. The shell now keeps frames coming for a second after it asks the keyboard to open
or close (`keyboard_settle`), reading the insets as the keyboard slides. On the phone the
application takes its full height back within the screenshot's 1.5 s, with no further input.

## Checked on the phone

frus-demo built with `cargo apk build --release` (7 MB), installed over the version already on
the phone, driven with `adb` (taps, `input swipe`, `input text`, key events) and checked with
screenshots, `dumpsys input_method` and logcat.

1. **Typing**: a tap on "New task" opens the keyboard, the field comes up above it with the caret
   shown, `input text` types, Backspace deletes, a long press selects the word with both handles
   and the Cut / Copy / Paste bar.
2. **Scrolling with the keyboard up**: a swipe started on the Tip card scrolled the page and the
   keyboard stayed (`mInputShown=true`), the field still focused; so did a swipe started on the
   "Mark active" button, and a quick fling. A tap on empty space then closed the keyboard and
   unfocused the field. Tapping **Add** with the keyboard up added the task. Moving from "Full
   name" to "Email" in the sign-up wizard kept the keyboard up throughout (three samples during
   the switch). Back closes the keyboard first, the field keeping its focus; a second Back goes
   back a page.
3. **Leaving and coming back**: Home, five seconds, `am start` — the same frame, field text, focus
   and keyboard; the screenshots before and after are identical. The recents screen and back:
   identical. Power off, power on, swipe up: identical, below the status bar.
4. **Rotation** (`user_rotation` 1 then 0): landscape lays out at 2224 px wide with the navigation
   bar on the right kept clear; back in portrait with the keyboard up, the layout and the insets
   are right. The settings were put back (`accelerometer_rotation=1`, `user_rotation=0`).
5. **Navigation**: the bottom tabs switch, the drawer opens, a page is pushed and Back pops it;
   Back at the root sends the application to the background, without a crash.

The shadows of milestone 746 are not in this branch's base, and were not checked.

## How it was tested

`app::keyboard_stays_tests` drives the shell (`Driver`) on a simulated Android phone, a form in a
`SingleChildScrollView`: a field, a button, sixty rows.

- `a_scroll_outside_the_field_keeps_the_keyboard` — focus the field, drag 200 px from a row: the
  field is still focused while the finger moves and after it lifts, the simulated keyboard is
  still up, the page scrolled.
- `a_tap_outside_lets_the_field_go` — a finger down on a row changes nothing; its release
  unfocuses the field and the keyboard goes.
- `a_button_tapped_with_the_keyboard_up_is_pressed` — the button's message arrives; the default
  lets the focus go.
- `a_field_told_to_keep_the_focus_keeps_it` — `TapOutside::Keep`: a tap on a row and a tap on the
  button leave the field focused and the keyboard up; the button is pressed.
- `a_scroll_view_told_to_dismiss_on_drag_lets_the_field_go` — `OnDrag`: unfocused while the
  finger is still down, and the page scrolled.
- `a_mouse_press_outside_lets_the_field_go_at_once` — a mouse press unfocuses on the press.

With the deferral switched off (the press deciding, as before this change), the scroll and the
tap-timing tests fail; with only the drag dismissal switched off, the `OnDrag` test fails (it
first checks that a finger down leaves the focus, so that it cannot pass on the old press-time
unfocus). The existing focus
tests
(`focus_change_tests`, `simulated_keyboard_tests`) pass unchanged. The whole `frus-widgets` and
`frus-shell` suites pass.

## Not done

- **A swipe that starts on another field** focuses that field and drag-selects its text instead
  of scrolling — the press on a field still places the caret at once. Phones scroll there and
  focus only on a tap. Moving it to the release touches the word selection by a hold and the
  drag selection, and was left for its own step. A swipe that starts on the focused field
  itself selects too.
- **The Web path was not run on the phone.** The `blur` listener is compiled and linted for
  `wasm32`, but the keyboard staying up through a scroll in Chrome on Android was not observed:
  this milestone's phone session was the native build.
- **Found on the phone, not fixed:**
  - In landscape with the keyboard up, the scaffold keeps its bottom navigation bar above the
    keyboard, which leaves the page about 60 px: the focused field cannot be seen, and the
    floating button is drawn over the status bar. In portrait the bottom bar above the keyboard
    costs the page its height too; the leading toolkit leaves the bottom bar under the keyboard.
  - After Back pops a page, the drawer's menu button shows the keyboard focus ring, on a touch
    screen.
  - The system killed the demo in the background (Huawei's low-memory killer, 12 s after it left
    the screen, 130 MB): coming back from the recents started it afresh, the draft gone. Back at
    the root ends the activity, and the process with it, likewise. Restoring an application's
    state after the process is killed is not done on Android.
  - `KEYCODE_APP_SWITCH` twice goes to the previous application on this launcher, not back:
    a test artefact, noted for the next session.
